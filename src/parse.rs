#![allow(unused)]
use winnow::{
    combinator::{
        alt, cut_err, delimited, dispatch, empty, fail, not, opt, peek,
        preceded, repeat, terminated, todo as todo_parser,
    },
    prelude::*,
    token::{any, take_till, take_until, take_while},
};

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
    //Variable(Variable),
}
/*#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Variable {
    Named(String),     // $VAR, ${VAR}
    ExitStatus,        // $?
    Pid,               // $$
    ShellName,         // $0
    Positional(usize), // $1-$9 ${10}...
    ArgCount,          // $#
    AllArgs,           // $@
    AllArgsStr,        // $*
}*/

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

pub fn command(input: &mut &str) -> ModalResult<Command> {
    *input = input.trim();
    let (name, args) =
        (word, repeat(0.., preceded(space1, word))).parse_next(input)?;
    Ok(Command { name, args })
}
pub fn word(input: &mut &str) -> ModalResult<Word> {
    dispatch!(peek(any);
        '\'' => quoted_string.map(Word::Literal),
        '"' => double_quoted_string.map(Word::Literal),
        //'$' => variable.map(Word::Variable),
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
    take_till(1.., |c: char| {
        c.is_whitespace() || "#$(){}|<>;&".contains(c)
    })
    .map(str::to_string)
    .parse_next(input)
}
/*fn variable(input: &mut &str) -> ModalResult<Variable> {
    // まだ
}*/

#[cfg(test)]
mod test;
