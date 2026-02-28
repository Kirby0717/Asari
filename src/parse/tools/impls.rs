use super::Span;
use crate::parse::error::*;

use std::marker::PhantomData;

use winnow::{
    ModalResult, Parser,
    error::{ErrMode, FromExternalError, ModalError, ParserError},
    stream::{Location, Stream},
};

pub struct Cut<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream,
    E: ParserError<I> + ModalError,
{
    pub(super) parser: F,
    pub(super) i: PhantomData<I>,
    pub(super) o: PhantomData<O>,
    pub(super) e: PhantomData<E>,
}
impl<F, I, O, E> Parser<I, O, E> for Cut<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream,
    E: ParserError<I> + ModalError,
{
    fn parse_next(&mut self, input: &mut I) -> winnow::Result<O, E> {
        self.parser.parse_next(input).map_err(|e| e.cut())
    }
}

pub struct Spanned<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream + Location,
{
    pub(super) parser: F,
    pub(super) i: PhantomData<I>,
    pub(super) o: PhantomData<O>,
    pub(super) e: PhantomData<E>,
}
impl<F, I, O, E> Parser<I, crate::parse::Spanned<O>, E> for Spanned<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream + Location,
{
    fn parse_next(
        &mut self,
        input: &mut I,
    ) -> winnow::Result<crate::parse::Spanned<O>, E> {
        let start = input.current_token_start();
        self.parser.parse_next(input).map(move |output| {
            let end = input.previous_token_end();
            crate::parse::Spanned {
                inner: output,
                span: start..end,
            }
        })
    }
}

pub struct RejectWithSpan<F, G, I, O, E, E2>
where
    F: Parser<I, O, E>,
    G: FnMut(O) -> E2,
    I: Stream + Location,
    ParseErrorKind: FromExternalError<I, E2>,
{
    pub(super) parser: F,
    pub(super) map: G,
    pub(super) i: PhantomData<I>,
    pub(super) o: PhantomData<O>,
    pub(super) e: PhantomData<E>,
    pub(super) e2: PhantomData<E2>,
}
impl<F, G, I, O, E, E2> Parser<I, E, ErrMode<ParseError>>
    for RejectWithSpan<F, G, I, O, E, E2>
where
    F: Parser<I, O, E>,
    G: FnMut(O) -> E2,
    I: Stream + Location,
    ParseErrorKind: FromExternalError<I, E2>,
{
    fn parse_next(&mut self, input: &mut I) -> ModalResult<E, ParseError> {
        let begin = input.current_token_start();
        match self.parser.parse_next(input) {
            Ok(output) => {
                let end = input.previous_token_end();
                Err(ErrMode::Cut(ParseError {
                    kind: ParseErrorKind::from_external_error(
                        input,
                        (self.map)(output),
                    ),
                    span: begin..end,
                }))
            }
            Err(e) => Ok(e),
        }
    }
}

pub struct MapErrWithSpan<F, G, I, O, E, E2>
where
    F: Parser<I, O, ErrMode<E>>,
    G: FnMut(E) -> E2,
    I: Location,
{
    pub(super) parser: F,
    pub(super) map: G,
    pub(super) i: PhantomData<I>,
    pub(super) o: PhantomData<O>,
    pub(super) e: PhantomData<E>,
    pub(super) e2: PhantomData<E2>,
}
impl<F, G, I, O, E, E2> Parser<I, O, ErrMode<ParseError>>
    for MapErrWithSpan<F, G, I, O, E, E2>
where
    F: Parser<I, O, ErrMode<E>>,
    G: FnMut(E) -> E2,
    I: Location,
    ParseErrorKind: FromExternalError<I, E2>,
{
    #[inline]
    fn parse_next(&mut self, input: &mut I) -> ModalResult<O, ParseError> {
        let begin = input.current_token_start();
        self.parser.parse_next(input).map_err(|e| {
            e.map(|e| ParseError {
                kind: ParseErrorKind::from_external_error(input, (self.map)(e)),
                span: begin..begin + 1,
            })
        })
    }
}

pub struct MapErrAt<F, G, I, O, E, E2>
where
    F: Parser<I, O, ErrMode<E>>,
    G: FnMut(E) -> E2,
    I: Location,
{
    pub(super) parser: F,
    pub(super) map: G,
    pub(super) span: Span,
    pub(super) i: PhantomData<I>,
    pub(super) o: PhantomData<O>,
    pub(super) e: PhantomData<E>,
    pub(super) e2: PhantomData<E2>,
}
impl<F, G, I, O, E, E2> Parser<I, O, ErrMode<ParseError>>
    for MapErrAt<F, G, I, O, E, E2>
where
    F: Parser<I, O, ErrMode<E>>,
    G: FnMut(E) -> E2,
    I: Location,
    ParseErrorKind: FromExternalError<I, E2>,
{
    #[inline]
    fn parse_next(&mut self, input: &mut I) -> ModalResult<O, ParseError> {
        self.parser.parse_next(input).map_err(|e| {
            e.map(|e| ParseError {
                kind: ParseErrorKind::from_external_error(input, (self.map)(e)),
                span: self.span.clone(),
            })
        })
    }
}

pub struct TryMapWithSpan<F, G, I, O, O2, E2>
where
    F: Parser<I, O, ErrMode<ParseError>>,
    G: FnMut(O) -> Result<O2, E2>,
    I: Stream + Location,
    ParseErrorKind: FromExternalError<I, E2>,
{
    pub(super) parser: F,
    pub(super) map: G,
    pub(super) i: PhantomData<I>,
    pub(super) o: PhantomData<O>,
    pub(super) o2: PhantomData<O2>,
    pub(super) e2: PhantomData<E2>,
}
impl<F, G, I, O, O2, E2> Parser<I, O2, ErrMode<ParseError>>
    for TryMapWithSpan<F, G, I, O, O2, E2>
where
    F: Parser<I, O, ErrMode<ParseError>>,
    G: FnMut(O) -> Result<O2, E2>,
    I: Stream + Location,
    ParseErrorKind: FromExternalError<I, E2>,
{
    fn parse_next(&mut self, input: &mut I) -> ModalResult<O2, ParseError> {
        //let start = input.checkpoint();
        let begin = input.current_token_start();
        let output = self.parser.parse_next(input)?;
        let end = input.previous_token_end();
        (self.map)(output).map_err(|err| {
            //input.reset(&start);
            ErrMode::Backtrack(ParseError {
                span: begin..end,
                kind: ParseErrorKind::from_external_error(input, err),
            })
        })
    }
}
