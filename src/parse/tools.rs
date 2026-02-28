mod impls;

use super::Span;
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
    #[inline(always)]
    fn reject_with_span<G, E2>(
        self,
        map: G,
    ) -> impls::RejectWithSpan<Self, G, I, O, E, E2>
    where
        Self: Sized,
        G: FnMut(O) -> E2,
        I: Stream + Location,
        ParseErrorKind: FromExternalError<I, E2>,
    {
        impls::RejectWithSpan {
            parser: self,
            map,
            i: PhantomData,
            o: PhantomData,
            e: PhantomData,
            e2: PhantomData,
        }
    }
}
impl<I, O, E, P: Parser<I, O, ErrMode<E>>> ParserModalExt<I, O, E> for P {}
pub trait ParserModalExt<I, O, E>: Parser<I, O, ErrMode<E>> {
    #[inline(always)]
    fn map_err_with_span<G, E2>(
        self,
        map: G,
    ) -> impls::MapErrWithSpan<Self, G, I, O, E, E2>
    where
        Self: Sized,
        G: FnMut(E) -> E2,
        I: Location,
    {
        impls::MapErrWithSpan {
            parser: self,
            map,
            i: PhantomData,
            o: PhantomData,
            e: PhantomData,
            e2: PhantomData,
        }
    }
    fn map_err_at<G, E2>(
        self,
        map: G,
        span: Span,
    ) -> impls::MapErrAt<Self, G, I, O, E, E2>
    where
        Self: Sized,
        G: FnMut(E) -> E2,
        I: Location,
    {
        impls::MapErrAt {
            parser: self,
            map,
            span,
            i: PhantomData,
            o: PhantomData,
            e: PhantomData,
            e2: PhantomData,
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
