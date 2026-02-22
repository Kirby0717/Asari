use std::fmt::Display;

use winnow::{
    error::{FromExternalError, ParserError},
    stream::Location,
};

use super::Input;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ParseErrorKind {
    ParseBinError(std::num::ParseIntError),
    ParseOctError(std::num::ParseIntError),
    ParseDecError(std::num::ParseIntError),
    ParseHexError(std::num::ParseIntError),
    ParseFloatError(std::num::ParseFloatError),
    NoIdent,
    InvalidIdent,
    InvalidUnicodeEscape(UnicodeEscapeError),
    UnrecognizedEscape(char),
    NoEndQuotation,
    NoEndDoubleQuotation,
    #[default]
    Other,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnicodeEscapeError {
    NoBeginBrace,
    InvalidUnicode,
    NoEndBrace,
}
impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ParseErrorKind::*;
        match self {
            ParseBinError(e) => {
                use std::num::IntErrorKind::*;
                match *e.kind() {
                    Empty => write!(f, "数値が空です"),
                    InvalidDigit => write!(f, "2進数で書いてください"),
                    NegOverflow => write!(f, "数値が小さすぎます"),
                    PosOverflow => write!(f, "数値が大きすぎます"),
                    _ => write!(f, "2進数の解析に失敗しました"),
                }
            }
            ParseOctError(e) => {
                use std::num::IntErrorKind::*;
                match *e.kind() {
                    Empty => write!(f, "数値が空です"),
                    InvalidDigit => write!(f, "8進数で書いてください"),
                    NegOverflow => write!(f, "数値が小さすぎます"),
                    PosOverflow => write!(f, "数値が大きすぎます"),
                    _ => write!(f, "8進数の解析に失敗しました"),
                }
            }
            ParseDecError(e) => {
                use std::num::IntErrorKind::*;
                match *e.kind() {
                    Empty => write!(f, "数値が空です"),
                    InvalidDigit => write!(f, "10進数で書いてください"),
                    NegOverflow => write!(f, "数値が小さすぎます"),
                    PosOverflow => write!(f, "数値が大きすぎます"),
                    _ => write!(f, "10進数の解析に失敗しました"),
                }
            }
            ParseHexError(e) => {
                use std::num::IntErrorKind::*;
                match *e.kind() {
                    Empty => write!(f, "数値が空です"),
                    InvalidDigit => write!(f, "16進数で書いてください"),
                    NegOverflow => write!(f, "数値が小さすぎます"),
                    PosOverflow => write!(f, "数値が大きすぎます"),
                    _ => write!(f, "16進数の解析に失敗しました"),
                }
            }
            ParseFloatError(_) => write!(f, "小数の解析に失敗しました"),
            InvalidUnicodeEscape(e) => {
                use UnicodeEscapeError::*;
                match e {
                    NoBeginBrace => write!(f, "{{が必要です"),
                    InvalidUnicode => write!(f, "不正なUnicodeです"),
                    NoEndBrace => write!(f, "}}が必要です"),
                }
            }
            NoIdent => write!(f, "名前がありません"),
            InvalidIdent => write!(f, "不正な名前です"),
            UnrecognizedEscape(c) => write!(f, "不明なエスケープ \\{c} です"),
            NoEndQuotation => write!(f, "クォーテーションを閉じてください"),
            NoEndDoubleQuotation => {
                write!(f, "ダブルクォーテーションを閉じてください")
            }
            Other => write!(f, "不明なエラーです"),
        }
    }
}
impl FromExternalError<Input<'_>, ParseErrorKind> for ParseErrorKind {
    fn from_external_error(_input: &Input<'_>, e: ParseErrorKind) -> Self {
        e
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
    fn from_external_error(_input: &Input<'_>, e: ParseError) -> Self {
        e
    }
}
