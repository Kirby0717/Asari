#![allow(unused)]
use winnow::{
    combinator::{
        alt, cut_err, delimited, dispatch, empty, fail, not, opt, peek,
        preceded, repeat, terminated, todo as todo_parser,
    },
    prelude::*,
    token::{any, take_while},
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Command {
    name: Word,
    args: Vec<Word>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Word {
    Literal(Literal),
    Variable(Variable),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Literal {
    tilde: Option<Tilde>,
    literal: String,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tilde {
    Home,         // ~ （自分のホーム）
    User(String), // ~username
    Pwd,          // ~+
    OldPwd,       // ~-
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Variable {
    Named(String),     // $VAR, ${VAR}
    ExitStatus,        // $?
    Pid,               // $$
    ShellName,         // $0
    Positional(usize), // $1-$9 ${10}...
    ArgCount,          // $#
    AllArgs,           // $@
    AllArgsStr,        // $*
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
    take_while(1..=6, ('0'..='9'))
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
            '~' => empty.value('~'),
            '$' => empty.value('$'),
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
        '\'' => delimited('\'', quoted_string, '\'').map(Word::Literal),
        '\"' => delimited('\"', double_quoted_string, '\"').map(Word::Literal),
        '$' => variable.map(Word::Variable),
        _ => unquoted_string.map(Word::Literal)
    )
    .parse_next(input)
}
fn tilde(input: &mut &str) -> ModalResult<Tilde> {
    // とりあえず単体チルダのみ解析
    let _tilde = '~'.parse_next(input)?;
    Ok(Tilde::Home)
}
fn quoted_string(input: &mut &str) -> ModalResult<Literal> {
    Ok(Literal {
        tilde: opt(tilde).parse_next(input)?,
        literal: repeat(0.., alt((escape_char, any.verify(|c| *c != '\''))))
            .parse_next(input)?,
    })
}
fn double_quoted_string(input: &mut &str) -> ModalResult<Literal> {
    Ok(Literal {
        tilde: opt(tilde).parse_next(input)?,
        literal: repeat(0.., alt((escape_char, any.verify(|c| *c != '\"'))))
            .parse_next(input)?,
    })
}
fn unquoted_string(input: &mut &str) -> ModalResult<Literal> {
    Ok(Literal {
        tilde: opt(tilde).parse_next(input)?,
        literal: repeat(0.., any.verify(|c: &char| !c.is_whitespace()))
            .parse_next(input)?,
    })
}
fn variable(input: &mut &str) -> ModalResult<Variable> {
    // まだ
    todo_parser.parse_next(input)
}

#[cfg(test)]
mod test;
