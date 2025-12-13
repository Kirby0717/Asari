use std::fmt::Display;

use winnow::{
    error::{AddContext, FromExternalError, ParserError},
    stream::Location,
};

use super::{Input, Span};

#[derive(Clone, Debug, Default)]
pub enum ParseErrorKind {
    ParseHexError(std::num::ParseIntError),
    InvalidUnicode,
    InvalidIdent,
    UnrecognizedEscape(char),
    NoEndQuotation,
    NoEndDoubleQuotation,
    #[default]
    Other,
}
impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ParseErrorKind::*;
        match self {
            ParseHexError(e) => {
                use std::num::IntErrorKind::*;
                match *e.kind() {
                    Empty => write!(f, "空です"),
                    InvalidDigit => write!(f, "16進数で書いてください"),
                    NegOverflow => write!(f, "小さすぎます"),
                    PosOverflow => write!(f, "大きすぎます"),
                    _ => write!(f, "16進数の解析に失敗しました"),
                }
            }
            InvalidUnicode => write!(f, "不正なUnicodeです"),
            InvalidIdent => write!(f, "不正な名前です"),
            UnrecognizedEscape(c) => write!(f, "不明なエスケープ \\{c} です"),
            NoEndQuotation => write!(f, "クォーテーションを閉じてください"),
            NoEndDoubleQuotation => write!(f, "ダブルクォーテーションを閉じてください"),
            e => write!(f, "不明なエラーです"),
        }
    }
}
impl FromExternalError<Input<'_>, ParseErrorKind> for ParseErrorKind {
    fn from_external_error(input: &Input<'_>, e: ParseErrorKind) -> Self {
        e
    }
}

#[derive(Clone, Debug, Default)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    // 範囲はむずいからとりあえず位置で
    pub span: usize,
    //pub span: Span,
    //pub context: Option<String>,
}
impl ParseError {
    pub fn display(&self, input: &Input) -> String {
        let source = **input;
        let mut display = String::new();
        display += &format!("{}\n", source.replace(['\n', '\r'], " "));
        display += &format!("{}^ {}\n", " ".repeat(self.span), self.kind);
        /*display += &format!(
            "{}{} {}\n",
            " ".repeat(self.span.start),
            "^".repeat(self.span.len()),
            self.kind
        );*/
        display
    }
}
impl ParserError<Input<'_>> for ParseError {
    type Inner = Self;
    fn from_input(input: &Input) -> Self {
        let pos = input.current_token_start();
        ParseError {
            span: pos,
            //span: pos..pos,
            kind: Default::default(),
        }
    }
    fn into_inner(self) -> winnow::Result<Self::Inner, Self> {
        Ok(self)
    }
}
impl FromExternalError<Input<'_>, ParseError> for ParseError {
    fn from_external_error(input: &Input<'_>, e: ParseError) -> Self {
        e
    }
}
