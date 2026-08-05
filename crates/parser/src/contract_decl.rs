use crate::tokens::{ident_p, int_literal_value_p, kw};
use crate::types::ty_p;
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{ContractDecl, Span, StructField};

fn to_span(sp: chumsky::span::SimpleSpan<usize>) -> Span {
    Span {
        start: sp.start,
        end: sp.end,
    }
}

fn state_fields_p<'a>() -> impl Parser<'a, &'a str, Vec<StructField>, ErrTy<'a>> {
    ident_p()
        .then_ignore(just(':').padded())
        .then(ty_p().padded())
        .then_ignore(just(';').padded().labelled("';'"))
        .map_with(|(name, ty), e| StructField {
            name,
            ty,
            span: to_span(e.span()),
        })
        .repeated()
        .collect::<Vec<_>>()
}

pub(crate) fn contract_p<'a>() -> impl Parser<'a, &'a str, ContractDecl, ErrTy<'a>> {
    kw("contract")
        .ignore_then(ident_p().map_with(|name, e| (name, to_span(e.span()))))
        .then(
            kw("version")
                .ignore_then(int_literal_value_p())
                .try_map(|version, span| {
                    u32::try_from(version)
                        .ok()
                        .filter(|version| *version > 0)
                        .ok_or_else(|| {
                            Rich::custom(span, "contract version must be a positive U32")
                        })
                }),
        )
        .then(
            kw("state")
                .ignore_then(state_fields_p().delimited_by(just('{').padded(), just('}').padded()))
                .delimited_by(just('{').padded(), just('}').padded()),
        )
        .map_with(|((name_and_span, version), fields), e| ContractDecl {
            name: name_and_span.0,
            name_span: name_and_span.1,
            version,
            fields,
            span: to_span(e.span()),
        })
}
