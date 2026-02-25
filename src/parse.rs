pub mod command;
pub mod error;
pub mod expr;
pub mod literal;
pub mod simple_expr;
pub mod tools;

use crate::shell_command::ShellCommandKind;
use crate::value::Type;
use command::command_line;
use error::*;
use expr::expr;
use literal::*;
use simple_expr::simple_expr;
use tools::*;

use serde::{Deserialize, Serialize};
use winnow::{
    LocatingSlice,
    combinator::{
        alt, delimited, dispatch, empty, fail, not, opt, peek, preceded,
        repeat, separated, terminated, trace,
    },
    prelude::*,
    token::{any, rest, take, take_till, take_until, take_while},
};

pub type Input<'i> = LocatingSlice<&'i str>;
type Span = std::ops::Range<usize>;

fn mix_span(a: &Span, b: &Span) -> Span {
    a.start.min(b.start)..a.end.max(b.end)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Spanned<T> {
    pub inner: T,
    #[serde(skip)]
    pub span: Span,
}
impl<T> Spanned<T> {
    pub fn _map<U, F: FnOnce(T) -> U>(self, f: F) -> Spanned<U> {
        Spanned {
            inner: f(self.inner),
            span: self.span,
        }
    }
}
impl<T> AsRef<T> for Spanned<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}
impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
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

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CommandLine {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub statements: Vec<Spanned<Statement>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<Spanned<String>>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Statement {
    ShellCommand(Spanned<ShellCommand>),
    Pipeline(Spanned<Pipeline>),
    EnvAssign(Spanned<EnvAssign>),
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ShellCommand {
    pub kind: Spanned<ShellCommandKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<Spanned<CommandPart>>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct EnvAssign {
    pub name: Spanned<String>,
    pub value: Spanned<Expr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Pipeline {
    pub first: Spanned<Command>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rest: Vec<(Spanned<Pipe>, Spanned<Command>)>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Pipe {
    Stdout,       // |   stdoutのみ
    StdoutStderr, // |&  stdout + stderr
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Command {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub temp_envs: Vec<Spanned<(Spanned<String>, Spanned<Expr>)>>,
    pub name: Spanned<CommandPart>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<Spanned<CommandPart>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub redirects: Vec<Spanned<Redirect>>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Redirect {
    Input(InputRedirect),
    Output(((OutputRedirect, OutputMode), Spanned<CommandPart>)),
    Merge(MergeRedirect),
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum InputRedirect {
    File(Spanned<CommandPart>),       // <    file
    HereDoc(Spanned<String>),         // <<   EOF ... EOF
    HereString(Spanned<CommandPart>), // <<< "string"
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum OutputRedirect {
    Stdout, // >  >>
    Stderr, // 2> 2>>
    Both,   // &> &>>
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum OutputMode {
    Truncate, // >  2>  &>
    Append,   // >> 2>> &>>
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum MergeRedirect {
    StderrToStdout, // 2>&1
    StdoutToStderr, // 1>&2
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum CommandPart {
    Unquoted(Spanned<String>),
    SimpleExpr(Spanned<Expr>),
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Expr {
    Primary(Spanned<Primary>),
    Prefix(ExprNode, ExprPrefix),
    Infix(ExprNode, ExprNode, ExprInfix),
    Postfix(ExprNode, ExprPostfix),
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Primary {
    String(String),                          // "abc", 'abc', r"abc"
    PathString(String),                      // p"abc"
    SpecialVar(SpecialVar),                  // $?, $$, $!, $@
    EnvVar(String),                          // $abc
    ShellVar(String),                        // @abc
    Paren(Box<Spanned<Expr>>),               // (expr)
    CommandSubst(Box<Spanned<CommandLine>>), // $(command args...)
    Array(Vec<Spanned<Expr>>),               // [e1, e2, ... , ek]
    Bool(bool),                              // true, false
    Int(i64),                                // 123
    Float(f64),                              // 12.3
    Option(Option<Box<Spanned<Expr>>>),      // none, some(expr)
    Unit,                                    // ()
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
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
    Spanned<CommandLine>,
    winnow::error::ParseError<LocatingSlice<&str>, ParseError>,
> {
    delimited(space0, command_line, space0).parse(Input::new(input))
}

fn space0<'a>(input: &mut Input<'a>) -> ModalResult<&'a str> {
    trace("space0", take_while(0.., char::is_whitespace)).parse_next(input)
}
fn space1<'a>(input: &mut Input<'a>) -> ModalResult<&'a str> {
    trace("space1", take_while(1.., char::is_whitespace)).parse_next(input)
}
fn ident(input: &mut Input) -> ModalResult<String> {
    use unicode_ident::*;
    trace(
        "ident",
        (
            any.map_err_with_span(|()| IdentError::Expected)
                .try_map_with_span(|c| {
                    if c == '_' || is_xid_start(c) {
                        Ok(c)
                    }
                    else {
                        Err(IdentError::Expected)
                    }
                }),
            take_while(0.., is_xid_continue),
        ),
    )
    .try_map_with_span(|(ident_start, ident_continue)| {
        if ident_start == '_' && ident_continue.is_empty() {
            Err(IdentError::Invalid)
        }
        else {
            Ok(String::from(ident_start) + ident_continue)
        }
    })
    .parse_next(input)
}
fn env_var(input: &mut Input) -> ModalResult<String> {
    trace("env_var", |input: &mut Input| {
        let _ = '$'.parse_next(input)?;
        let name = ident.cut().parse_next(input)?;
        Ok(name)
    })
    .parse_next(input)
}
fn shell_var(input: &mut Input) -> ModalResult<String> {
    trace("env_var", |input: &mut Input| {
        let _ = '@'.parse_next(input)?;
        let name = ident.cut().parse_next(input)?;
        Ok(name)
    })
    .parse_next(input)
}

fn keyword<'a>(
    s: &'static str,
) -> impl Parser<Input<'a>, &'a str, winnow::error::ErrMode<ParseError>> {
    trace(
        "keyword",
        (
            s,
            peek(not(any.verify(|c| unicode_ident::is_xid_continue(*c)))),
        ),
    )
    .map(|(s, _)| s)
}
fn special_var(input: &mut Input) -> ModalResult<SpecialVar> {
    trace(
        "special_var",
        dispatch! {any;
            '?' => empty.value(SpecialVar::ExitStatus),
            '$' => empty.value(SpecialVar::Pid),
            '!' => empty.value(SpecialVar::BackgroundPid),
            '@' => empty.value(SpecialVar::ShellName),
            _ => fail,
        },
    )
    .parse_next(input)
}
