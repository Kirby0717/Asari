use super::{Input, Span};

use std::fmt::Display;
use std::num::{ParseFloatError, ParseIntError};

use winnow::{
    error::{FromExternalError, ParserError},
    stream::Location,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ParseErrorKind {
    Literal(LiteralError),
    Number(NumberError),
    Ident(IdentError),
    Expr(ExprError),
    Type(TypeError),
    Command(CommandError),
    #[default]
    Other,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiteralError {
    UnclosedQuote,
    UnclosedDoubleQuote,
    UnclosedRawString,
    UnclosedPathString,
    UnrecognizedEscape(char),
    UnicodeEscape(UnicodeEscapeError),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnicodeEscapeError {
    NoOpenBrace,
    NoCloseBrace,
    InvalidCodePoint,
    InvalidHex(ParseIntError),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumberError {
    Bin(ParseIntError),
    Oct(ParseIntError),
    Dec(ParseIntError),
    Hex(ParseIntError),
    Float(ParseFloatError),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentError {
    Expected,
    Invalid,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprError {
    UnclosedParen,
    UnclosedBracket,
    UnclosedCommandSubst,
    NoRhs,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeError {
    UnclosedTypeParam,
    NoType,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
    NoCommand,
    InvalidPipe,
    InvalidRedirect,
    NoRedirectTarget,
    NoValue,
}
impl std::error::Error for ParseErrorKind {}
impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ParseErrorKind::*;
        match self {
            Literal(e) => e.fmt(f),
            Number(e) => e.fmt(f),
            Ident(e) => e.fmt(f),
            Expr(e) => e.fmt(f),
            Type(e) => e.fmt(f),
            Command(e) => e.fmt(f),
            Other => write!(f, "不明なエラーです"),
        }
    }
}
impl std::error::Error for LiteralError {}
impl Display for LiteralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use LiteralError::*;
        match self {
            UnclosedQuote => write!(f, "クォーテーションが閉じられていません"),
            UnclosedDoubleQuote => {
                write!(f, "ダブルクォーテーションが閉じられていません")
            }
            UnclosedRawString => write!(f, "raw文字列が閉じられていません"),
            UnclosedPathString => write!(f, "パス文字列が閉じられていません"),
            UnrecognizedEscape(c) => write!(f, "不明なエスケープ \\{c} です"),
            UnicodeEscape(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for UnicodeEscapeError {}
impl Display for UnicodeEscapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use UnicodeEscapeError::*;
        match self {
            NoOpenBrace => write!(f, "{{が必要です"),
            NoCloseBrace => write!(f, "}}が必要です"),
            InvalidCodePoint => write!(f, "不正なUnicodeです"),
            InvalidHex(e) => fmt_int_error(f, "16進数", e),
        }
    }
}
impl std::error::Error for NumberError {}
impl Display for NumberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use NumberError::*;
        match self {
            Bin(e) => fmt_int_error(f, "2進数", e),
            Oct(e) => fmt_int_error(f, "8進数", e),
            Dec(e) => fmt_int_error(f, "10進数", e),
            Hex(e) => fmt_int_error(f, "16進数", e),
            Float(_) => write!(f, "小数の解析に失敗しました"),
        }
    }
}
fn fmt_int_error(
    f: &mut std::fmt::Formatter<'_>,
    radix: &str,
    e: &std::num::ParseIntError,
) -> std::fmt::Result {
    use std::num::IntErrorKind::*;
    match e.kind() {
        Empty => write!(f, "数値が空です"),
        InvalidDigit => write!(f, "{radix}で書いてください"),
        NegOverflow => write!(f, "数値が小さすぎます"),
        PosOverflow => write!(f, "数値が大きすぎます"),
        _ => write!(f, "{radix}の解析に失敗しました"),
    }
}
impl std::error::Error for IdentError {}
impl Display for IdentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use IdentError::*;
        match self {
            Expected => write!(f, "名前がありません"),
            Invalid => write!(f, "不正な名前です"),
        }
    }
}
impl std::error::Error for ExprError {}
impl Display for ExprError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ExprError::*;
        match self {
            UnclosedParen => write!(f, "()が閉じられていません"),
            UnclosedBracket => write!(f, "[]が閉じられていません"),
            UnclosedCommandSubst => write!(f, "$()が閉じられていません"),
            NoRhs => write!(f, "演算子の右辺がありません"),
        }
    }
}
impl std::error::Error for TypeError {}
impl Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TypeError::*;
        match self {
            UnclosedTypeParam => write!(f, "type<>が閉じられていません"),
            NoType => write!(f, "型がありません"),
        }
    }
}
impl std::error::Error for CommandError {}
impl Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use CommandError::*;
        match self {
            NoCommand => write!(f, "コマンドがありません"),
            InvalidPipe => write!(f, "パイプは使えません"),
            InvalidRedirect => write!(f, "リダイレクトは使えません"),
            NoRedirectTarget => write!(f, "リダイレクト先が指定されていません"),
            NoValue => write!(f, "値がありません"),
        }
    }
}

