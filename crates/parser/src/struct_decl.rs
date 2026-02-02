use crate::generics::type_params_p;
use crate::tokens::{ident_p, kw};
use crate::types::ty_p;
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{Span, StructDecl, StructField};

fn to_span(sp: chumsky::span::SimpleSpan<usize>) -> Span {
    Span {
        start: sp.start,
        end: sp.end,
    }
}

fn struct_fields_p<'a>() -> impl Parser<'a, &'a str, Vec<StructField>, ErrTy<'a>> {
    let field = ident_p()
        .then_ignore(just(':').padded())
        .then(ty_p().padded())
        .then_ignore(just(';').padded().labelled("';'"))
        .map_with(|(name, ty), e| StructField {
            name,
            ty,
            span: to_span(e.span()),
        });

    field.repeated().collect::<Vec<_>>()
}

pub(crate) fn struct_p<'a>() -> impl Parser<'a, &'a str, StructDecl, ErrTy<'a>> {
    kw("struct")
        .ignore_then(ident_p().map_with(|name, e| (name, to_span(e.span()))))
        .then(type_params_p().or_not())
        .then(struct_fields_p().delimited_by(just('{').padded(), just('}').padded()))
        .map_with(|(((name, name_span), type_params), fields), e| StructDecl {
            name,
            name_span,
            type_params: type_params.unwrap_or_default(),
            fields,
            span: to_span(e.span()),
        })
}
