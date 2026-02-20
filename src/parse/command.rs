use super::*;

pub fn shell_command(input: &mut Input) -> SpannedResult<ShellCommand> {
    (
        repeat(
            0..=1,
            preceded(peek(not('#')), (command, empty.value(None))),
        ),
        //repeat(0.., preceded(peek(not('#')), (command, opt(pipe)))),
        opt(preceded(space0, comment)),
    )
        .map(|(commands, comment)| ShellCommand { commands, comment })
        .spanned()
        .parse_next(input)
}
#[allow(unused)]
pub fn pipe(_input: &mut Input) -> SpannedResult<Pipe> {
    todo!()
}
pub fn comment(input: &mut Input) -> SpannedResult<String> {
    preceded('#', rest)
        .map(str::to_string)
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
pub fn command(input: &mut Input) -> SpannedResult<Command> {
    (
        command_part,
        repeat(0.., preceded((space1, peek(not('#'))), command_part)),
    )
        .map(|(name, args)| Command { name, args })
        .spanned()
        .parse_next(input)
}
