use chumsky::prelude::*;
use chumsky::text;
use crate::ErrTy;
use lumi_ast::{Expr, Span};

pub(crate) fn int_lit<'a>() -> impl Parser<'a, &'a str, Expr, ErrTy<'a>> {
    text::int(10)
        .from_str::<i64>()
        .unwrapped()
        .map_with(|n, e| {
            let sp: chumsky::span::SimpleSpan<usize> = e.span();
            Expr::Int(n, Span { start: sp.start, end: sp.end })
        })
        .padded()
        .labelled("int literal")
}

pub(crate) fn bool_lit<'a>() -> impl Parser<'a, &'a str, Expr, ErrTy<'a>> {
    choice((
        just("true").padded().map_with(|_, e| { let sp: chumsky::span::SimpleSpan<usize> = e.span(); Expr::Bool(true,  Span { start: sp.start, end: sp.end }) }),
        just("false").padded().map_with(|_, e| { let sp: chumsky::span::SimpleSpan<usize> = e.span(); Expr::Bool(false, Span { start: sp.start, end: sp.end }) }),
    ))
    .labelled("bool literal")
}

pub(crate) fn str_lit<'a>() -> impl Parser<'a, &'a str, Expr, ErrTy<'a>> {
    // Simple string literal with escapes; allows multi-line until closing quote
    let escape = just('\\').ignore_then(choice((
        just('\\').to('\\'),
        just('"').to('"'),
        just('n').to('\n'),
        just('t').to('\t'),
        just('r').to('\r'),
        just('0').to('\0'),
    )));
    let normal = any().filter(|c: &char| *c != '"' && *c != '\\');
    just('"')
        .ignore_then(escape.or(normal).repeated().collect::<String>())
        .then_ignore(just('"'))
        .map_with(|s, e| {
            let sp: chumsky::span::SimpleSpan<usize> = e.span();
            Expr::String(s, Span { start: sp.start, end: sp.end })
        })
        .padded()
        .labelled("string literal")
}
