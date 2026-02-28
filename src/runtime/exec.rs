use super::eval::{eval_command_part, eval_expr};
use super::{Context, Result, Value, status_into_i32};
use crate::parse::{
    Command, CommandLine, CommandPart, InputRedirect, MergeRedirect,
    OutputMode, OutputRedirect, Pipe, Pipeline, Redirect, Statement,
};

use std::fmt::Display;
use std::io::Write;
use std::process::Stdio;
use std::thread::JoinHandle;
use std::{ffi::OsString, io::Error as IoError, path::PathBuf};

#[derive(Debug)]
pub enum Error {
    EmptyCommand,
    NotFoundCommand(String),
    InvalidCommandNameType,
    Spawn(IoError),
    Pipe(IoError),
    Redirect(RedirectError),
    InvalidEnvValueType,
    InvalidTempEnvValueType,
}
impl std::error::Error for Error {}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            EmptyCommand => write!(f, "空のコマンド名です"),
            NotFoundCommand(name) => {
                write!(f, "コマンド{name}が見つかりませんでした")
            }
            InvalidCommandNameType => {
                write!(f, "コマンド名に文字列以外が指定されました")
            }
            Spawn(e) => write!(f, "コマンドの実行に失敗しました : {e}"),
            Pipe(e) => write!(f, "パイプエラー : {e}"),
            Redirect(e) => write!(f, "リダイレクトエラー : {e}"),
            InvalidEnvValueType => {
                write!(f, "環境変数に代入する値が文字列ではありません")
            }
            InvalidTempEnvValueType => {
                write!(f, "一時変数に代入する値が文字列ではありません")
            }
        }
    }
}
#[derive(Debug)]
pub enum RedirectError {
    FailCloneFile(IoError),
    FailOpenFile(IoError),
    InvalidFileNameType,
    InvalidHereInputType,
}
impl std::error::Error for RedirectError {}
impl Display for RedirectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use RedirectError::*;
        match self {
            FailCloneFile(e) => {
                write!(f, "ファイルディスクリプタのコピーに失敗しました : {e}")
            }
            FailOpenFile(e) => write!(f, "ファイルが開けませんでした : {e}"),
            InvalidFileNameType => {
                write!(f, "ファイル名は文字列で指定してください")
            }
            InvalidHereInputType => {
                write!(f, "標準入力する値は文字列で指定してください")
            }
        }
    }
}

pub fn value_to_args(value: &Value) -> Vec<String> {
    use Value::*;
    match value {
        String(str) => vec![str.clone()],
        Int(a) => vec![a.to_string()],
        Float(a) => vec![a.to_string()],
        Bool(a) => vec![a.to_string()],
        Array(v) => v.iter().flat_map(value_to_args).collect(),
        Option(o) => o.as_ref().map(|v| value_to_args(v)).unwrap_or_default(),
        Unit => vec![],
    }
}

pub fn execute_command_line(
    command_line: &CommandLine,
    env: &mut Context,
) -> Result<()> {
    for statement in &command_line.statements {
        execute_statement(statement, env)?;
    }
    Ok(())
}

fn execute_statement(statement: &Statement, env: &mut Context) -> Result<()> {
    use Statement::*;
    match statement {
        ShellCommand(shell_command) => {
            let command = shell_command.kind.clone();
            let args = shell_command
                .args
                .iter()
                .map(|arg| eval_command_part(arg, env))
                .collect::<Result<Vec<_>>>()?;
            env.last_status = super::shell_command::run(&command, &args)?;
        }
        Pipeline(pipeline) => {
            // エラーは無視してエラー出力のみ行う
            if let Err(e) = execute_pipeline(pipeline, env) {
                eprintln!("TODO:コマンドの場所付きのエラーにする");
                eprintln!("{e}");
            }
        }
        EnvAssign(env_assign) => {
            let name = &env_assign.name;
            let value = eval_expr(&env_assign.value, env)?;
            match value {
                Value::String(value) => {
                    // シングルスレッドでのみ環境変数を書き換えているので安全
                    unsafe { std::env::set_var(name.as_ref(), value) };
                }
                Value::Option(Some(value))
                    if matches!(*value, Value::String(_)) =>
                {
                    let Value::String(value) = *value
                    else {
                        unreachable!()
                    };
                    // シングルスレッドでのみ環境変数を書き換えているので安全
                    unsafe { std::env::set_var(name.as_ref(), value) };
                }
                Value::Option(None) => {
                    // シングルスレッドでのみ環境変数を書き換えているので安全
                    unsafe { std::env::remove_var(name.as_ref()) };
                }
                _ => {
                    return Err(Error::InvalidEnvValueType.into());
                }
            }
        }
    }
    Ok(())
}

