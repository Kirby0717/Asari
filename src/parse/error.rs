use std::fmt::Display;

use winnow::{
    error::{AddContext, FromExternalError, ParserError},
    stream::Location,
};

use super::{Input, Span};

#[derive(Clone, Debug, Default)]
pub enum ParseErrorKind {
    InvalidUnicode,
    NotHex,
    #[default]
    Other,
}
impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ParseErrorKind::*;
        match self {
            InvalidUnicode => write!(f, "不正なUnicodeです"),
            NotHex => write!(f, "16進数で書いてください"),
            _ => todo!(),
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
    //pub context: Option<String>,
}
impl ParseError {
    pub fn display(&self, input: &Input) -> String {
        let mut display = String::new();
        display += &format!("{}\n", input.replace(['\n', '\r'], " "));
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
            span: pos..pos,
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
