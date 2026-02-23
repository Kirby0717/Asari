use crate::builtin::{Error as BuiltinError, find_command};
use crate::eval::{Context, Error as EvalError, eval_command_part, eval_expr};
use crate::parse::{
    Command, CommandPart, InputRedirect, MergeRedirect, OutputMode,
    OutputRedirect, Pipe, Pipeline, Redirect, ShellCommand, Spanned, Statement,
};
use crate::value::Value;

use std::io::{Read, Write};
use std::process::Stdio;
use std::thread::JoinHandle;
use std::{ffi::OsString, io::Error as IoError, path::PathBuf};

#[derive(Debug)]
pub enum Error {
    Exit(i32),
    EvalError(EvalError),
    CommandError(IoError),
    BuiltinCommandError(BuiltinError),
    PipeError(IoError),
    RedirectError(IoError),
    EmptyCommand,
    NotFoundCommand(String),
    InvalidCommandType,
    InvalidInputRedirectType,
    InvalidRedirectFileType,
    FailOpenRedirectFile(IoError),
    CommandOutIsNotUtf8,
    EnvAssignNotStringOrNone,
    TempEnvAssignNotString,
}
impl From<EvalError> for Error {
    #[inline(always)]
    fn from(value: EvalError) -> Self {
        Error::EvalError(value)
    }
}
impl From<BuiltinError> for Error {
    #[inline(always)]
    fn from(value: BuiltinError) -> Self {
        match value {
            BuiltinError::Exit(code) => Error::Exit(code),
            e => Error::BuiltinCommandError(e),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
type Result<T> = ::std::result::Result<T, Error>;

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
    for statement in &shell_command.inner.statements {
        execute_statement(statement, output, env)?;
    }
    Ok(())
}
pub fn execute_statement(
    statement: &Spanned<Statement>,
    output: &mut Output,
    env: &mut Context,
) -> Result<()> {
    use Statement::*;
    match &statement.inner {
        Pipeline(pipeline) => {
            // Exitエラーは伝播させる
            // それ以外はエラー表示で次に進む
            if let Err(e) = execute_pipeline(pipeline, output, env) {
                if matches!(e, Error::Exit(_)) {
                    return Err(e);
                }
                eprintln!("{e:?}");
            }
        }
        EnvAssign(env_assign) => {
            let name = &env_assign.inner.name.inner;
            let value = eval_expr(&env_assign.inner.value, env)?;
            match value {
                Value::String(value) => {
                    // シングルスレッドでのみ環境変数を書き換えているので安全
                    unsafe { std::env::set_var(name, value) };
                }
                Value::Option(None) => {
                    // シングルスレッドでのみ環境変数を書き換えているので安全
                    unsafe { std::env::remove_var(name) };
                }
                _ => {
                    return Err(Error::EnvAssignNotStringOrNone);
                }
            }
        }
    }
    Ok(())
}

enum CommandHandle {
    External(std::process::Child),
    Builtin(JoinHandle<crate::builtin::Result<i32>>),
}
impl CommandHandle {
    fn wait(self) -> Result<i32> {
        match self {
            CommandHandle::External(mut child) => {
                let status = child.wait().map_err(Error::CommandError)?;
                Ok(status.code().unwrap_or(1))
            }
            CommandHandle::Builtin(handle) => {
                // ビルトインコマンドのパニックは伝播させる
                Ok(handle.join().unwrap()?)
            }
        }
    }
}

pub fn execute_pipeline(
    pipeline: &Spanned<Pipeline>,
    output: &mut Output,
    env: &mut Context,
) -> Result<()> {
    // outputがCaptureならパイプを渡す
    let (output_reader, output_writer) = match output {
        Output::Inherit => (None, None),
        Output::Capture(_) => {
            let (reader, writer) = std::io::pipe().map_err(Error::PipeError)?;
            (Some(reader), Some(writer))
        }
    };

    // コマンドの整理
    let resolved_commands = resolve_pipeline(pipeline, output_writer, env)?;

    // コマンドを実行
    let mut command_handles = vec![];
    let mut string_handles = vec![];
    for resolved_command in resolved_commands {
        let ResolvedCommand {
            command,
            envs,
            args,
            stdin,
            stdout,
            stderr,
        } = resolved_command;

        match command {
            ExecutableCommand::Builtin(command) => {
                let builtin_handle = std::thread::spawn(move || {
                    let stdin = stdin.into_read();
                    let stdout = stdout.into_write_stdout();
                    let stderr = stderr.into_write_stderr();
                    crate::builtin::run(command, &args, stdin, stdout, stderr)
                });
                command_handles.push(CommandHandle::Builtin(builtin_handle));
            }
            ExecutableCommand::External(command) => {
                let (stdin, handle) = stdin.into_stdio()?;
                let stdout = stdout.into_stdio();
                let stderr = stderr.into_stdio();
                if let Some(handle) = handle {
                    string_handles.push(handle);
                }

                // 引数の展開
                let args =
                    args.iter().flat_map(Value::to_args).collect::<Vec<_>>();

                let external_handle = std::process::Command::new(command)
                    .envs(envs)
                    .args(args)
                    .stdin(stdin)
                    .stdout(stdout)
                    .stderr(stderr)
                    .spawn()
                    .map_err(Error::CommandError)?;
                command_handles.push(CommandHandle::External(external_handle));
            }
        }
    }

    // アウトプットの受け取りスレッド
    let output_handle = output_reader.map(|mut reader| {
        std::thread::spawn(move || {
            let mut v = vec![];
            let _ = reader.read_to_end(&mut v);
            v
        })
    });

    // 全実行を待つ
    for handle in command_handles {
        env.last_status = handle.wait()?;
    }

    // アウトプットを追記
    if let Some(output_handle) = output_handle
        && let Output::Capture(v) = output
    {
        let output = output_handle.join().unwrap();
        v.extend_from_slice(&output);
    }

    Ok(())
}

#[derive(Debug)]
enum ExecutableCommand {
    Builtin(crate::builtin::BuiltinCommand),
    External(PathBuf),
}
#[derive(Debug)]
struct ResolvedCommand {
    command: ExecutableCommand,
    envs: Vec<(String, String)>,
    args: Vec<Value>,
    stdin: StdioInputConfig,
    stdout: StdioOutputConfig,
    stderr: StdioOutputConfig,
}
impl ResolvedCommand {
    fn new(command: &Spanned<Command>, env: &mut Context) -> Result<Self> {
        // コマンド名の評価
        let name = eval_command_part(&command.inner.name, env)?;
        let Value::String(name) = name
        else {
            return Err(Error::InvalidCommandType);
        };
        if name.is_empty() {
            return Err(Error::EmptyCommand);
        }

        // 一時環境変数の評価
        let envs: Vec<_> = command
            .inner
            .temp_env
            .iter()
            .map(|temp_env| {
                let (env_var, env_val) = &temp_env.inner;
                let Value::String(env_val) = eval_expr(env_val, env)?
                else {
                    return Err(Error::TempEnvAssignNotString);
                };
                Ok((env_var.inner.clone(), env_val))
            })
            .collect::<Result<Vec<_>>>()?;

        // 引数の評価
        let args: Vec<_> = command
            .inner
            .args
            .iter()
            .map(|arg| eval_command_part(arg, env))
            .collect::<Result<Vec<_>>>()?;

        // ビルトインコマンドの確認
        let command = if let Some(command) = find_command(&name) {
            ExecutableCommand::Builtin(command)
        }
        // 外部コマンドの確認
        else {
            ExecutableCommand::External(
                find_executable(&name)
                    .ok_or(Error::NotFoundCommand(name.to_string()))?,
            )
        };

        Ok(Self {
            command,
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
    fn into_read(self) -> Box<dyn std::io::Read> {
        use StdioInputConfig::*;
        match self {
            Inherit => Box::new(std::io::stdin()),
            String(s) => Box::new(std::io::Cursor::new(s)),
            File(file) => Box::new(file),
            PipeReader(pipe) => Box::new(pipe),
        }
    }
    fn into_stdio(
        self,
    ) -> Result<(Stdio, Option<JoinHandle<std::io::Result<()>>>)> {
        use StdioInputConfig::*;
        Ok(match self {
            Inherit => (Stdio::inherit(), None),
            String(s) => {
                let (reader, mut writer) =
                    std::io::pipe().map_err(Error::PipeError)?;
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
            File(file) => File(file.try_clone().map_err(Error::RedirectError)?),
            PipeWriter(writer) => {
                PipeWriter(writer.try_clone().map_err(Error::PipeError)?)
            }
        })
    }
    fn into_write_stdout(self) -> Box<dyn std::io::Write> {
        use StdioOutputConfig::*;
        match self {
            Inherit => Box::new(std::io::stdout()),
            File(file) => Box::new(file),
            PipeWriter(pipe) => Box::new(pipe),
        }
    }
    fn into_write_stderr(self) -> Box<dyn std::io::Write> {
        use StdioOutputConfig::*;
        match self {
            Inherit => Box::new(std::io::stderr()),
            File(file) => Box::new(file),
            PipeWriter(pipe) => Box::new(pipe),
        }
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
    pipeline: &Spanned<Pipeline>,
    capture: Option<std::io::PipeWriter>,
    env: &mut Context,
) -> Result<Vec<ResolvedCommand>> {
    let Pipeline { first, rest } = &pipeline.inner;
    let mut commands_with_redirects = vec![];

    // 初期設定
    commands_with_redirects.push((
        ResolvedCommand::new(first, env)?,
        first.inner.redirects.clone(),
    ));
    for (pipe, command) in rest {
        // パイプに対する左右のコマンドを取得
        let (left, _) = commands_with_redirects.last_mut().unwrap();
        let mut right = ResolvedCommand::new(command, env)?;

        // パイプの作成
        let (reader, writer) = std::io::pipe().map_err(Error::PipeError)?;
        let reader = StdioInputConfig::PipeReader(reader);
        let writer = StdioOutputConfig::PipeWriter(writer);

        // 種類ごとに設定
        if matches!(pipe.inner, Pipe::StdoutStderr) {
            left.stderr = writer.try_clone()?;
        }
        left.stdout = writer;
        right.stdin = reader;

        commands_with_redirects.push((right, command.inner.redirects.clone()));
    }

    // キャプチャー設定
    if let Some(capture) = capture {
        commands_with_redirects.last_mut().unwrap().0.stdout =
            StdioOutputConfig::PipeWriter(capture);
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
    redirects: Vec<Spanned<Redirect>>,
    env: &mut Context,
) -> Result<()> {
    for redirect in redirects {
        match &redirect.inner {
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
                return Err(Error::InvalidRedirectFileType);
            };

            let file = std::fs::OpenOptions::new()
                .read(true)
                .open(path)
                .map_err(Error::FailOpenRedirectFile)?;
            resolved_command.stdin = StdioInputConfig::File(file);
        }
        // 文字列をそのまま渡す
        HereDoc(s) => {
            resolved_command.stdin = StdioInputConfig::String(s.inner.clone());
        }
        // 文字列を評価してそのまま渡す
        HereString(value) => {
            let value = eval_command_part(value, env)?;
            let Value::String(s) = value
            else {
                return Err(Error::InvalidInputRedirectType);
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
    file_expression: &Spanned<CommandPart>,
    env: &mut Context,
) -> Result<()> {
    use OutputMode::*;
    use OutputRedirect::*;

    // ファイル名の評価
    let value = eval_command_part(file_expression, env)?;
    let Value::String(path) = value
    else {
        return Err(Error::InvalidRedirectFileType);
    };

    // ファイルをモードに応じて開く
    let mut file_opt = std::fs::OpenOptions::new();
    file_opt.create(true);
    file_opt.write(true);
    match output_mode {
        Append => file_opt.append(true),
        Truncate => file_opt.truncate(true),
    };
    let file = file_opt.open(path).map_err(Error::FailOpenRedirectFile)?;
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
        .map(|paths| std::env::split_paths(&paths).collect())
        .unwrap_or_default()
}