fn execute_pipeline(pipeline: &Pipeline, env: &mut Context) -> Result<()> {
    // ゾンビプロセス対策用の構造体
    struct DropKillChild(std::process::Child);
    impl DropKillChild {
        fn wait(&mut self) -> Result<i32> {
            Ok(status_into_i32(self.0.wait().map_err(Error::Spawn)?))
        }
    }
    impl Drop for DropKillChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
        }
    }

    // コマンドの整理
    let resolved_commands = resolve_pipeline(pipeline, env)?;

    // コマンドを実行
    let mut command_handles = vec![];
    let mut string_handles = vec![];
    for resolved_command in resolved_commands {
        // 整理されたコマンドを分解
        let ResolvedCommand {
            name: command,
            envs,
            args,
            stdin,
            stdout,
            stderr,
        } = resolved_command;

        // 引数の展開
        let args = args.iter().flat_map(value_to_args).collect::<Vec<_>>();

        // Stdioを取得
        let (stdin, handle) = stdin.into_stdio()?;
        let stdout = stdout.into_stdio();
        let stderr = stderr.into_stdio();
        if let Some(handle) = handle {
            string_handles.push(handle);
        }

        // 外部コマンドの実行
        let external_handle = std::process::Command::new(command)
            .envs(envs)
            .args(args)
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .map(DropKillChild)
            .map_err(Error::Spawn)?;
        command_handles.push(external_handle);
    }

    // 全実行を待つ
    for mut handle in command_handles {
        env.last_status = handle.wait()?;
    }

    Ok(())
}

#[derive(Debug)]
struct ResolvedCommand {
    name: PathBuf,
    envs: Vec<(String, String)>,
    args: Vec<Value>,
    stdin: StdioInputConfig,
    stdout: StdioOutputConfig,
    stderr: StdioOutputConfig,
}
impl ResolvedCommand {
    fn new(command: &Command, env: &mut Context) -> Result<Self> {
        // コマンド名の評価
        let name = eval_command_part(&command.name, env)?;
        let Value::String(name) = name
        else {
            return Err(Error::InvalidCommandNameType.into());
        };
        if name.is_empty() {
            return Err(Error::EmptyCommand.into());
        }
        let Some(name) = find_executable(&name)
        else {
            return Err(Error::NotFoundCommand(name).into());
        };

        // 一時環境変数の評価
        let envs: Vec<_> = command
            .temp_envs
            .iter()
            .map(|temp_env| {
                let (env_var, env_val) = temp_env.as_ref();
                let Value::String(env_val) = eval_expr(env_val, env)?
                else {
                    return Err(Error::InvalidTempEnvValueType.into());
                };
                Ok((env_var.as_ref().clone(), env_val))
            })
            .collect::<Result<Vec<_>>>()?;

        // 引数の評価
        let args: Vec<_> = command
            .args
            .iter()
            .map(|arg| eval_command_part(arg, env))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            name,
            envs,
            args,
            stdin: StdioInputConfig::default(),
            stdout: StdioOutputConfig::default(),
            stderr: StdioOutputConfig::default(),
        })
    }
}
#[derive(Default, Debug)]
enum StdioInputConfig {
    #[default]
    Inherit,
    String(String),
    File(std::fs::File),
    PipeReader(std::io::PipeReader),
}
impl StdioInputConfig {
    fn into_stdio(
        self,
    ) -> Result<(Stdio, Option<JoinHandle<std::io::Result<()>>>)> {
        use StdioInputConfig::*;
        Ok(match self {
            Inherit => (Stdio::inherit(), None),
            String(s) => {
                let (reader, mut writer) =
                    std::io::pipe().map_err(Error::Pipe)?;
                let handle =
                    std::thread::spawn(move || writer.write_all(s.as_bytes()));
                (reader.into(), Some(handle))
            }
            File(file) => (file.into(), None),
            PipeReader(pipe) => (pipe.into(), None),
        })
    }
}
#[derive(Default, Debug)]
enum StdioOutputConfig {
    #[default]
    Inherit,
    File(std::fs::File),
    PipeWriter(std::io::PipeWriter),
}
impl StdioOutputConfig {
    fn try_clone(&self) -> Result<StdioOutputConfig> {
        use StdioOutputConfig::*;
        Ok(match self {
            Inherit => Inherit,
            File(file) => {
                File(file.try_clone().map_err(RedirectError::FailCloneFile)?)
            }
            PipeWriter(writer) => {
                PipeWriter(writer.try_clone().map_err(Error::Pipe)?)
            }
        })
    }
    fn into_stdio(self) -> Stdio {
        use StdioOutputConfig::*;
        match self {
            Inherit => Stdio::inherit(),
            File(file) => file.into(),
            PipeWriter(pipe) => pipe.into(),
        }
    }
}

