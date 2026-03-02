#![allow(unused)]
use crate::parse::{
    Command, CommandLine, EnvAssign, MergeRedirect, OutputRedirect, Pipe,
    Pipeline, Redirect, ShellCommand, Statement,
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
    command_line: &CommandLine,
    errors: &mut Vec<CheckError>,
) {
    for statement in &command_line.statements {
        check_statement(statement.as_ref(), errors);
    }
}
fn check_statement(statement: &Statement, errors: &mut Vec<CheckError>) {
    use Statement::*;
    match &statement {
        ShellCommand(shell_command) => {
            check_shell_command(shell_command, errors)
        }
        Pipeline(pipeline) => check_pipline(pipeline, errors),
        EnvAssign(env_assign) => check_env_assign(env_assign, errors),
    }
}
fn check_shell_command(
    _shell_command: &ShellCommand,
    _errors: &mut Vec<CheckError>,
) {
    // 型チェックでもする
}
fn check_env_assign(_env_assign: &EnvAssign, _errors: &mut Vec<CheckError>) {
    // 型チェックでもする
}
fn check_pipline(pipeline: &Pipeline, errors: &mut Vec<CheckError>) {
    let pipeline = &pipeline;
    // 最初
    check_fd_count(
        pipeline.first.as_ref(),
        None,
        pipeline.rest.first().map(|(pipe, _)| pipe.as_ref()),
        errors,
    );
    // 最後を除く2番目以降
    let iter = pipeline.rest.iter().zip(pipeline.rest.iter().skip(1));
    for ((pre_pipe, command), (post_pipe, _)) in iter {
        check_fd_count(
            command.as_ref(),
            Some(pre_pipe.as_ref()),
            Some(post_pipe.as_ref()),
            errors,
        );
    }
    // 最後
    if let Some((pre_pipe, command)) = pipeline.rest.last() {
        check_fd_count(command.as_ref(), Some(pre_pipe.as_ref()), None, errors);
    }
}

fn check_fd_count(
    command: &Command,
    pre_pipe: Option<&Pipe>,
    post_pipe: Option<&Pipe>,
    errors: &mut Vec<CheckError>,
) {
    // [in out err]
    let mut count = [0; 3];

    // リダイレクト
    for redirect in &command.redirects {
        match redirect.as_ref() {
            Redirect::Input(_) => count[0] += 1,
            Redirect::Output(((output, _), _)) => match output {
                OutputRedirect::Stdout => count[1] += 1,
                OutputRedirect::Stderr => count[2] += 1,
                OutputRedirect::Both => {
                    count[1] += 1;
                    count[2] += 1;
                }
            },
            Redirect::Merge(merge) => match merge.as_ref() {
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
        match &pipe {
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
