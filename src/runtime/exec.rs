use super::eval::{Error as EvalError, eval_command_part, eval_expr};
use super::shell_command::Error as ShellCommandError;
use super::{Context, SpannedError, Value, status_into_i32};
use crate::parse::{
    Command, CommandLine, InputRedirect, MergeRedirect, OutputMode,
    OutputRedirect, Pipe, Pipeline, Redirect, Span, Spanned, Statement,
};
use crate::runtime::WithSpan;

use std::fmt::Display;
use std::io::Write;
use std::process::Stdio;
use std::thread::JoinHandle;
use std::{ffi::OsString, io::Error as IoError, path::PathBuf};

type Result<T> = ::std::result::Result<T, Error>;
type SpannedResult<T> = ::std::result::Result<T, SpannedError<Error>>;
#[derive(Debug)]
pub enum Error {
    Eval(EvalError),
    ShellCommand(ShellCommandError),
    EmptyCommand,
    NotFoundCommand(String),
    InvalidCommandNameType,
    Spawn(IoError),
    Pipe(IoError),
    Redirect(RedirectError),
    InvalidEnvValueType,
    InvalidTempEnvValueType,
}
impl Error {
    pub fn is_exit(&self) -> Option<i32> {
        match self {
            Error::ShellCommand(ShellCommandError::Exit(code)) => Some(*code),
            _ => None,
        }
    }
}
impl std::error::Error for Error {}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            Eval(e) => e.fmt(f),
            ShellCommand(e) => e.fmt(f),
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
impl From<EvalError> for Error {
    fn from(value: EvalError) -> Self {
        Error::Eval(value)
    }
}
impl From<ShellCommandError> for Error {
    fn from(value: ShellCommandError) -> Self {
        Error::ShellCommand(value)
    }
}
impl From<RedirectError> for Error {
    fn from(value: RedirectError) -> Self {
        Error::Redirect(value)
    }
}
#[derive(Debug)]
pub enum RedirectError {
    FailOpenFile(IoError),
    InvalidFileNameType,
    InvalidHereInputType,
}
impl std::error::Error for RedirectError {}
impl Display for RedirectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use RedirectError::*;
        match self {
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

pub fn execute_command_line(command_line: &CommandLine, env: &mut Context) {
    for statement in &command_line.statements {
        // Exit以外のエラーは無視してエラー出力のみ行う
        match execute_statement(statement, env) {
            Err(e) => {
                if let Some(code) = e.kind.is_exit() {
                    std::process::exit(code);
                }
                else {
                    let display = e.display(&env.current_input);
                    eprintln!("{display}");
                }
            }
            Ok(_) => {}
        }
    }
}

fn execute_statement(
    Spanned {
        span,
        inner: statement,
    }: &Spanned<Statement>,
    env: &mut Context,
) -> SpannedResult<()> {
    use Statement::*;
    match statement {
        ShellCommand(shell_command) => {
            let command = shell_command.kind;
            let args = shell_command
                .args
                .iter()
                .map(|arg| eval_command_part(arg, env))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            env.last_status =
                super::shell_command::run(&command, &args).with_span(span)?;
        }
        Pipeline(pipeline) => execute_pipeline(pipeline, env)?,
        EnvAssign(env_assign) => {
            let name = &env_assign.name;
            let value_span = env_assign.value.span.clone();
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
                    return Err(Error::InvalidEnvValueType)
                        .with_span(&value_span);
                }
            }
        }
    }
    Ok(())
}

