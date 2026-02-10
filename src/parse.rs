pub mod error;
pub mod tools;

use error::*;
use std::fmt::Display;
use tools::*;
use winnow::{
    LocatingSlice,
    combinator::{
        alt, delimited, dispatch, empty, fail, not, opt, peek, preceded,
        repeat, todo as todo_parser,
    },
    prelude::*,
    token::{any, one_of, rest, take_till, take_until, take_while},
};

use crate::parse::tools::ParserExt;

pub type Input<'i> = LocatingSlice<&'i str>;
type Span = std::ops::Range<usize>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spanned<T> {
    inner: T,
    span: Span,
}
impl<T> Spanned<T> {
    pub fn map<U, F>(self, f: F) -> Spanned<U>
    where
        F: FnOnce(T) -> U,
    {
        Spanned {
            inner: f(self.inner),
            span: self.span,
        }
    }
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
    pub name: Spanned<Primary>,
    pub args: Vec<Spanned<Primary>>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Postfix {
    Unwrap,               // !
    IsSome,               // ?
    Length,               // @
    Index(Spanned<Expr>), //[expr]
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Expr {
    Primary(Spanned<Primary>),
    Array(Vec<Spanned<Expr>>),  // [e1, e2, ... , ek]
    Unwrap(Box<Spanned<Expr>>), // expr!
    IsSome(Box<Spanned<Expr>>), // expr?
    Length(Box<Spanned<Expr>>), // expr@
    Index(Box<Spanned<Expr>>, Box<Spanned<Expr>>), // expr1[expr2]
    UnwrapOr(Box<Spanned<Expr>>, Box<Spanned<Expr>>), // expr1^expr2
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
// とりあえずStringで
pub enum Primary {
    Literal(String),           // "abc", 'abc', r"abc"
    PathLiteral(String),       // p"abc"
    SpecialVar(SpecialVar),    // $?, $$, $!, $@
    EnvVar(String),            // $abc
    ShellVar(String),          // %abs
    Paren(Box<Spanned<Expr>>), // (expr)
    Unit,                      // ()
}
impl Display for Primary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Primary::*;
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

type ModalResult<O> = winnow::ModalResult<O, ParseError>;
type SpannedResult<O> = ModalResult<Spanned<O>>;

fn space0<'a>(input: &mut Input<'a>) -> ModalResult<&'a str> {
    take_while(0.., char::is_whitespace).parse_next(input)
}
fn space1<'a>(input: &mut Input<'a>) -> ModalResult<&'a str> {
    take_while(1.., char::is_whitespace).parse_next(input)
}
fn unicode_number(input: &mut Input) -> ModalResult<char> {
    take_until(0.., '}')
        .map_err_with_span(|()| {
            ParseErrorKind::InvalidUnicodeEscape(UnicodeEscapeError::NoEndBrace)
        })
        .try_map_with_span(|input| {
            let code = u32::from_str_radix(input, 16)
                .map_err(ParseErrorKind::ParseHexError)?;
            char::from_u32(code).ok_or(ParseErrorKind::InvalidUnicodeEscape(
                UnicodeEscapeError::InvalidUnicode,
            ))
        })
        .parse_next(input)
}
fn unicode_escape_char(input: &mut Input) -> ModalResult<char> {
    let _ = 'u'.parse_next(input)?;
    let _ = '{'
        .map_err_with_span(|()| {
            ParseErrorKind::InvalidUnicodeEscape(
                UnicodeEscapeError::NoBeginBrace,
            )
        })
        .cut()
        .parse_next(input)?;
    let c = unicode_number.cut().parse_next(input)?;
    let _ = '}'
        .map_err_with_span(|()| {
            ParseErrorKind::InvalidUnicodeEscape(UnicodeEscapeError::NoEndBrace)
        })
        .cut()
        .parse_next(input)?;
    Ok(c)
}
fn escape_char(input: &mut Input) -> ModalResult<char> {
    preceded(
        '\\',
        dispatch!(peek(any);
            'n' => any.value('\n'),
            'r' => any.value('\r'),
            't' => any.value('\t'),
            'u' => unicode_escape_char,
            '\\' => any.value('\\'),
            '\"' => any.value('\"'),
            '\'' => any.value('\''),
            '0' => any.value('\0'),
            c => any.try_map_with_span(|_| {
                Err(ParseErrorKind::UnrecognizedEscape(c))
            }).cut(),
        ),
    )
    .parse_next(input)
}
fn ident(input: &mut Input) -> ModalResult<String> {
    use unicode_ident::*;
    (
        any.map_err_with_span(|()| ParseErrorKind::NoIdent)
            .try_map_with_span(|c| {
                if c == '_' || is_xid_start(c) {
                    Ok(c)
                }
                else {
                    Err(ParseErrorKind::NoIdent)
                }
            }),
        take_while(0.., is_xid_continue),
    )
        .try_map_with_span(|(ident_start, ident_continue)| {
            if ident_start == '_' && ident_continue.is_empty() {
                Err(ParseErrorKind::InvalidIdent)
            }
            else {
                Ok(String::from(ident_start) + ident_continue)
            }
        })
        .cut()
        .parse_next(input)
}

