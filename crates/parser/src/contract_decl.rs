use crate::func::{func_p, ParsedFunc};
use crate::tokens::{ident_p, int_literal_value_p, kw};
use crate::types::ty_p;
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{ContractDecl, RefinedAlias, Span, StructField};

#[derive(Debug, Clone)]
pub(crate) struct ParsedContract {
    pub contract: ContractDecl,
    pub inline_aliases: Vec<RefinedAlias>,
}

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

pub(crate) fn contract_p<'a>() -> impl Parser<'a, &'a str, ParsedContract, ErrTy<'a>> {
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
                .then(func_p().repeated().collect::<Vec<ParsedFunc>>())
                .delimited_by(just('{').padded(), just('}').padded()),
        )
        .map_with(
            |((name_and_span, version), (fields, parsed_functions)), e| {
                let mut inline_aliases = Vec::new();
                let functions = parsed_functions
                    .into_iter()
                    .map(|parsed| {
                        inline_aliases.extend(parsed.inline_aliases);
                        parsed.func
                    })
                    .collect();
                ParsedContract {
                    contract: ContractDecl {
                        name: name_and_span.0,
                        name_span: name_and_span.1,
                        version,
                        fields,
                        functions,
                        span: to_span(e.span()),
                    },
                    inline_aliases,
                }
            },
        )
}
