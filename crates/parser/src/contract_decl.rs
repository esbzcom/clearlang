use crate::expr::expr_p;
use crate::func::{func_p, ParsedFunc};
use crate::tokens::{ident_p, int_literal_value_p, kw};
use crate::types::ty_p;
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{
    Contract, ContractDecl, ContractInitDecl, EventDecl, Expr, MigrationDecl, Param, ParamKind,
    RefinedAlias, Span, StructField,
};

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

fn invariant_p<'a>() -> impl Parser<'a, &'a str, Contract, ErrTy<'a>> {
    kw("invariant")
        .ignore_then(just('{').padded().labelled("'{'"))
        .ignore_then(expr_p())
        .then_ignore(just('}').padded().labelled("'}'"))
        .map_with(|expr, e| Contract {
            span: to_span(e.span()),
            expr,
        })
}

fn event_p<'a>() -> impl Parser<'a, &'a str, EventDecl, ErrTy<'a>> {
    kw("event")
        .ignore_then(ident_p().map_with(|name, e| (name, to_span(e.span()))))
        .then(state_fields_p().delimited_by(just('{').padded(), just('}').padded()))
        .map_with(|((name, name_span), fields), e| EventDecl {
            name,
            name_span,
            fields,
            span: to_span(e.span()),
        })
}

fn migration_p<'a>() -> impl Parser<'a, &'a str, MigrationDecl, ErrTy<'a>> {
    kw("migrate")
        .ignore_then(kw("from"))
        .ignore_then(kw("schema"))
        .ignore_then(expr_p().try_map(|expr, span| match expr {
            Expr::String(from_schema, string_span) => Ok((from_schema, string_span)),
            _ => Err(Rich::custom(
                span,
                "migration schema must be a string literal",
            )),
        }))
        .then(expr_p().try_map(|body, span| match body {
            Expr::Block { .. } => Ok(body),
            _ => Err(Rich::custom(span, "migration body must be a block")),
        }))
        .map_with(|((from_schema, from_schema_span), body), e| MigrationDecl {
            from_schema,
            from_schema_span,
            body,
            span: to_span(e.span()),
        })
}

fn init_param_p<'a>() -> impl Parser<'a, &'a str, Param, ErrTy<'a>> {
    ident_p()
        .then_ignore(just(':').padded().labelled("':'"))
        .then(ty_p().padded())
        .map(|(name, ty)| Param {
            kind: ParamKind::Borrow,
            name,
            ty,
        })
        .padded()
}

fn init_p<'a>() -> impl Parser<'a, &'a str, ContractInitDecl, ErrTy<'a>> {
    kw("init")
        .ignore_then(
            init_param_p()
                .separated_by(just(',').padded().labelled("comma"))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just('(').padded(), just(')').padded()),
        )
        .then(expr_p().try_map(|body, span| match body {
            Expr::Block { .. } => Ok(body),
            _ => Err(Rich::custom(span, "init body must be a block")),
        }))
        .map_with(|(params, body), e| ContractInitDecl {
            params,
            body,
            span: to_span(e.span()),
        })
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
                .then(event_p().repeated().collect::<Vec<_>>())
                .then(invariant_p().repeated().collect::<Vec<_>>())
                .then(init_p().or_not())
                .then(migration_p().or_not())
                .then(func_p().repeated().collect::<Vec<ParsedFunc>>())
                .delimited_by(just('{').padded(), just('}').padded()),
        )
        .map_with(
            |(
                (name_and_span, version),
                (((((fields, events), invariants), init), migration), parsed_functions),
            ),
             e| {
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
                        events,
                        invariants,
                        init,
                        migration,
                        functions,
                        span: to_span(e.span()),
                    },
                    inline_aliases,
                }
            },
        )
}
