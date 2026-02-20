#![allow(unused)]
use crate::eval::{Context, EvalError, ExecuteError, eval_command_part};
use crate::parse::{ShellCommand, Spanned};
use crate::value::Value;
use std::{ffi::OsString, fmt::Display, path::PathBuf};

type Result<T> = ::std::result::Result<T, ExecuteError>;

pub enum Output {
    Inherit,
    Capture(Vec<u8>),
}
impl std::io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        use Output::*;
        match self {
            Inherit => std::io::stdout().write(buf),
            Capture(vec) => vec.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        use Output::*;
        match self {
            Inherit => std::io::stdout().flush(),
            Capture(_) => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Shell {
    context: Context,
}
impl Shell {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn execute(
        &mut self,
        shell_command: &Spanned<ShellCommand>,
    ) -> Result<()> {
        execute_shell_command(
            shell_command,
            &mut Output::Inherit,
            &mut self.context,
        )
    }
}

pub fn execute_shell_command(
    shell_command: &Spanned<ShellCommand>,
    output: &mut Output,
    env: &mut Context,
) -> Result<()> {
    use crate::builtin::Error as BuiltinError;
    for (command, _pipe) in &shell_command.inner.commands {
        // 評価
        let name = eval_command_part(&command.inner.name, env)?;
        let args: Vec<_> = command
            .inner
            .args
            .iter()
            .map(|arg| eval_command_part(arg, env))
            .collect::<Result<Vec<_>>>()?;

        // コマンド名の展開
        // コマンド名は必ずStringである必要がある
        let Value::String(name) = name
        else {
            return Err(ExecuteError::InvalidCommandType);
        };

        // 空文字列なら何もしない
        if name.is_empty() {
            continue;
        }

        // ビルトインの実行を試す
        match crate::builtin::run(&name, &args) {
            // 実際のエラー処理はパイプとかを考える
            Ok(_) => {
                continue;
            }
            Err(BuiltinError::Exit(code)) => {
                return Err(ExecuteError::Exit(code));
            }
            Err(BuiltinError::CommandNotFound) => {}
            Err(e) => return Err(ExecuteError::CommandError(e.to_string())),
        }

        // 引数の展開
        let args = args.iter().flat_map(Value::to_args).collect::<Vec<_>>();
        // 外部コマンドの実行を試す
        let Some(name) = crate::exec::find_executable(&name)
        else {
            return Err(ExecuteError::CommandError(
                "コマンドが見つかりませんでした".to_string(),
            ));
        };
        let mut command = std::process::Command::new(name);
        command.args(args);
        // コマンドの出力を指定されたところへ
        let status = match output {
            Output::Inherit => command.status(),
            Output::Capture(vec) => {
                let output =
                    command.stderr(std::process::Stdio::inherit()).output();
                match output {
                    Ok(output) => {
                        vec.extend_from_slice(&output.stdout);
                        Ok(output.status)
                    }
                    Err(e) => Err(e),
                }
            }
        };
        env.last_status = match status {
            // とりあえず基本的に来ないNoneは1へ変換
            #[cfg(unix)]
            Ok(status) => status.code().unwrap_or_else(|| {
                use std::os::unix::process::ExitStatusExt;
                status.signal().map(|sig| 128 + sig).unwrap_or(1)
            }),
            #[cfg(not(unix))]
            Ok(status) => status.code().unwrap_or(1),
            Err(e) => return Err(ExecuteError::CommandError(e.to_string())),
        };
    }
    Ok(())
}

/// 実行可能ファイルのフルパスを探索
#[cfg(windows)]
fn find_executable(name: &str) -> Option<PathBuf> {
    let name = PathBuf::from(name);

    // 探索する拡張子を取得
    let extensions = name
        .extension()
        .map(|ext| vec![ext.to_os_string()])
        .unwrap_or_else(get_pathext);
    // 探索するパスを取得
    let search_dirs = name
        .parent()
        .and_then(|parent| {
            if parent.as_os_str().is_empty() {
                None
            }
            else {
                Some(vec![parent.to_owned()])
            }
        })
        .unwrap_or_else(get_path);

    let file_name = name.file_stem()?;
    for dir in search_dirs {
        for ext in &extensions {
            let candidate = dir.join(file_name).with_extension(ext);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}
/// 実行可能ファイルのフルパスを探索
#[cfg(not(windows))]
fn find_executable(name: &str) -> Option<PathBuf> {
    let name = PathBuf::from(name);
    // 探索するパスを取得
    let search_dirs = name
        .parent()
        .and_then(|parent| {
            if parent.as_os_str().is_empty() {
                None
            }
            else {
                Some(vec![parent.to_owned()])
            }
        })
        .unwrap_or_else(get_path);

    let file_name = name.file_name()?;
    for dir in search_dirs {
        let candidate = dir.join(file_name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(windows)]
fn get_pathext() -> Vec<OsString> {
    // var_osを使用するとより正確
    std::env::var("PATHEXT")
        .unwrap_or(".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .map(|s| OsString::from(s.trim_start_matches('.')))
        .collect()
}
fn get_path() -> Vec<PathBuf> {
    // var_osを使用するとより正確
    std::env::var("PATH")
        .map(|paths| std::env::split_paths(&paths).collect())
        .unwrap_or_default()
}