pub fn parse_shell_command(
    input: &str,
) -> Result<
    ShellCommand,
    winnow::error::ParseError<LocatingSlice<&str>, ParseError>,
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
fn pipe(_input: &mut Input) -> ModalResult<Pipe> {
    todo!()
}
fn comment(input: &mut Input) -> ModalResult<String> {
    preceded('#', rest).map(str::to_string).parse_next(input)
}
pub fn command(input: &mut Input) -> ModalResult<Command> {
    Ok(Command {
        name: primary.parse_next(input)?,
        args: repeat(0.., preceded((space1, peek(not('#'))), primary))
            .parse_next(input)?,
    })
}

fn simple_expr_primary(input: &mut Input) -> SpannedResult<Primary> {
    dispatch!(peek(any);
        '\'' => quoted_string.map(Primary::Literal),
        '"' => double_quoted_string.map(Primary::Literal),
        '$' => preceded('$', alt((
            special_var.map(Primary::SpecialVar),
            ident.map(Primary::EnvVar),
        ))),
        '%' => preceded('%', ident).map(Primary::ShellVar),
        '(' => alt((
            ('(', space0, ')').value(Primary::Unit),
            delimited(('(', space0), expr, (space0, ')'))
                .map(|expr| Primary::Paren(Box::new(expr)))
        )),
        'r' => raw_string.map(Primary::Literal),
        'p' =>path_string.map(Primary::PathLiteral),
        _ => fail,
    )
    .spanned()
    .parse_next(input)
}
fn simple_expr_postfix(input: &mut Input) -> SpannedResult<Postfix> {
    dispatch!(any;
        '!' => empty.value(Postfix::Unwrap),
        '?' => empty.value(Postfix::IsSome),
        '@' => empty.value(Postfix::Length),
        '[' => delimited(space0, expr, (space0, ']')).map(Postfix::Index),
        _ => fail,
    )
    .spanned()
    .parse_next(input)
}
pub fn simple_expr(input: &mut Input) -> SpannedResult<Expr> {
    let primary = simple_expr_primary.parse_next(input)?;
    let mut lhs = Spanned {
        span: primary.span.clone(),
        inner: Expr::Primary(primary),
    };
    loop {
        // 後置演算子
        if input
            .as_bytes()
            .first()
            .is_some_and(|c| b"!?@[".contains(c))
        {
            let op = simple_expr_postfix.parse_next(input)?;
            use Postfix::*;
            lhs = Spanned {
                span: lhs.span.start..op.span.end,
                inner: match op.inner {
                    Unwrap => Expr::Unwrap(Box::new(lhs)),
                    IsSome => Expr::IsSome(Box::new(lhs)),
                    Length => Expr::Length(Box::new(lhs)),
                    Index(index) => Expr::Index(Box::new(lhs), Box::new(index)),
                },
            };
            continue;
        }

        // 中置演算子
        if opt('^').parse_next(input)?.is_some() {
            let rhs = simple_expr.parse_next(input)?;
            lhs = Spanned {
                span: lhs.span.start..rhs.span.end,
                inner: Expr::UnwrapOr(Box::new(lhs), Box::new(rhs)),
            };
            continue;
        }

        break;
    }
    Ok(lhs)
}
fn expr(input: &mut Input) -> SpannedResult<Expr> {
    //use winnow::combinator::{Infix, Postfix, Prefix, expression};
    todo!()
}

fn primary(input: &mut Input) -> SpannedResult<Primary> {
    dispatch!(peek(any);
        '\'' => quoted_string.map(Primary::Literal),
        '"' => double_quoted_string.map(Primary::Literal),
        '$' => preceded('$', alt((
            special_var.map(Primary::SpecialVar),
            ident.map(Primary::EnvVar),
        ))),
        '%' => preceded('%', ident).map(Primary::ShellVar),
        '(' => "()".value(Primary::Unit),
        _ => alt((
            raw_string.map(Primary::Literal),
            path_string.map(Primary::PathLiteral),
            unquoted_string.map(Primary::Literal),
        )),
    )
    .spanned()
    .parse_next(input)
}
fn quoted_string(input: &mut Input) -> ModalResult<String> {
    const DELIMITER: char = '\'';
    delimited(
        DELIMITER,
        repeat(0.., alt((escape_char, any.verify(|c| *c != DELIMITER)))),
        DELIMITER
            .map_err_with_span(|()| ParseErrorKind::NoEndQuotation)
            .cut(),
    )
    .parse_next(input)
}
fn double_quoted_string(input: &mut Input) -> ModalResult<String> {
    const DELIMITER: char = '\"';
    delimited(
        DELIMITER,
        repeat(0.., alt((escape_char, any.verify(|c| *c != DELIMITER)))),
        DELIMITER
            .map_err_with_span(|()| ParseErrorKind::NoEndDoubleQuotation)
            .cut(),
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
    dispatch!(any;
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
