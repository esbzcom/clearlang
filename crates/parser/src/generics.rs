use crate::tokens::{ident_p, kw};
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{Span, TraitBound, TypeParam};

fn to_span(sp: chumsky::span::SimpleSpan<usize>) -> Span {
    Span {
        start: sp.start,
        end: sp.end,
    }
}

pub(crate) fn type_params_p<'a>() -> impl Parser<'a, &'a str, Vec<TypeParam>, ErrTy<'a>> {
    ident_p()
        .map_with(|name, e| TypeParam {
            name,
            span: to_span(e.span()),
        })
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('<').padded(), just('>').padded())
}

pub(crate) fn type_params_with_bounds_p<'a>(
) -> impl Parser<'a, &'a str, (Vec<TypeParam>, Vec<TraitBound>), ErrTy<'a>> {
    let param = ident_p().map_with(|name, e| TypeParam {
        name,
        span: to_span(e.span()),
    });
    let param_with_bound = param.then(
        just(':')
            .padded()
            .ignore_then(ident_p().map_with(|trait_name, e| (trait_name, to_span(e.span()))))
            .or_not(),
    );
    param_with_bound
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('<').padded(), just('>').padded())
        .map(|entries| {
            let mut params = Vec::with_capacity(entries.len());
            let mut bounds = Vec::new();
            for (param, bound) in entries {
                if let Some((trait_name, span)) = bound {
                    bounds.push(TraitBound {
                        param: param.name.clone(),
                        trait_name,
                        span,
                    });
                }
                params.push(param);
            }
            (params, bounds)
        })
}

pub(crate) fn where_bounds_p<'a>() -> impl Parser<'a, &'a str, Vec<TraitBound>, ErrTy<'a>> {
    let bound = ident_p()
        .then_ignore(just(':').padded())
        .then(ident_p())
        .map_with(|(param, trait_name), e| TraitBound {
            param,
            trait_name,
            span: to_span(e.span()),
        });
    kw("where")
        .ignore_then(
            bound
                .separated_by(just(',').padded())
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .padded()
}
