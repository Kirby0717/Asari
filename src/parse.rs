#![allow(unused)]
use winnow::{
    combinator::{alt, delimited, not, preceded, repeat, terminated},
    prelude::*,
    token::{any, take_while},
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Command {
    name: String,
    args: Vec<String>,
}

fn is_space(c: &char) -> bool {
    [' ', '\t'].contains(c)
}
fn space(input: &mut &str) -> ModalResult<char> {
    any.verify(is_space).parse_next(input)
}
fn not_space(input: &mut &str) -> ModalResult<char> {
    any.verify(|c| !is_space(c)).parse_next(input)
}
fn space0(input: &mut &str) -> ModalResult<String> {
    repeat(0.., space).parse_next(input)
}
fn space1(input: &mut &str) -> ModalResult<String> {
    repeat(1.., space).parse_next(input)
}
fn escape_char(input: &mut &str) -> ModalResult<char> {
    preceded('\\', any).parse_next(input)
}

pub fn command(input: &mut &str) -> ModalResult<Command> {
    *input = input.trim();
    let (name, args) = (
        preceded(space0, ident),
        repeat(0.., preceded(space1, ident)),
    )
        .parse_next(input)?;
    Ok(Command { name, args })
}

pub fn ident(input: &mut &str) -> ModalResult<String> {
    repeat(1.., alt((quoted_string, double_quoted_string, string)))
        .map(|strings: Vec<String>| strings.concat())
        .parse_next(input)
}
fn string(input: &mut &str) -> ModalResult<String> {
    repeat(
        1..,
        alt((escape_char, not_space.verify(|c| !['\'', '\"'].contains(c)))),
    )
    .parse_next(input)
}
fn quoted_string(input: &mut &str) -> ModalResult<String> {
    delimited('\'', repeat(0.., any.verify(|c| *c != '\'')), '\'')
        .parse_next(input)
}
pub fn double_quoted_string(input: &mut &str) -> ModalResult<String> {
    delimited(
        '\"',
        repeat(0.., alt((escape_char, any.verify(|c| *c != '\"')))),
        '\"',
    )
    .parse_next(input)
}

#[cfg(test)]
mod test;
