#![allow(unused)]
use winnow::{
    combinator::{
        alt, cut_err, delimited, dispatch, empty, fail, not, opt, peek,
        preceded, repeat, separated, terminated, todo as todo_parser,
    },
    prelude::*,
    token::{any, rest, take_till, take_until, take_while},
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShellCommand {
    commands: Vec<(Command, Option<Pipe>)>,
    comment: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pipe {
    Split,
    Pipe,
    In,
    Out,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Command {
    name: Word,
    args: Vec<Word>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Word {
    Literal(String),
    // とりあえずStringで
    PathLiteral(String),
    SpecialVar(SpecialVar),
    EnvVar(String),
    ShellVar(String),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpecialVar {
    ExitStatus,    // $?
    Pid,           // $$
    BackgroundPid, // $!
    ShellName,     // $@
}

fn space0(input: &mut &str) -> ModalResult<String> {
    take_while(0.., char::is_whitespace)
        .map(str::to_string)
        .parse_next(input)
}
fn space1(input: &mut &str) -> ModalResult<String> {
    take_while(1.., char::is_whitespace)
        .map(str::to_string)
        .parse_next(input)
}
fn unicode_number(input: &mut &str) -> ModalResult<char> {
    take_while(1..=6, (('0'..='9'), ('A'..='F'), ('a'..='f')))
        .verify_map(|num: &str| {
            u32::from_str_radix(num, 16).ok().and_then(char::from_u32)
        })
        .parse_next(input)
}
fn escape_char(input: &mut &str) -> ModalResult<char> {
    preceded(
        '\\',
        dispatch!(any;
            'n' => empty.value('\n'),
            'r' => empty.value('\r'),
            't' => empty.value('\t'),
            'u' => cut_err(delimited('{', unicode_number, '}')),
            '\\' => empty.value('\\'),
            '\"' => empty.value('\"'),
            '\'' => empty.value('\''),
            '0' => empty.value('\0'),
            _ => cut_err(fail)
        ),
    )
    .parse_next(input)
}
fn ident(input: &mut &str) -> ModalResult<String> {
    use unicode_xid::UnicodeXID;
    alt((
        (
            any.verify(|c: &char| c.is_xid_start()),
            take_while(0.., char::is_xid_continue),
        ),
        ('_', take_while(1.., char::is_xid_continue)),
    ))
    .map(|(start, r#continue)| String::from(start) + r#continue)
    .parse_next(input)
}

pub fn shell_command(input: &mut &str) -> ModalResult<ShellCommand> {
    *input = input.trim_start();
    Ok(ShellCommand {
        commands: repeat(
            0..=1,
            preceded(peek(not('#')), (command, empty.value(None))),
        )
        .parse_next(input)?,
        //commands: repeat(0.., preceded(peek(not('#')), (command, opt(pipe)))).parse_next(input)?,
        comment: opt(preceded(space0, comment)).parse_next(input)?,
    })
}
fn pipe(input: &mut &str) -> ModalResult<Pipe> {
    todo!()
}
fn comment(input: &mut &str) -> ModalResult<String> {
    preceded('#', rest).map(str::to_string).parse_next(input)
}
pub fn command(input: &mut &str) -> ModalResult<Command> {
    Ok(Command {
        name: word.parse_next(input)?,
        args: repeat(0.., preceded((space1, peek(not('#'))), word))
            .parse_next(input)?,
    })
}
fn word(input: &mut &str) -> ModalResult<Word> {
    dispatch!(peek(any);
        '\'' => quoted_string.map(Word::Literal),
        '"' => double_quoted_string.map(Word::Literal),
        '$' => alt((
            preceded('$', ident).map(Word::EnvVar),
            special_var.map(Word::SpecialVar),
        )),
        '%' => preceded('%', ident).map(Word::ShellVar),
        _ => alt((
            raw_string.map(Word::Literal),
            path_string.map(Word::PathLiteral),
            unquoted_string.map(Word::Literal),
        ))
    )
    .parse_next(input)
}
fn quoted_string(input: &mut &str) -> ModalResult<String> {
    const DELIMITER: char = '\'';
    delimited(
        DELIMITER,
        repeat(0.., alt((escape_char, any.verify(|c| *c != DELIMITER)))),
        DELIMITER,
    )
    .parse_next(input)
}
fn double_quoted_string(input: &mut &str) -> ModalResult<String> {
    const DELIMITER: char = '\"';
    delimited(
        DELIMITER,
        repeat(0.., alt((escape_char, any.verify(|c| *c != DELIMITER)))),
        DELIMITER,
    )
    .parse_next(input)
}
fn raw_string(input: &mut &str) -> ModalResult<String> {
    let _ = 'r'.parse_next(input)?;
    let sharp = take_while(0.., '#').parse_next(input)?;
    let _ = '"'.parse_next(input)?;
    let delimiter = '"'.to_string() + sharp;
    let raw = take_until(0.., delimiter.as_str()).parse_next(input)?;
    let _ = delimiter.as_str().parse_next(input)?;
    Ok(raw.to_string())
}
fn path_string(input: &mut &str) -> ModalResult<String> {
    let _ = 'p'.parse_next(input)?;
    let sharp = take_while(0.., '#').parse_next(input)?;
    let _ = '"'.parse_next(input)?;
    let delimiter = '"'.to_string() + sharp;
    let raw = take_until(0.., delimiter.as_str()).parse_next(input)?;
    let _ = delimiter.as_str().parse_next(input)?;
    Ok(raw.to_string())
}
fn unquoted_string(input: &mut &str) -> ModalResult<String> {
    take_till(1.., |c: char| c.is_whitespace() || "(){}|<>;&".contains(c))
        .map(str::to_string)
        .parse_next(input)
}
fn special_var(input: &mut &str) -> ModalResult<SpecialVar> {
    dispatch!(preceded('$', any);
        '?' => empty.value(SpecialVar::ExitStatus),
        '$' => empty.value(SpecialVar::Pid),
        '!' => empty.value(SpecialVar::BackgroundPid),
        '@' => empty.value(SpecialVar::ShellName),
        _ => fail,
    )
    .parse_next(input)
}

#[cfg(test)]
mod test;
