use super::*;

pub fn command_line(input: &mut Input) -> ModalResult<CommandLine> {
    trace("command_line", |input: &mut Input| {
        let statements =
            separated(0.., statement.spanned(), (space0, ';', space0))
                .parse_next(input)?;
        let _ = opt((space0, ';')).parse_next(input)?;
        let comment =
            opt(preceded(space0, comment.spanned())).parse_next(input)?;
        Ok(CommandLine {
            statements,
            comment,
        })
    })
    .parse_next(input)
}
pub fn comment(input: &mut Input) -> ModalResult<String> {
    trace("comment", |input: &mut Input| {
        let comment =
            preceded('#', rest).map(str::to_string).parse_next(input)?;
        Ok(comment)
    })
    .parse_next(input)
}
pub fn statement(input: &mut Input) -> ModalResult<Statement> {
    use Statement::*;
    trace(
        "statement",
        alt((
            shell_command.spanned().map(ShellCommand),
            env_assign.spanned().map(EnvAssign),
            pipeline.spanned().map(Pipeline),
        )),
    )
    .parse_next(input)
}
pub fn shell_command(input: &mut Input) -> ModalResult<ShellCommand> {
    use CommandError::*;
    trace("shell_command", |input: &mut Input| {
        let kind = shell_command_kind.spanned().parse_next(input)?;
        let args = repeat(
            0..,
            preceded(space0, |input: &mut Input| {
                // パイプ、リダイレクトは禁止
                alt(("|", "|&"))
                    .reject_with_span(|_| InvalidPipe)
                    .parse_next(input)?;
                alt((
                    "<<<", "<<", "<", "2>&1", "1>&2", "&>>", "&>", "2>>", "2>",
                    ">>", ">",
                ))
                .reject_with_span(|_| InvalidRedirect)
                .parse_next(input)?;

                command_part.spanned().parse_next(input)
            }),
        )
        .parse_next(input)?;
        Ok(ShellCommand { kind, args })
    })
    .parse_next(input)
}
pub fn shell_command_kind(input: &mut Input) -> ModalResult<ShellCommandKind> {
    unquoted_string
        .verify_map(|name| crate::shell_command::find_shell_command(&name))
        .parse_next(input)
}
pub fn env_assign(input: &mut Input) -> ModalResult<EnvAssign> {
    trace("env_assign", |input: &mut Input| {
        let name = env_var.spanned().parse_next(input)?;
        let _ = space0.parse_next(input)?;
        let _ = '='.parse_next(input)?;
        let _ = space0.parse_next(input)?;
        let value = expr.spanned().cut().parse_next(input)?;
        Ok(EnvAssign { name, value })
    })
    .parse_next(input)
}
pub fn pipeline(input: &mut Input) -> ModalResult<Pipeline> {
    trace("pipeline", |input: &mut Input| {
        let first = command.spanned().parse_next(input)?;
        let rest = repeat(0.., |input: &mut Input| {
            let pipe =
                delimited(space0, pipe.spanned(), space0).parse_next(input)?;
            let command = command.spanned().parse_next(input)?;
            Ok((pipe, command))
        })
        .parse_next(input)?;
        Ok(Pipeline { first, rest })
    })
    .parse_next(input)
}
pub fn pipe(input: &mut Input) -> ModalResult<Pipe> {
    use Pipe::*;
    trace("pipe", alt(("|&".value(StdoutStderr), "|".value(Stdout))))
        .parse_next(input)
}
pub fn command(input: &mut Input) -> ModalResult<Command> {
    trace("command", |input: &mut Input| {
        let temp_envs = repeat(0.., terminated(temp_env.spanned(), space0))
            .parse_next(input)?;
        let name = command_part.spanned().parse_next(input)?;
        let mut args = vec![];
        let mut redirects = vec![];
        loop {
            if let Some(redirect) =
                opt(preceded(space0, redirect.spanned())).parse_next(input)?
            {
                redirects.push(redirect);
                continue;
            }
            if let Some(arg) = opt(preceded(space0, command_part.spanned()))
                .parse_next(input)?
            {
                args.push(arg);
                continue;
            }
            break;
        }
        Ok(Command {
            temp_envs,
            name,
            args,
            redirects,
        })
    })
    .parse_next(input)
}
pub fn temp_env(
    input: &mut Input,
) -> ModalResult<(Spanned<String>, Spanned<Expr>)> {
    trace("temp_env", |input: &mut Input| {
        let var = env_var.spanned().parse_next(input)?;
        let _ = space0.parse_next(input)?;
        let _ = ":=".parse_next(input)?;
        let _ = space0.parse_next(input)?;
        let val = expr
            .spanned()
            .map_err_with_span(|_| CommandError::NoValue)
            .cut()
            .parse_next(input)?;
        Ok((var, val))
    })
    .parse_next(input)
}
pub fn command_part(input: &mut Input) -> ModalResult<CommandPart> {
    use CommandPart::*;
    trace(
        "command_part",
        alt((
            simple_expr.spanned().map(SimpleExpr),
            unquoted_string.spanned().map(Unquoted),
        )),
    )
    .parse_next(input)
}
pub fn redirect(input: &mut Input) -> ModalResult<Redirect> {
    use Redirect::*;
    trace(
        "redirect",
        alt((
            merge_redirect.map(Merge),
            input_redirect.map(Input),
            output_redirect.map(Output),
        )),
    )
    .parse_next(input)
}
pub fn merge_redirect(input: &mut Input) -> ModalResult<MergeRedirect> {
    use MergeRedirect::*;
    trace(
        "merge_redirect",
        alt(("2>&1".value(StderrToStdout), "1>&2".value(StdoutToStderr))),
    )
    .parse_next(input)
}
pub fn input_redirect(input: &mut Input) -> ModalResult<InputRedirect> {
    use InputRedirect::*;
    trace(
        "input_redirect",
        alt((
            preceded(("<<<", space0), command_part.spanned()).map(HereString),
            //HereDocは後回し
            //preceded("<<", rest.spanned()).map(|doc| HereDoc(doc.map(str::to_string))),
            preceded(("<", space0), command_part.spanned()).map(File),
        )),
    )
    .parse_next(input)
}
pub fn output_redirect(
    input: &mut Input,
) -> ModalResult<((OutputRedirect, OutputMode), Spanned<CommandPart>)> {
    use OutputMode::*;
    use OutputRedirect::*;
    trace(
        "output_redirect",
        (
            alt((
                "&>>".value((Both, Append)),
                "&>".value((Both, Truncate)),
                "2>>".value((Stderr, Append)),
                "2>".value((Stderr, Truncate)),
                ">>".value((Stdout, Append)),
                ">".value((Stdout, Truncate)),
            )),
            preceded(space0, command_part.spanned()),
        ),
    )
    .parse_next(input)
}
