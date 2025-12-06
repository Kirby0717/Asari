#![allow(unused)]
use std::fmt::{Display, write};
use winnow::{
    LocatingSlice,
    combinator::{
        alt, cut_err, delimited, dispatch, empty, fail, not, opt, peek,
        preceded, repeat, separated, terminated, todo as todo_parser,
    },
    error::{
        AddContext, ContextError, ErrMode, ParserError, StrContext,
        StrContextValue,
    },
    prelude::*,
    stream::Offset,
    token::{any, rest, take_till, take_until, take_while},
};

type Input<'i> = LocatingSlice<&'i str>;
type Span = std::ops::Range<usize>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spanned<T> {
    inner: T,
    span: Span,
}
impl<T: PartialOrd> PartialOrd for Spanned<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}
impl<T: Ord> Ord for Spanned<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}
impl<T: Display> Display for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

fn spanned<T>(input: (T, Span)) -> Spanned<T> {
    Spanned {
        inner: input.0,
        span: input.1,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShellCommand {
    pub commands: Vec<(Command, Option<Pipe>)>,
    pub comment: Option<String>,
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
    pub name: Spanned<Word>,
    pub args: Vec<Spanned<Word>>,
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
impl Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Word::*;
        match self {
            Literal(literal) => write!(f, "{literal}"),
            _ => todo!(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpecialVar {
    ExitStatus,    // $?
    Pid,           // $$
    BackgroundPid, // $!
    ShellName,     // $@
}
#[derive(Clone, Debug)]
pub enum ParseErrorKind {}
#[derive(Clone, Debug)]
pub struct ParseError {
    //kind: ParseErrorKind,
    span: Span,
    context: Option<String>,
}
impl winnow::error::ParserError<Input<'_>> for ParserError {
    fn or(self, other: Self) -> Self {
        
    }
    fn from_input(input: &Input<'_>) -> Self {}
}

fn space0<'a>(input: &mut Input<'a>) -> ModalResult<&'a str> {
    take_while(0.., char::is_whitespace).parse_next(input)
}
fn space1<'a>(input: &mut Input<'a>) -> ModalResult<&'a str> {
    take_while(1.., char::is_whitespace).parse_next(input)
}
fn unicode_number(input: &mut Input) -> ModalResult<char> {
    let hex = take_while(1.., char::is_hex_digit)
        .context(StrContext::Label("Unicode エスケープ"))
        .context(StrContext::Expected(StrContextValue::Description("16進数")))
        .parse_next(input)?;

    /*take_while(1..=6, (('0'..='9'), ('A'..='F'), ('a'..='f')))
    .verify_map(|num: &str| {
        u32::from_str_radix(num, 16).ok().and_then(char::from_u32)
    })
    .parse_next(input)*/
    todo!()
}
fn escape_char(input: &mut Input) -> ModalResult<char> {
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
fn ident(input: &mut Input) -> ModalResult<String> {
    use unicode_ident::*;
    alt((
        (
            any.verify(|c: &char| is_xid_start(*c)),
            take_while(0.., is_xid_continue),
        ),
        ('_', take_while(1.., is_xid_continue)),
    ))
    .map(|(start, r#continue)| String::from(start) + r#continue)
    .parse_next(input)
}

pub fn parse_shell_command(
    input: &str,
) -> Result<
    ShellCommand,
    winnow::error::ParseError<LocatingSlice<&str>, ContextError>,
> {
    shell_command.parse(Input::new(input))
}
fn shell_command(input: &mut Input) -> ModalResult<ShellCommand> {
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
fn pipe(input: &mut Input) -> ModalResult<Pipe> {
    todo!()
}
fn comment(input: &mut Input) -> ModalResult<String> {
    preceded('#', rest).map(str::to_string).parse_next(input)
}
pub fn command(input: &mut Input) -> ModalResult<Command> {
    Ok(Command {
        name: word.parse_next(input)?,
        args: repeat(0.., preceded((space1, peek(not('#'))), word))
            .parse_next(input)?,
    })
}
fn word(input: &mut Input) -> ModalResult<Spanned<Word>> {
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
    .with_span()
    .map(spanned)
    .parse_next(input)
}
fn quoted_string(input: &mut Input) -> ModalResult<String> {
    const DELIMITER: char = '\'';
    delimited(
        DELIMITER,
        repeat(0.., alt((escape_char, any.verify(|c| *c != DELIMITER)))),
        DELIMITER,
    )
    .parse_next(input)
}
fn double_quoted_string(input: &mut Input) -> ModalResult<String> {
    const DELIMITER: char = '\"';
    delimited(
        DELIMITER,
        repeat(0.., alt((escape_char, any.verify(|c| *c != DELIMITER)))),
        DELIMITER,
    )
    .parse_next(input)
}
fn raw_string(input: &mut Input) -> ModalResult<String> {
    let _ = 'r'.parse_next(input)?;
    let sharp = take_while(0.., '#').parse_next(input)?;
    let _ = '"'.parse_next(input)?;
    let delimiter = '"'.to_string() + sharp;
    let raw = take_until(0.., delimiter.as_str()).parse_next(input)?;
    let _ = delimiter.as_str().parse_next(input)?;
    Ok(raw.to_string())
}
fn path_string(input: &mut Input) -> ModalResult<String> {
    let _ = 'p'.parse_next(input)?;
    let sharp = take_while(0.., '#').parse_next(input)?;
    let _ = '"'.parse_next(input)?;
    let delimiter = '"'.to_string() + sharp;
    let raw = take_until(0.., delimiter.as_str()).parse_next(input)?;
    let _ = delimiter.as_str().parse_next(input)?;
    Ok(raw.to_string())
}
fn unquoted_string(input: &mut Input) -> ModalResult<String> {
    take_till(1.., |c: char| c.is_whitespace() || "(){}|<>;&".contains(c))
        .map(str::to_string)
        .parse_next(input)
}
fn special_var(input: &mut Input) -> ModalResult<SpecialVar> {
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
