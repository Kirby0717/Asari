use super::*;

pub fn shell_command(input: &mut Input) -> SpannedResult<ShellCommand> {
    (
        separated(0.., pipeline, (space0, ';', space0)),
        opt((space0, ';')),
        opt(preceded(space0, comment)),
    )
        .map(|(pipelines, _, comment)| ShellCommand { pipelines, comment })
        .spanned()
        .parse_next(input)
}
pub fn comment(input: &mut Input) -> SpannedResult<String> {
    preceded('#', rest)
        .map(str::to_string)
        .spanned()
        .parse_next(input)
}
pub fn pipeline(input: &mut Input) -> SpannedResult<Pipeline> {
    (
        command,
        repeat(0.., (delimited(space0, pipe, space0), command)),
    )
        .map(|(first, rest)| Pipeline { first, rest })
        .spanned()
        .parse_next(input)
}
pub fn pipe(input: &mut Input) -> SpannedResult<Pipe> {
    use Pipe::*;
    alt(("|&".value(StdoutStderr), "|".value(Stdout)))
        .spanned()
        .parse_next(input)
}
pub fn command(input: &mut Input) -> SpannedResult<Command> {
    enum ArgOrRedirect {
        Arg(Spanned<CommandPart>),
        Redirect(Spanned<Redirect>),
    }
    (
        command_part,
        repeat(
            0..,
            alt((
                preceded(space0, redirect).map(ArgOrRedirect::Redirect),
                preceded(space1, command_part).map(ArgOrRedirect::Arg),
            )),
        ),
    )
        .map(|(name, arg_or_redirect): (_, Vec<_>)| {
            let mut args = vec![];
            let mut redirects = vec![];
            for arg_or_redirect in arg_or_redirect {
                match arg_or_redirect {
                    ArgOrRedirect::Arg(arg) => args.push(arg),
                    ArgOrRedirect::Redirect(redirect) => {
                        redirects.push(redirect)
                    }
                }
            }
            Command {
                name,
                args,
                redirects,
            }
        })
        .spanned()
        .parse_next(input)
}
pub fn command_part(input: &mut Input) -> SpannedResult<CommandPart> {
    use CommandPart::*;
    alt((
        simple_expr.map(SimpleExpr),
        unquoted_string.spanned().map(Unquoted),
    ))
    .spanned()
    .parse_next(input)
}
pub fn redirect(input: &mut Input) -> SpannedResult<Redirect> {
    use Redirect::*;
    alt((
        input_redirect.map(Input),
        merge_redirect.map(Merge),
        (output_redirect, preceded(space0, command_part)).map(Output),
    ))
    .spanned()
    .parse_next(input)
}
pub fn input_redirect(input: &mut Input) -> ModalResult<InputRedirect> {
    use InputRedirect::*;
    alt((
        preceded(("<<<", space0), command_part).map(HereString),
        //HereDocは後回し
        //preceded("<<", rest.spanned()).map(|doc| HereDoc(doc.map(str::to_string))),
        preceded(("<", space0), command_part).map(File),
    ))
    .parse_next(input)
}
pub fn output_redirect(input: &mut Input) -> ModalResult<OutputRedirect> {
    use OutputMode::*;
    use OutputRedirect::*;
    alt((
        "&>>".value(Both(Append)),
        "&>".value(Both(Truncate)),
        "2>>".value(Stderr(Append)),
        "2>".value(Stderr(Truncate)),
        ">>".value(Stdout(Append)),
        ">".value(Stdout(Truncate)),
    ))
    .parse_next(input)
}
pub fn merge_redirect(input: &mut Input) -> ModalResult<MergeRedirect> {
    use MergeRedirect::*;
    alt(("2>&1".value(StderrToStdout), "1>&2".value(StdoutToStderr)))
        .parse_next(input)
}
