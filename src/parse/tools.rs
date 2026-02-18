mod impls;

use crate::parse::error::*;
use std::marker::PhantomData;
use winnow::{
    Parser,
    error::{ErrMode, FromExternalError, ModalError, ParserError},
    stream::{Location, Stream},
};

impl<I, O, E, P: Parser<I, O, E>> ParserExt<I, O, E> for P {}
pub trait ParserExt<I, O, E>: Parser<I, O, E> {
    #[inline(always)]
    fn cut(self) -> impls::Cut<Self, I, O, E>
    where
        Self: Sized,
        I: Stream,
        E: ParserError<I> + ModalError,
    {
        impls::Cut {
            parser: self,
            i: PhantomData,
            o: PhantomData,
            e: PhantomData,
        }
    }
    #[inline(always)]
    fn spanned(self) -> impls::Spanned<Self, I, O, E>
    where
        Self: Sized,
        I: Stream + Location,
    {
        impls::Spanned {
            parser: self,
            i: PhantomData,
            o: PhantomData,
            e: PhantomData,
        }
    }
}
impl<I, O, E, P: Parser<I, O, ErrMode<E>>> ParserModalExt<I, O, E> for P {}
pub trait ParserModalExt<I, O, E>: Parser<I, O, ErrMode<E>> {
    #[inline(always)]
    fn map_err_with_span<G>(
        self,
        map: G,
    ) -> impls::MapErrWithSpan<Self, G, I, O, E>
    where
        G: FnMut(E) -> ParseErrorKind,
        Self: core::marker::Sized,
        I: Location,
    {
        impls::MapErrWithSpan {
            parser: self,
            map,
            i: Default::default(),
            o: Default::default(),
            e: Default::default(),
        }
    }
}

impl<I, O, P: Parser<I, O, ErrMode<ParseError>>> ParserSpanExt<I, O> for P {}
pub trait ParserSpanExt<I, O>: Parser<I, O, ErrMode<ParseError>> {
    #[inline(always)]
    fn try_map_with_span<G, O2, E2>(
        self,
        map: G,
    ) -> impls::TryMapWithSpan<Self, G, I, O, O2, E2>
    where
        Self: Sized,
        G: FnMut(O) -> Result<O2, E2>,
        I: Stream + Location,
        ParseErrorKind: FromExternalError<I, E2>,
    {
        impls::TryMapWithSpan {
            parser: self,
            map,
            i: PhantomData,
            o: PhantomData,
            o2: PhantomData,
            e2: PhantomData,
        }
    }
}
