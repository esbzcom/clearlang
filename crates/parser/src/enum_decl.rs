use crate::generics::type_params_p;
use crate::tokens::{ident_p, kw};
use crate::types::ty_p;
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{EnumDecl, EnumVariant, Span};

fn to_span(sp: chumsky::span::SimpleSpan<usize>) -> Span {
    Span {
        start: sp.start,
        end: sp.end,
    }
}

fn variant_p<'a>() -> impl Parser<'a, &'a str, EnumVariant, ErrTy<'a>> {
    let tuple_fields = ty_p()
        .separated_by(just(',').padded().labelled("comma"))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('(').padded(), just(')').padded());

    ident_p()
        .map_with(|name, e| (name, to_span(e.span())))
        .then(tuple_fields.or_not())
        .map_with(|((name, name_span), fields), e| EnumVariant {
            name,
            name_span,
            fields: fields.unwrap_or_default(),
            span: to_span(e.span()),
        })
}

pub(crate) fn enum_p<'a>() -> impl Parser<'a, &'a str, EnumDecl, ErrTy<'a>> {
    let variants = variant_p()
        .separated_by(just(',').padded().labelled("comma"))
        .allow_trailing()
        .collect::<Vec<_>>();

    kw("enum")
        .ignore_then(ident_p().map_with(|name, e| (name, to_span(e.span()))))
        .then(type_params_p().or_not())
        .then(variants.delimited_by(just('{').padded(), just('}').padded()))
        .map_with(|(((name, name_span), type_params), variants), e| EnumDecl {
            is_exported: false,
            name,
            name_span,
            type_params: type_params.unwrap_or_default(),
            variants,
            span: to_span(e.span()),
        })
}
