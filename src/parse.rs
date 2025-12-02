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
    SpecialVar(SpecialVar),
    EnvVar(EnvVar),
    LocalVar(LocalVar),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EnvVar {
    name: String,
    modifier: Option<VarModifier>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalVar {
    name: String,
    modifier: Option<VarModifier>,
    //type_annotation: Option<Type>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VarModifier {
    Length, // $#VAR, %#VAR
    Exists, // $?VAR, %?VAR
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpecialVar {
    ExitStatus, // $?
    Pid,        // $$
    ShellName,  // $@
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
fn ident_name(input: &mut &str) -> ModalResult<String> {
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
        '$' => alt((
            env_var.map(Word::EnvVar),
            special_var.map(Word::SpecialVar),
        )),
        '%' => local_var.map(Word::LocalVar),
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
        c.is_whitespace() || "#$%(){}|<>;&".contains(c)
    })
    .map(str::to_string)
    .parse_next(input)
}
fn special_var(input: &mut &str) -> ModalResult<SpecialVar> {
    dispatch!(preceded('$', any);
        '?' => empty.value(SpecialVar::ExitStatus),
        '$' => empty.value(SpecialVar::Pid),
        '@' => empty.value(SpecialVar::ShellName),
        _ => fail,
    )
    .parse_next(input)
}
fn modifier(input: &mut &str) -> ModalResult<VarModifier> {
    alt((
        '#'.value(VarModifier::Length),
        '?'.value(VarModifier::Exists),
    ))
    .parse_next(input)
}
fn env_var(input: &mut &str) -> ModalResult<EnvVar> {
    Ok(EnvVar {
        modifier: preceded('$', opt(modifier)).parse_next(input)?,
        name: ident_name.parse_next(input)?,
    })
}
fn local_var(input: &mut &str) -> ModalResult<LocalVar> {
    Ok(LocalVar {
        modifier: preceded('%', opt(modifier)).parse_next(input)?,
        name: ident_name.parse_next(input)?,
    })
}

#[cfg(test)]
mod test;
