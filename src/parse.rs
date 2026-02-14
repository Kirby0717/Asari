pub mod error;
pub mod tools;

use error::*;
use std::fmt::Display;
use tools::*;
#[allow(unused_imports)]
use winnow::combinator::todo as todo_parser;
use winnow::{
    LocatingSlice,
    combinator::{
        alt, delimited, dispatch, empty, fail, not, opt, peek, preceded,
        repeat, separated,
    },
    prelude::*,
    token::{any, rest, take_till, take_until, take_while},
};

use crate::parse::tools::ParserExt;

pub type Input<'i> = LocatingSlice<&'i str>;
type Span = std::ops::Range<usize>;

fn mix_span(a: &Span, b: &Span) -> Span {
    a.start.min(b.start)..a.end.max(b.end)
}

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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShellCommand {
    pub commands: Vec<(Command, Option<Pipe>)>,
    pub comment: Option<String>,
}
#[allow(unused)]
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
enum ExprPrefix {
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
        use ExprPrefix::*;
        Spanned {
            span: mix_span(&self.span, &expr.span),
            inner: match self.inner {
                Not => Expr::Not(Box::new(expr)),
                Neg => Expr::Neg(Box::new(expr)),
            },
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ExprInfix {
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
    #[rustfmt::skip]
    fn apply(
        self,
        l_expr: Spanned<Expr>,
        r_expr: Spanned<Expr>,
    ) -> Spanned<Expr> {
        use ExprInfix::*;
        Spanned {
            span: mix_span(&l_expr.span, &r_expr.span),
            inner: match self.inner {
                UnwrapOr     => Expr::UnwrapOr    (Box::new(l_expr), Box::new(r_expr)),
                Add          => Expr::Add         (Box::new(l_expr), Box::new(r_expr)),
                Sub          => Expr::Sub         (Box::new(l_expr), Box::new(r_expr)),
                Mul          => Expr::Mul         (Box::new(l_expr), Box::new(r_expr)),
                Div          => Expr::Div         (Box::new(l_expr), Box::new(r_expr)),
                Rem          => Expr::Rem         (Box::new(l_expr), Box::new(r_expr)),
                Equal        => Expr::Equal       (Box::new(l_expr), Box::new(r_expr)),
                NotEqual     => Expr::NotEqual    (Box::new(l_expr), Box::new(r_expr)),
                Less         => Expr::Less        (Box::new(l_expr), Box::new(r_expr)),
                LessEqual    => Expr::LessEqual   (Box::new(l_expr), Box::new(r_expr)),
                Greater      => Expr::Greater     (Box::new(l_expr), Box::new(r_expr)),
                GreaterEqual => Expr::GreaterEqual(Box::new(l_expr), Box::new(r_expr)),
                And          => Expr::And         (Box::new(l_expr), Box::new(r_expr)),
                Or           => Expr::Or          (Box::new(l_expr), Box::new(r_expr)),
            },
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ExprPostfix {
    Unwrap,               // !
    IsSome,               // ?
    Length,               // @
    Index(Spanned<Expr>), //[expr]
}
impl ExprPostfix {
    fn power(&self) -> i32 {
        use ExprPostfix::*;
        match self {
            Unwrap => 10,
            IsSome => 10,
            Length => 10,
            Index(..) => 10,
        }
    }
}
impl Spanned<ExprPostfix> {
    fn apply(self, expr: Spanned<Expr>) -> Spanned<Expr> {
        use ExprPostfix::*;
        Spanned {
            span: mix_span(&self.span, &expr.span),
            inner: match self.inner {
                Unwrap => Expr::Unwrap(Box::new(expr)),
                IsSome => Expr::IsSome(Box::new(expr)),
                Length => Expr::Length(Box::new(expr)),
                Index(index) => Expr::Index(Box::new(expr), Box::new(index)),
            },
        }
    }
}
type ExprNode = Box<Spanned<Expr>>;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Expr {
    Primary(Spanned<Primary>),        // primary
    Unwrap(ExprNode),                 // expr!           <- 10
    IsSome(ExprNode),                 // expr?           <- 10
    Length(ExprNode),                 // expr@           <- 10
    Index(ExprNode, ExprNode),        // expr1[expr2]    <- 10
    Not(ExprNode),                    // !expr              9  ->
    Neg(ExprNode),                    // -expr              9  ->
    Mul(ExprNode, ExprNode),          // expr1 * expr2   <- 7
    Div(ExprNode, ExprNode),          // expr1 / expr2   <- 7
    Rem(ExprNode, ExprNode),          // expr1 % expr2   <- 7
    UnwrapOr(ExprNode, ExprNode),     // expr1 ^ expr2      1  ->
    Add(ExprNode, ExprNode),          // expr1 + expr2   <- 6
    Sub(ExprNode, ExprNode),          // expr1 - expr2   <- 6
    Less(ExprNode, ExprNode),         // expr1 < expr2   <- 5
    LessEqual(ExprNode, ExprNode),    // expr1 <= expr2  <- 5
    Greater(ExprNode, ExprNode),      // expr1 > expr2   <- 5
    GreaterEqual(ExprNode, ExprNode), // expr1 >= expr2  <- 5
    Equal(ExprNode, ExprNode),        // expr1 == expr2  <- 4
    NotEqual(ExprNode, ExprNode),     // expr1 != expr2  <- 4
    And(ExprNode, ExprNode),          // expr1 && expr2  <- 3
    Or(ExprNode, ExprNode),           // expr1 || expr2  <- 2
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
// とりあえずStringで
pub enum Primary {
    String(String),                     // "abc", 'abc', r"abc"
    PathString(String),                 // p"abc"
    SpecialVar(SpecialVar),             // $?, $$, $!, $@
    EnvVar(String),                     // $abc
    ShellVar(String),                   // @abc
    Paren(Box<Spanned<Expr>>),          // (expr)
    Array(Vec<Spanned<Expr>>),          // [e1, e2, ... , ek]
    Bool(bool),                         // true, false
    Number(u64),                        // 123
    Option(Option<Box<Spanned<Expr>>>), // none, some(expr)
    Unit,                               // ()
}
impl Display for Primary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Primary::*;
        match self {
            String(literal) => write!(f, "{literal}"),
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
#[allow(unused)]
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
        '\'' => quoted_string.map(Primary::String),
        '"' => double_quoted_string.map(Primary::String),
        '$' => preceded('$', alt((
            special_var.map(Primary::SpecialVar),
            ident.map(Primary::EnvVar),
        ))),
        '@' => preceded('@', ident).map(Primary::ShellVar),
        '(' => alt((
            ('(', space0, ')').value(Primary::Unit),
            delimited(('(', space0), expr, (space0, ')'))
                .map(|expr| Primary::Paren(Box::new(expr)))
        )),
        'r' => raw_string.map(Primary::String),
        'p' => path_string.map(Primary::PathString),
        '[' => delimited(('[', space0), separated(0.., expr, delimited(space0, ',', space0)), ']')
            .map(|exprs| {
                Primary::Array(exprs)
            }),
        _ => fail,
    )
    .spanned()
    .parse_next(input)
}
fn simple_expr_postfix(input: &mut Input) -> SpannedResult<ExprPostfix> {
    use ExprPostfix::*;
    dispatch!(any;
        '!' => empty.value(Unwrap),
        '?' => empty.value(IsSome),
        '@' => empty.value(Length),
        '[' => delimited(space0, expr, (space0, ']')).map(Index),
        _ => fail,
    )
    .spanned()
    .parse_next(input)
}
fn simple_expr_infix(input: &mut Input) -> SpannedResult<ExprInfix> {
    use ExprInfix::*;
    dispatch!(any;
        '^' => empty.value(UnwrapOr),
        _ => fail,
    )
    .spanned()
    .parse_next(input)
}
pub fn simple_expr(input: &mut Input) -> SpannedResult<Expr> {
    // 値
    let primary = simple_expr_primary.parse_next(input)?;
    let mut lhs = Spanned {
        span: primary.span.clone(),
        inner: Expr::Primary(primary),
    };

    loop {
        // 後置演算子
        if let Some(postfix) = opt(simple_expr_postfix).parse_next(input)? {
            lhs = postfix.apply(lhs);
            continue;
        }

        // 中置演算子
        if let Some(infix) = opt(simple_expr_infix).parse_next(input)? {
            let rhs = simple_expr.parse_next(input)?;
            lhs = infix.apply(lhs, rhs);
            continue;
        }

        break;
    }

    Ok(lhs)
}
fn expr_primary(input: &mut Input) -> SpannedResult<Primary> {
    dispatch!(peek(any);
        '\'' => quoted_string.map(Primary::String),
        '"' => double_quoted_string.map(Primary::String),
        '$' => preceded('$', alt((
            special_var.map(Primary::SpecialVar),
            ident.map(Primary::EnvVar),
        ))),
        '@' => preceded('@', ident).map(Primary::ShellVar),
        '(' => alt((
            ('(', space0, ')').value(Primary::Unit),
            delimited(('(', space0), expr, (space0, ')'))
                .map(|expr| Primary::Paren(Box::new(expr)))
        )),
        'r' => raw_string.map(Primary::String),
        'p' => path_string.map(Primary::PathString),
        '[' => delimited(('[', space0), separated(0.., expr, delimited(space0, ',', space0)), ']')
            .map(|exprs| {
                Primary::Array(exprs)
            }),
        't' => "true".value(Primary::Bool(true)),
        'f' => "false".value(Primary::Bool(false)),
        // 簡易整数
        '0'..='9' => winnow::ascii::dec_uint.map(Primary::Number),
        'n' => "none".value(Primary::Option(None)),
        's' => delimited(("some", space0, '(', space0), expr, (space0, ')'))
            .map(|expr| Primary::Option(Some(Box::new(expr)))),
        _ => fail,
    )
    .spanned()
    .parse_next(input)
}
fn expr_prefix(input: &mut Input) -> SpannedResult<ExprPrefix> {
    use ExprPrefix::*;
    dispatch! {any;
        '!' => empty.value(Not),
        '-' => empty.value(Neg),
        _ => fail,
    }
    .spanned()
    .parse_next(input)
}
fn expr_infix(input: &mut Input) -> SpannedResult<ExprInfix> {
    use ExprInfix::*;
    dispatch! {any;
        '^' => empty.value(UnwrapOr),
        '+' => empty.value(Add),
        '-' => empty.value(Sub),
        '*' => empty.value(Mul),
        '/' => empty.value(Div),
        '%' => empty.value(Rem),
        '=' => '='.value(Equal),
        '!' => '='.value(NotEqual),
        '<' => opt('='.value(LessEqual))
                .map(|c| c.unwrap_or(Less)),
        '>' => opt('='.value(GreaterEqual))
                .map(|c| c.unwrap_or(Greater)),
        '&' => '&'.value(And),
        '|' => '|'.value(Or),
        _ => fail,
    }
    .spanned()
    .parse_next(input)
}
fn expr_postfix(input: &mut Input) -> SpannedResult<ExprPostfix> {
    use ExprPostfix::*;
    dispatch! {any;
        '!' => empty.value(Unwrap),
        '?' => empty.value(IsSome),
        '@' => empty.value(Length),
        '[' => delimited(space0, expr, (space0, ']')).map(Index),
        _ => fail,
    }
    .spanned()
    .parse_next(input)
}
fn expr(input: &mut Input) -> SpannedResult<Expr> {
    expr_pratt(input, 0)
}
fn expr_pratt(input: &mut Input, min_power: i32) -> SpannedResult<Expr> {
    // 前置演算子 or 値
    let mut lhs = if let Some(prefix) = opt(expr_prefix).parse_next(input)? {
        let _ = space0.parse_next(input)?;
        let rhs = expr_pratt(input, prefix.inner.power())?;
        prefix.apply(rhs)
    }
    else {
        let primary = expr_primary.parse_next(input)?;
        Spanned {
            span: primary.span.clone(),
            inner: Expr::Primary(primary),
        }
    };

    loop {
        let _ = space0.parse_next(input)?;
        let checkpoint = input.checkpoint();

        // 中置演算子
        if let Some(infix) = opt(expr_infix).parse_next(input)? {
            let (l_power, r_power) = infix.inner.power();
            if l_power < min_power {
                input.reset(&checkpoint);
                break;
            }
            let _ = space0.parse_next(input)?;
            let rhs = expr_pratt(input, r_power)?;
            lhs = infix.apply(lhs, rhs);
            continue;
        }

        // 後置演算子
        if let Some(postfix) = opt(expr_postfix).parse_next(input)? {
            let power = postfix.inner.power();
            if power < min_power {
                input.reset(&checkpoint);
                break;
            }
            lhs = postfix.apply(lhs);
            continue;
        }

        break;
    }

    Ok(lhs)
}

fn primary(input: &mut Input) -> SpannedResult<Primary> {
    dispatch!(peek(any);
        '\'' => quoted_string.map(Primary::String),
        '"' => double_quoted_string.map(Primary::String),
        '$' => preceded('$', alt((
            special_var.map(Primary::SpecialVar),
            ident.map(Primary::EnvVar),
        ))),
        '@' => preceded('@', ident).map(Primary::ShellVar),
        '(' => "()".value(Primary::Unit),
        _ => alt((
            raw_string.map(Primary::String),
            path_string.map(Primary::PathString),
            unquoted_string.map(Primary::String),
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