impl FromExternalError<Input<'_>, ParseErrorKind> for ParseErrorKind {
    fn from_external_error(_input: &Input<'_>, e: ParseErrorKind) -> Self {
        e
    }
}
impl FromExternalError<Input<'_>, LiteralError> for ParseErrorKind {
    fn from_external_error(_input: &Input<'_>, e: LiteralError) -> Self {
        ParseErrorKind::Literal(e)
    }
}
impl FromExternalError<Input<'_>, UnicodeEscapeError> for ParseErrorKind {
    fn from_external_error(_input: &Input<'_>, e: UnicodeEscapeError) -> Self {
        ParseErrorKind::Literal(LiteralError::UnicodeEscape(e))
    }
}
impl FromExternalError<Input<'_>, NumberError> for ParseErrorKind {
    fn from_external_error(_input: &Input<'_>, e: NumberError) -> Self {
        ParseErrorKind::Number(e)
    }
}
impl FromExternalError<Input<'_>, IdentError> for ParseErrorKind {
    fn from_external_error(_input: &Input<'_>, e: IdentError) -> Self {
        ParseErrorKind::Ident(e)
    }
}
impl FromExternalError<Input<'_>, ExprError> for ParseErrorKind {
    fn from_external_error(_input: &Input<'_>, e: ExprError) -> Self {
        ParseErrorKind::Expr(e)
    }
}
impl FromExternalError<Input<'_>, TypeError> for ParseErrorKind {
    fn from_external_error(_input: &Input<'_>, e: TypeError) -> Self {
        ParseErrorKind::Type(e)
    }
}
impl FromExternalError<Input<'_>, CommandError> for ParseErrorKind {
    fn from_external_error(_input: &Input<'_>, e: CommandError) -> Self {
        ParseErrorKind::Command(e)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
    //pub context: Option<String>,
}
impl ParseError {
    pub fn display(&self, input: &str) -> String {
        let mut display = String::new();
        display += &format!("{}\n", input.replace(['\n', '\r'], " "));
        //display += &format!("{}^ {}\n", " ".repeat(self.span), self.kind);
        display += &format!(
            "{}{} {}\n",
            " ".repeat(self.span.start),
            "^".repeat(self.span.len()),
            self.kind
        );
        display
    }
}
impl ParserError<Input<'_>> for ParseError {
    type Inner = Self;
    fn from_input(input: &Input) -> Self {
        let pos = input.current_token_start();
        ParseError {
            //span: pos,
            span: pos..pos + 1,
            kind: Default::default(),
        }
    }
    fn into_inner(self) -> winnow::Result<Self::Inner, Self> {
        Ok(self)
    }
}
impl FromExternalError<Input<'_>, ParseError> for ParseError {
    fn from_external_error(_input: &Input<'_>, e: ParseError) -> Self {
        e
    }
}