fn execute_pipeline(
    pipeline: &Pipeline,
    env: &mut Context,
) -> SpannedResult<()> {
    // ゾンビプロセス対策用の構造体
    struct DropKillChild(std::process::Child, Span);
    impl DropKillChild {
        fn wait(&mut self) -> SpannedResult<i32> {
            Ok(status_into_i32(
                self.0.wait().map_err(Error::Spawn).with_span(&self.1)?,
            ))
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
        let Spanned {
            span: command_span,
            inner:
                ResolvedCommand {
                    name: command,
                    envs,
                    args,
                    stdin,
                    stdout,
                    stderr,
                },
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
            .map(|child| DropKillChild(child, command_span.clone()))
            .map_err(Error::Spawn)
            .with_span(&command_span)?;
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
    fn new(command: &Command, env: &mut Context) -> SpannedResult<Self> {
        // コマンド名の評価
        let name_span = command.name.span.clone();
        let name = eval_command_part(&command.name, env)?;
        let Value::String(name) = name
        else {
            return Err(Error::InvalidCommandNameType).with_span(&name_span);
        };
        if name.is_empty() {
            return Err(Error::EmptyCommand).with_span(&name_span);
        }
        let Some(name) = find_executable(&name)
        else {
            return Err(Error::NotFoundCommand(name)).with_span(&name_span);
        };

        // 一時環境変数の評価
        let envs: Vec<_> = command
            .temp_envs
            .iter()
            .map(|temp_env| {
                let (env_var, env_val) = temp_env.as_ref();
                let span = env_val.span.clone();
                let Value::String(env_val) = eval_expr(env_val, env)?
                else {
                    return Err(Error::InvalidTempEnvValueType)
                        .with_span(&span);
                };
                Ok((env_var.as_ref().clone(), env_val))
            })
            .collect::<SpannedResult<Vec<_>>>()?;

        // 引数の評価
        let args = command
            .args
            .iter()
            .map(|arg| eval_command_part(arg, env))
            .collect::<std::result::Result<Vec<_>, _>>()?;

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
    String(String, Span),
    File(std::fs::File),
    PipeReader(std::io::PipeReader),
}
impl StdioInputConfig {
    fn into_stdio(
        self,
    ) -> SpannedResult<(Stdio, Option<JoinHandle<std::io::Result<()>>>)> {
        use StdioInputConfig::*;
        Ok(match self {
            Inherit => (Stdio::inherit(), None),
            String(s, span) => {
                let (reader, mut writer) =
                    std::io::pipe().map_err(Error::Pipe).with_span(&span)?;
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
                File(file.try_clone().map_err(RedirectError::FailOpenFile)?)
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
) -> SpannedResult<Vec<Spanned<ResolvedCommand>>> {
    let Pipeline {
        first:
            Spanned {
                span: first_span,
                inner: first,
            },
        rest,
    } = &pipeline;
    let mut commands_with_redirects = vec![];

    // 初期設定
    commands_with_redirects.push((
        Spanned::new(first_span.clone(), ResolvedCommand::new(first, env)?),
        first.redirects.clone(),
    ));
    for (
        pipe,
        Spanned {
            span,
            inner: command,
        },
    ) in rest
    {
        // パイプに対する左右のコマンドを取得
        let (left, _) = commands_with_redirects.last_mut().unwrap();
        let mut right = ResolvedCommand::new(command, env)?;

        // パイプの作成
        let pipe_span = pipe.span.clone();
        let (reader, writer) =
            std::io::pipe().map_err(Error::Pipe).with_span(&pipe_span)?;
        let reader = StdioInputConfig::PipeReader(reader);
        let writer = StdioOutputConfig::PipeWriter(writer);

        // 種類ごとに設定
        if matches!(pipe.as_ref(), Pipe::StdoutStderr) {
            left.inner.stderr = writer.try_clone().with_span(&pipe_span)?;
        }
        left.inner.stdout = writer;
        right.stdin = reader;

        commands_with_redirects.push((
            Spanned::new(span.clone(), right),
            command.redirects.clone(),
        ));
    }

    // リダイレクトの反映
    let mut commands = vec![];
    for (
        Spanned {
            span,
            inner: mut command,
        },
        redirects,
    ) in commands_with_redirects
    {
        apply_redirect(&mut command, redirects, env)?;
        commands.push(Spanned::new(span, command));
    }

    Ok(commands)
}
fn apply_redirect(
    resolved_command: &mut ResolvedCommand,
    redirects: Vec<Spanned<Redirect>>,
    env: &mut Context,
) -> SpannedResult<()> {
    for Spanned {
        span: redirect_span,
        inner: redirect,
    } in redirects
    {
        match redirect {
            Redirect::Input(input_redirect) => {
                apply_input_redirect(
                    resolved_command,
                    input_redirect,
                    &redirect_span,
                    env,
                )?;
            }
            Redirect::Output((
                (output_redirect, output_mode),
                file_expression,
            )) => {
                // ファイル名の評価
                let file_path_span = file_expression.span.clone();
                let value = eval_command_part(&file_expression, env)?;
                let Value::String(file_path) = value
                else {
                    return Err(RedirectError::InvalidFileNameType.into())
                        .with_span(&file_path_span);
                };

                apply_output_redirect(
                    resolved_command,
                    output_redirect,
                    output_mode,
                    &file_path,
                )
                .with_span(&file_path_span)?;
            }
            Redirect::Merge(Spanned {
                span,
                inner: merge_redirect,
            }) => {
                apply_merge_redirect(resolved_command, merge_redirect)
                    .with_span(&span)?;
            }
        }
    }
    Ok(())
}
fn apply_input_redirect(
    resolved_command: &mut ResolvedCommand,
    input_redirect: InputRedirect,
    redirect_span: &Span,
    env: &mut Context,
) -> SpannedResult<()> {
    use InputRedirect::*;
    match input_redirect {
        // ファイルを読み込み用に開いて設定
        File(file) => {
            // ファイル名の評価
            let file_span = file.span.clone();
            let value = eval_command_part(&file, env)?;
            let Value::String(path) = value
            else {
                return Err(RedirectError::InvalidFileNameType.into())
                    .with_span(&file_span);
            };

            let file = std::fs::OpenOptions::new()
                .read(true)
                .open(path)
                .map_err(RedirectError::FailOpenFile)
                .with_span(&file_span)?;
            resolved_command.stdin = StdioInputConfig::File(file);
        }
        // 文字列をそのまま渡す
        HereDoc(s) => {
            resolved_command.stdin = StdioInputConfig::String(
                s.as_ref().clone(),
                redirect_span.clone(),
            );
        }
        // 文字列を評価してそのまま渡す
        HereString(value) => {
            let value_span = value.span.clone();
            let value = eval_command_part(&value, env)?;
            let Value::String(s) = value
            else {
                return Err(RedirectError::InvalidHereInputType.into())
                    .with_span(&value_span);
            };
            resolved_command.stdin =
                StdioInputConfig::String(s, redirect_span.clone());
        }
    }

    Ok(())
}
fn apply_output_redirect(
    resolved_command: &mut ResolvedCommand,
    output_redirect: OutputRedirect,
    output_mode: OutputMode,
    file_path: &str,
) -> Result<()> {
    use OutputMode::*;
    use OutputRedirect::*;

    // ファイルをモードに応じて開く
    let mut file_opt = std::fs::OpenOptions::new();
    file_opt.create(true);
    file_opt.write(true);
    match output_mode {
        Append => file_opt.append(true),
        Truncate => file_opt.truncate(true),
    };
    let file = file_opt
        .open(file_path)
        .map_err(RedirectError::FailOpenFile)?;
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
    merge_redirect: MergeRedirect,
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
