use crate::parse::{
    Command, CommandLine, EnvAssign, MergeRedirect, OutputRedirect, Pipe,
    Pipeline, Redirect, ShellCommand, Spanned, Statement,
};

#[derive(Clone, Debug)]
pub enum CheckError {
    Redirect(RedirectError),
}
#[derive(Clone, Debug)]
pub enum RedirectError {
    DuplicateStdin,
    DuplicateStdout,
    DuplicateStderr,
}

pub fn check_command_line(
    command_line: &Spanned<CommandLine>,
    errors: &mut Vec<CheckError>,
) {
    for statement in &command_line.inner.statements {
        check_statement(statement, errors);
    }
}
fn check_statement(
    statement: &Spanned<Statement>,
    errors: &mut Vec<CheckError>,
) {
    use Statement::*;
    match &statement.inner {
        ShellCommand(shell_command) => {
            check_shell_command(shell_command, errors)
        }
        Pipeline(pipeline) => check_pipline(pipeline, errors),
        EnvAssign(env_assign) => check_env_assign(env_assign, errors),
    }
}
fn check_shell_command(
    _shell_command: &Spanned<ShellCommand>,
    _errors: &mut Vec<CheckError>,
) {
    // 型チェックでもする
}
fn check_env_assign(
    _env_assign: &Spanned<EnvAssign>,
    _errors: &mut Vec<CheckError>,
) {
    // 型チェックでもする
}
fn check_pipline(pipeline: &Spanned<Pipeline>, errors: &mut Vec<CheckError>) {
    let pipeline = &pipeline.inner;
    // 最初
    check_fd_count(
        &pipeline.first,
        None,
        pipeline.rest.first().map(|(pipe, _)| pipe),
        errors,
    );
    // 最後を除く2番目以降
    let iter = pipeline.rest.iter().zip(pipeline.rest.iter().skip(1));
    for ((pre_pipe, command), (post_pipe, _)) in iter {
        check_fd_count(command, Some(pre_pipe), Some(post_pipe), errors);
    }
    // 最後
    if let Some((pre_pipe, command)) = pipeline.rest.last() {
        check_fd_count(command, Some(pre_pipe), None, errors);
    }
}

fn check_fd_count(
    command: &Spanned<Command>,
    pre_pipe: Option<&Spanned<Pipe>>,
    post_pipe: Option<&Spanned<Pipe>>,
    errors: &mut Vec<CheckError>,
) {
    // [in out err]
    let mut count = [0; 3];

    // リダイレクト
    for redirect in &command.inner.redirects {
        match &redirect.inner {
            Redirect::Input(_) => count[0] += 1,
            Redirect::Output(((output, _), _)) => match output {
                OutputRedirect::Stdout => count[1] += 1,
                OutputRedirect::Stderr => count[2] += 1,
                OutputRedirect::Both => {
                    count[1] += 1;
                    count[2] += 1;
                }
            },
            Redirect::Merge(merge) => match merge {
                MergeRedirect::StdoutToStderr => count[1] += 1,
                MergeRedirect::StderrToStdout => count[2] += 1,
            },
        }
    }

    // パイプ
    if pre_pipe.is_some() {
        count[0] += 1;
    }
    if let Some(pipe) = post_pipe {
        match &pipe.inner {
            Pipe::Stdout => count[1] += 1,
            Pipe::StdoutStderr => {
                count[1] += 1;
                count[2] += 1;
            }
        }
    }

    // エラーの追加
    if 2 <= count[0] {
        errors.push(CheckError::Redirect(RedirectError::DuplicateStdin));
    }
    if 2 <= count[1] {
        errors.push(CheckError::Redirect(RedirectError::DuplicateStdout));
    }
    if 2 <= count[2] {
        errors.push(CheckError::Redirect(RedirectError::DuplicateStderr));
    }
}