fn resolve_pipeline(
    pipeline: &Pipeline,
    env: &mut Context,
) -> Result<Vec<ResolvedCommand>> {
    let Pipeline { first, rest } = &pipeline;
    let mut commands_with_redirects = vec![];

    // 初期設定
    commands_with_redirects
        .push((ResolvedCommand::new(first, env)?, first.redirects.clone()));
    for (pipe, command) in rest {
        // パイプに対する左右のコマンドを取得
        let (left, _) = commands_with_redirects.last_mut().unwrap();
        let mut right = ResolvedCommand::new(command, env)?;

        // パイプの作成
        let (reader, writer) = std::io::pipe().map_err(Error::Pipe)?;
        let reader = StdioInputConfig::PipeReader(reader);
        let writer = StdioOutputConfig::PipeWriter(writer);

        // 種類ごとに設定
        if matches!(pipe.as_ref(), Pipe::StdoutStderr) {
            left.stderr = writer.try_clone()?;
        }
        left.stdout = writer;
        right.stdin = reader;

        commands_with_redirects.push((right, command.redirects.clone()));
    }

    // リダイレクトの反映
    let mut commands = vec![];
    for (mut command, redirects) in commands_with_redirects {
        apply_redirect(&mut command, redirects, env)?;
        commands.push(command);
    }

    Ok(commands)
}
fn apply_redirect(
    resolved_command: &mut ResolvedCommand,
    redirects: Vec<crate::parse::Spanned<Redirect>>,
    env: &mut Context,
) -> Result<()> {
    for redirect in redirects {
        match redirect.as_ref() {
            Redirect::Input(input_redirect) => {
                apply_input_redirect(resolved_command, input_redirect, env)?;
            }
            Redirect::Output((
                (output_redirect, output_mode),
                file_expression,
            )) => {
                apply_output_redirect(
                    resolved_command,
                    output_redirect,
                    output_mode,
                    file_expression,
                    env,
                )?;
            }
            Redirect::Merge(merge_redirect) => {
                apply_merge_redirect(resolved_command, merge_redirect)?;
            }
        }
    }
    Ok(())
}
fn apply_input_redirect(
    resolved_command: &mut ResolvedCommand,
    input_redirect: &InputRedirect,
    env: &mut Context,
) -> Result<()> {
    use InputRedirect::*;
    match input_redirect {
        // ファイルを読み込み用に開いて設定
        File(file) => {
            // ファイル名の評価
            let value = eval_command_part(file, env)?;
            let Value::String(path) = value
            else {
                return Err(RedirectError::InvalidFileNameType.into());
            };

            let file = std::fs::OpenOptions::new()
                .read(true)
                .open(path)
                .map_err(RedirectError::FailOpenFile)?;
            resolved_command.stdin = StdioInputConfig::File(file);
        }
        // 文字列をそのまま渡す
        HereDoc(s) => {
            resolved_command.stdin =
                StdioInputConfig::String(s.as_ref().clone());
        }
        // 文字列を評価してそのまま渡す
        HereString(value) => {
            let value = eval_command_part(value, env)?;
            let Value::String(s) = value
            else {
                return Err(RedirectError::InvalidHereInputType.into());
            };
            resolved_command.stdin = StdioInputConfig::String(s);
        }
    }

    Ok(())
}
fn apply_output_redirect(
    resolved_command: &mut ResolvedCommand,
    output_redirect: &OutputRedirect,
    output_mode: &OutputMode,
    file_expression: &CommandPart,
    env: &mut Context,
) -> Result<()> {
    use OutputMode::*;
    use OutputRedirect::*;

    // ファイル名の評価
    let value = eval_command_part(file_expression, env)?;
    let Value::String(path) = value
    else {
        return Err(RedirectError::InvalidFileNameType.into());
    };

    // ファイルをモードに応じて開く
    let mut file_opt = std::fs::OpenOptions::new();
    file_opt.create(true);
    file_opt.write(true);
    match output_mode {
        Append => file_opt.append(true),
        Truncate => file_opt.truncate(true),
    };
    let file = file_opt.open(path).map_err(RedirectError::FailOpenFile)?;
    let file = StdioOutputConfig::File(file);

    // 書き込み元を設定
    match output_redirect {
        Stdout => {
            resolved_command.stdout = file;
        }
        Stderr => {
            resolved_command.stderr = file;
        }
        Both => {
            resolved_command.stdout = file.try_clone()?;
            resolved_command.stderr = file;
        }
    }
    Ok(())
}
fn apply_merge_redirect(
    resolved_command: &mut ResolvedCommand,
    merge_redirect: &MergeRedirect,
) -> Result<()> {
    use MergeRedirect::*;
    match merge_redirect {
        StderrToStdout => {
            resolved_command.stderr = resolved_command.stdout.try_clone()?
        }
        StdoutToStderr => {
            resolved_command.stdout = resolved_command.stderr.try_clone()?
        }
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
        .map(|paths| {
            use std::sync::LazyLock;
            static BUILTIN_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
                crate::CURRENT_EXE
                    .parent()
                    .expect("自身のディレクトリの取得に失敗しました")
                    .join("builtin")
                    .to_path_buf()
            });
            // 最初にビルトインコマンドを探す
            [BUILTIN_DIR.clone()]
                .into_iter()
                .chain(std::env::split_paths(&paths))
                .collect()
        })
        .unwrap_or_default()
}
