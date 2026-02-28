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
/*impl<I, O, E, P: Parser<I, O, ErrMode<E>>> ParserModalExt<I, O, E> for P {}
pub trait ParserModalExt<I, O, E>: Parser<I, O, ErrMode<E>> {
}*/

impl<I, O, P: Parser<I, O, ErrMode<ParseError>>> ParserSpanExt<I, O> for P {}
pub trait ParserSpanExt<I, O>: Parser<I, O, ErrMode<ParseError>> {
    #[inline(always)]
    fn or_err_with_span<E2>(
        self,
        kind: E2,
    ) -> impls::OrErrWithSpan<Self, I, O, E2>
    where
        Self: Sized,
        I: Location,
        E2: Clone,
        ParseErrorKind: FromExternalError<I, E2>,
    {
        impls::OrErrWithSpan {
            parser: self,
            kind,
            i: PhantomData,
            o: PhantomData,
        }
    }
    fn or_err_at<E2>(
        self,
        kind: E2,
        span: Span,
    ) -> impls::OrErrAt<Self, I, O, E2>
    where
        Self: Sized,
        I: Location,
        E2: Clone,
        ParseErrorKind: FromExternalError<I, E2>,
    {
        impls::OrErrAt {
            parser: self,
            kind,
            span,
            i: PhantomData,
            o: PhantomData,
        }
    }
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
