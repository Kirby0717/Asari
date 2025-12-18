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
    pub(crate) parser: F,
    pub(crate) i: PhantomData<I>,
    pub(crate) o: PhantomData<O>,
    pub(crate) e: PhantomData<E>,
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
    pub(crate) parser: F,
    pub(crate) i: PhantomData<I>,
    pub(crate) o: PhantomData<O>,
    pub(crate) e: PhantomData<E>,
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

pub struct MapErrWithSpan<F, G, I, O, E>
where
    F: Parser<I, O, ErrMode<E>>,
    G: FnMut(E) -> ParseErrorKind,
    I: Location,
{
    pub(crate) parser: F,
    pub(crate) map: G,
    pub(crate) i: core::marker::PhantomData<I>,
    pub(crate) o: core::marker::PhantomData<O>,
    pub(crate) e: core::marker::PhantomData<E>,
}
impl<F, G, I, O, E> Parser<I, O, ErrMode<ParseError>>
    for MapErrWithSpan<F, G, I, O, E>
where
    F: Parser<I, O, ErrMode<E>>,
    G: FnMut(E) -> ParseErrorKind,
    I: Location,
{
    #[inline]
    fn parse_next(&mut self, input: &mut I) -> ModalResult<O, ParseError> {
        let begin = input.current_token_start();
        //let span = begin..input.previous_token_end();
        self.parser.parse_next(input).map_err(|e| {
            e.map(|e| ParseError {
                kind: (self.map)(e),
                span: begin,
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
    pub(crate) parser: F,
    pub(crate) map: G,
    pub(crate) i: PhantomData<I>,
    pub(crate) o: PhantomData<O>,
    pub(crate) o2: PhantomData<O2>,
    pub(crate) e2: PhantomData<E2>,
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
        //let span = begin..input.previous_token_end();
        (self.map)(output).map_err(|err| {
            //input.reset(&start);
            ErrMode::Backtrack(ParseError {
                span: begin,
                kind: ParseErrorKind::from_external_error(input, err),
            })
        })
    }
}
