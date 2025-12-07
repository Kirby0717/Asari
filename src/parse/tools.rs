use std::marker::PhantomData;

use super::error::*;
use winnow::{
    ModalResult, Parser,
    combinator::impls::TryMap,
    error::{ErrMode, FromExternalError},
    stream::{Location, Stream},
};

impl<I, O, E, P: Parser<I, O, E>> ParserExt<I, O, E> for P {}
pub trait ParserExt<I, O, E>: Parser<I, O, E> + Sized {
    fn try_map_with_span<O2, F>(self, map: F) -> TryMapWithSpan<Self, F, O>
    where
        F: FnMut(O) -> Result<O2, ParseErrorKind>,
    {
        TryMapWithSpan {
            parser: self,
            map,
            o: PhantomData,
        }
    }
}

pub struct TryMapWithSpan<P, F, O> {
    parser: P,
    map: F,
    o: PhantomData<O>,
}

impl<I, O, O2, E, P, F> Parser<I, O2, E> for TryMapWithSpan<P, F, O>
where
    I: Stream + Location,
    P: Parser<I, O, E>,
    F: FnMut(O) -> Result<O2, ParseErrorKind>,
    E: FromExternalError<I, ParseError>,
{
    fn parse_next(&mut self, input: &mut I) -> winnow::Result<O2, E> {
        let start = input.checkpoint();
        let begin = input.current_token_start();
        let output = self.parser.parse_next(input)?;
        let span = begin..input.previous_token_end();
        (self.map)(output).map_err(|kind| {
            input.reset(&start);
            E::from_external_error(input, ParseError { kind, span })
        })
    }
}
