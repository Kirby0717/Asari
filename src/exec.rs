#![allow(unused)]
use crate::eval::{Context, Error as EvalError, eval_command_part};
use crate::parse::ShellCommand;
use crate::value::Value;
use std::{ffi::OsString, fmt::Display, path::PathBuf};

#[derive(Clone, Debug)]
pub enum Error {
    Exit(i32),
    EvalError(EvalError),
    TypeError,
    CommandError(String),
}
impl From<EvalError> for Error {
    fn from(value: EvalError) -> Self {
        Error::EvalError(value)
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
type Result<T> = ::std::result::Result<T, Error>;

#[derive(Clone, Debug, Default)]
pub struct Shell {
    context: Context,
}
impl Shell {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn execute(&mut self, cmd: &ShellCommand) -> Result<()> {
        use crate::builtin::Error as BuiltinError;
        for (command, _pipe) in &cmd.commands {
            // 評価
            let name = eval_command_part(&command.name, &self.context)?;
            let args: Vec<_> = command
                .args
                .iter()
                .map(|arg| {
                    eval_command_part(arg, &self.context)
                        .map_err(Error::EvalError)
                })
                .collect::<Result<Vec<_>>>()?;

            // コマンド名の展開
            // コマンド名は必ずStringである必要がある
            let Value::String(name) = name
            else {
                return Err(Error::TypeError);
            };

            // ビルトインの実行を試す
            match crate::builtin::run(&name, &args) {
                // 実際のエラー処理はパイプとかを考える
                Ok(_) => {
                    continue;
                }
                Err(BuiltinError::Exit(code)) => return Err(Error::Exit(code)),
                Err(BuiltinError::CommandNotFound) => {}
                Err(e) => return Err(Error::CommandError(e.to_string())),
            }

            // 引数の展開
            let args = args.iter().flat_map(Value::to_args).collect::<Vec<_>>();
            // 外部コマンドの実行を試す
            let Some(name) = find_executable(&name)
            else {
                return Err(Error::CommandError(
                    "コマンドが見つかりませんでした".to_string(),
                ));
            };
            self.context.last_status =
                match std::process::Command::new(name).args(args).status() {
                    // とりあえず基本的に来ないNoneは1へ変換
                    #[cfg(unix)]
                    Ok(status) => status.code().unwrap_or_else(|| {
                        use std::os::unix::process::ExitStatusExt;
                        status.signal().map(|sig| 128 + sig).unwrap_or(1)
                    }),
                    #[cfg(not(unix))]
                    Ok(status) => status.code().unwrap_or(1),
                    Err(e) => return Err(Error::CommandError(e.to_string())),
                };
        }
        Ok(())
    }
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
