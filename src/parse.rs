pub mod command;
pub mod error;
pub mod expr;
pub mod literal;
pub mod simple_expr;
pub mod tools;

use super::value::Type;
use command::*;
use error::*;
use expr::*;
use literal::*;
use simple_expr::*;
use tools::*;

pub use simple_expr::simple_expr;

#[allow(unused_imports)]
use winnow::combinator::todo as todo_parser;
use winnow::{
    LocatingSlice,
    combinator::{
        alt, delimited, dispatch, empty, fail, not, opt, peek, preceded,
        repeat, separated, trace,
    },
    prelude::*,
    token::{any, rest, take, take_till, take_until, take_while},
};

pub type Input<'i> = LocatingSlice<&'i str>;
type Span = std::ops::Range<usize>;

fn mix_span(a: &Span, b: &Span) -> Span {
    a.start.min(b.start)..a.end.max(b.end)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Spanned<T> {
    pub inner: T,
    pub span: Span,
}
impl<T: PartialOrd> PartialOrd for Spanned<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}
impl<T: std::fmt::Display> std::fmt::Display for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct ShellCommand {
    pub commands: Vec<(Command, Option<Pipe>)>,
    pub comment: Option<String>,
}
#[allow(unused)]
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Pipe {
    Split,
    Pipe,
    In,
    Out,
}
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum CommandPart {
    Unquoted(Spanned<String>),
    SimpleExpr(Spanned<Expr>),
}
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Command {
    pub name: CommandPart,
    pub args: Vec<CommandPart>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum ExprPrefix {
    Not, // !
    Neg, // -
}
impl ExprPrefix {
    fn power(&self) -> i32 {
        use ExprPrefix::*;
        match self {
            Not => 9,
            Neg => 9,
        }
    }
}
impl Spanned<ExprPrefix> {
    fn apply(self, expr: Spanned<Expr>) -> Spanned<Expr> {
        let expr = Box::new(expr);
        Spanned {
            span: mix_span(&self.span, &expr.span),
            inner: Expr::Prefix(expr, self.inner),
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum ExprInfix {
    UnwrapOr,     // ^
    Add,          // +
    Sub,          // -
    Mul,          // *
    Div,          // /
    Rem,          // %
    Equal,        // ==
    NotEqual,     // !=
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=
    And,          // &&
    Or,           // ||
}
impl ExprInfix {
    #[rustfmt::skip]
    fn power(&self) -> (i32, i32) {
        use ExprInfix::*;
        let right = |power| (power, power - 1);
        let left = |power| (power, power + 1);
        match self {
            UnwrapOr     => right(1),
            Add          => left(6),
            Sub          => left(6),
            Mul          => left(7),
            Div          => left(7),
            Rem          => left(7),
            Equal        => left(4),
            NotEqual     => left(4),
            Less         => left(5),
            LessEqual    => left(5),
            Greater      => left(5),
            GreaterEqual => left(5),
            And          => left(3),
            Or           => left(2),
        }
    }
}
impl Spanned<ExprInfix> {
    fn apply(
        self,
        l_expr: Spanned<Expr>,
        r_expr: Spanned<Expr>,
    ) -> Spanned<Expr> {
        let l_expr = Box::new(l_expr);
        let r_expr = Box::new(r_expr);
        Spanned {
            span: mix_span(&l_expr.span, &r_expr.span),
            inner: Expr::Infix(l_expr, r_expr, self.inner),
        }
    }
}
type ExprNode = Box<Spanned<Expr>>;
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum ExprPostfix {
    Unwrap,              // !
    IsSome,              // ?
    Length,              // @
    Index(ExprNode),     // [expr]
    Cast(Spanned<Type>), // as type
}
impl ExprPostfix {
    fn power(&self) -> i32 {
        use ExprPostfix::*;
        match self {
            Unwrap => 10,
            IsSome => 10,
            Length => 10,
            Index(_) => 10,
            Cast(_) => 9,
        }
    }
}
impl Spanned<ExprPostfix> {
    fn apply(self, expr: Spanned<Expr>) -> Spanned<Expr> {
        let expr = Box::new(expr);
        Spanned {
            span: mix_span(&expr.span, &self.span),
            inner: Expr::Postfix(expr, self.inner),
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Expr {
    Primary(Spanned<Primary>),
    Prefix(ExprNode, ExprPrefix),
    Infix(ExprNode, ExprNode, ExprInfix),
    Postfix(ExprNode, ExprPostfix),
}
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Primary {
    String(String),                     // "abc", 'abc', r"abc"
    PathString(String),                 // p"abc"
    SpecialVar(SpecialVar),             // $?, $$, $!, $@
    EnvVar(String),                     // $abc
    ShellVar(String),                   // @abc
    Paren(Box<Spanned<Expr>>),          // (expr)
    Array(Vec<Spanned<Expr>>),          // [e1, e2, ... , ek]
    Bool(bool),                         // true, false
    Int(i64),                           // 123
    Float(f64),                         // 12.3
    Option(Option<Box<Spanned<Expr>>>), // none, some(expr)
    Unit,                               // ()
}
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum SpecialVar {
    ExitStatus,    // $?
    Pid,           // $$
    BackgroundPid, // $!
    ShellName,     // $@
}

type ModalResult<O> = winnow::ModalResult<O, ParseError>;
type SpannedResult<O> = ModalResult<Spanned<O>>;

pub fn parse_shell_command(
    input: &str,
) -> Result<
    ShellCommand,
    winnow::error::ParseError<LocatingSlice<&str>, ParseError>,
> {
    shell_command.parse(Input::new(input))
}

fn space0<'a>(input: &mut Input<'a>) -> ModalResult<&'a str> {
    take_while(0.., char::is_whitespace).parse_next(input)
}
fn space1<'a>(input: &mut Input<'a>) -> ModalResult<&'a str> {
    take_while(1.., char::is_whitespace).parse_next(input)
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
fn keyword<'a>(
    s: &'static str,
) -> impl Parser<Input<'a>, &'a str, winnow::error::ErrMode<ParseError>> {
    (
        s,
        peek(not(any.verify(|c| unicode_ident::is_xid_continue(*c)))),
    )
        .map(|(s, _)| s)
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
