use super::*;

pub fn shell_command(input: &mut Input) -> ModalResult<ShellCommand> {
    let _ = space0.parse_next(input)?;
    let commands = repeat(
        0..=1,
        preceded(peek(not('#')), (command, empty.value(None))),
    )
    .parse_next(input)?;
    //commands: repeat(0.., preceded(peek(not('#')), (command, opt(pipe)))).parse_next(input)?,
    let comment = opt(preceded(space0, comment)).parse_next(input)?;
    let _ = space0.parse_next(input)?;
    Ok(ShellCommand { commands, comment })
}
#[allow(unused)]
pub fn pipe(_input: &mut Input) -> ModalResult<Pipe> {
    todo!()
}
pub fn comment(input: &mut Input) -> ModalResult<String> {
    preceded('#', rest).map(str::to_string).parse_next(input)
}
pub fn command_part(input: &mut Input) -> ModalResult<CommandPart> {
    use CommandPart::*;
    alt((
        simple_expr.map(SimpleExpr),
        unquoted_string.spanned().map(Unquoted),
    ))
    .parse_next(input)
}
pub fn command(input: &mut Input) -> ModalResult<Command> {
    Ok(Command {
        name: command_part.parse_next(input)?,
        args: repeat(0.., preceded((space1, peek(not('#'))), command_part))
            .parse_next(input)?,
    })
}
