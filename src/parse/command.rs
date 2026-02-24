use super::*;

pub fn command_line(input: &mut Input) -> SpannedResult<CommandLine> {
    trace(
        "command_line",
        (
            separated(0.., statement, (space0, ';', space0)),
            opt((space0, ';')),
            opt(preceded(space0, comment)),
        ),
    )
    .map(|(statements, _, comment)| CommandLine {
        statements,
        comment,
    })
    .spanned()
    .parse_next(input)
}
pub fn comment(input: &mut Input) -> SpannedResult<String> {
    preceded('#', rest)
        .map(str::to_string)
        .spanned()
        .parse_next(input)
}
pub fn statement(input: &mut Input) -> SpannedResult<Statement> {
    use Statement::*;
    trace(
        "statement",
        alt((
            shell_command.map(ShellCommand),
            env_assign.map(EnvAssign),
            pipeline.map(Pipeline),
        )),
    )
    .spanned()
    .parse_next(input)
}
pub fn shell_command(input: &mut Input) -> SpannedResult<ShellCommand> {
    trace(
        "shell_command",
        (
            shell_command_kind,
            repeat(0.., preceded(space1, command_part)),
        ),
    )
    .map(|(kind, args)| ShellCommand { kind, args })
    .spanned()
    .parse_next(input)
}
pub fn shell_command_kind(
    input: &mut Input,
) -> SpannedResult<ShellCommandKind> {
    unquoted_string
        .verify_map(|name| crate::shell_command::find_shell_command(&name))
        .spanned()
        .parse_next(input)
}
pub fn env_assign(input: &mut Input) -> SpannedResult<EnvAssign> {
    trace(
        "env_assign",
        (preceded('$', ident).spanned(), space1, "=", space1, expr),
    )
    .map(|(name, _, _, _, value)| EnvAssign { name, value })
    .spanned()
    .parse_next(input)
}
pub fn pipeline(input: &mut Input) -> SpannedResult<Pipeline> {
    trace(
        "pipeline",
        (
            command,
            repeat(0.., (delimited(space0, pipe, space0), command)),
        ),
    )
    .map(|(first, rest)| Pipeline { first, rest })
    .spanned()
    .parse_next(input)
}
pub fn pipe(input: &mut Input) -> SpannedResult<Pipe> {
    use Pipe::*;
    trace("pipe", alt(("|&".value(StdoutStderr), "|".value(Stdout))))
        .spanned()
        .parse_next(input)
}
pub fn command(input: &mut Input) -> SpannedResult<Command> {
    trace(
        "command",
        alt((
            // 一時変数付きコマンド
            // コマンドが無ければcut
            (
                separated(1.., temp_env, space1),
                preceded(space1, command_part).cut(),
                args_and_redirects,
            ),
            // 通常のコマンド
            (empty.value(vec![]), command_part, args_and_redirects),
        )),
    )
    .map(|(temp_env, name, (args, redirects))| Command {
        temp_env,
        name,
        args,
        redirects,
    })
    .spanned()
    .parse_next(input)
}
pub fn temp_env(
    input: &mut Input,
) -> SpannedResult<(Spanned<String>, Spanned<Expr>)> {
    trace(
        "temp_env",
        (preceded('$', ident).spanned(), space1, ":=", space1, expr),
    )
    .map(|(var, _, _, _, val)| (var, val))
    .spanned()
    .parse_next(input)
}
fn args_and_redirects(
    input: &mut Input,
) -> ModalResult<(Vec<Spanned<CommandPart>>, Vec<Spanned<Redirect>>)> {
    enum ArgOrRedirect {
        Arg(Spanned<CommandPart>),
        Redirect(Spanned<Redirect>),
    }
    repeat(
        0..,
        preceded(
            space1,
            alt((
                redirect.map(ArgOrRedirect::Redirect),
                command_part.map(ArgOrRedirect::Arg),
            )),
        ),
    )
    .map(|items: Vec<ArgOrRedirect>| {
        let mut args = vec![];
        let mut redirects = vec![];
        for item in items {
            match item {
                ArgOrRedirect::Arg(a) => args.push(a),
                ArgOrRedirect::Redirect(r) => redirects.push(r),
            }
        }
        (args, redirects)
    })
    .parse_next(input)
}
pub fn command_part(input: &mut Input) -> SpannedResult<CommandPart> {
    use CommandPart::*;
    trace(
        "command_part",
        alt((
            simple_expr.map(SimpleExpr),
            unquoted_string.spanned().map(Unquoted),
        )),
    )
    .spanned()
    .parse_next(input)
}
pub fn redirect(input: &mut Input) -> SpannedResult<Redirect> {
    use Redirect::*;
    trace(
        "redirect",
        alt((
            input_redirect.map(Input),
            merge_redirect.map(Merge),
            (output_redirect, preceded(space0, command_part)).map(Output),
        )),
    )
    .spanned()
    .parse_next(input)
}
pub fn input_redirect(input: &mut Input) -> ModalResult<InputRedirect> {
    use InputRedirect::*;
    trace(
        "input_redirect",
        alt((
            preceded(("<<<", space0), command_part).map(HereString),
            //HereDocは後回し
            //preceded("<<", rest.spanned()).map(|doc| HereDoc(doc.map(str::to_string))),
            preceded(("<", space0), command_part).map(File),
        )),
    )
    .parse_next(input)
}
pub fn output_redirect(
    input: &mut Input,
) -> ModalResult<(OutputRedirect, OutputMode)> {
    use OutputMode::*;
    use OutputRedirect::*;
    trace(
        "output_redirect",
        alt((
            "&>>".value((Both, Append)),
            "&>".value((Both, Truncate)),
            "2>>".value((Stderr, Append)),
            "2>".value((Stderr, Truncate)),
            ">>".value((Stdout, Append)),
            ">".value((Stdout, Truncate)),
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
