use crate::generics::type_params_p;
use crate::tokens::{func_name_p, ident_p, kw};
use crate::types::{effect_p, ty_p};
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{Effect, Param, ParamKind, Span, TraitDecl, TraitMethod};

fn to_span(sp: chumsky::span::SimpleSpan<usize>) -> Span {
    Span {
        start: sp.start,
        end: sp.end,
    }
}

fn param_p<'a>() -> impl Parser<'a, &'a str, Param, ErrTy<'a>> {
    kw("consume")
        .or_not()
        .then(ident_p())
        .then_ignore(just(':').padded().labelled("':'"))
        .then(
            ty_p()
                .padded()
                .or_not()
                .try_map(|maybe_ty, span| match maybe_ty {
                    Some(ty) => Ok(ty),
                    None => Err(Rich::custom(span, "expected type")),
                }),
        )
        .map(|((consume_kw, name), ty)| Param {
            kind: if consume_kw.is_some() {
                ParamKind::Consume
            } else {
                ParamKind::Borrow
            },
            name,
            ty,
        })
        .padded()
}

fn params_p<'a>() -> impl Parser<'a, &'a str, Vec<Param>, ErrTy<'a>> {
    param_p()
        .separated_by(just(',').padded().labelled("comma"))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(
            just('(').padded().labelled("'('"),
            just(')').padded().labelled("')'"),
        )
}

fn trait_method_p<'a>() -> impl Parser<'a, &'a str, TraitMethod, ErrTy<'a>> {
    effect_p()
        .map_with(|eff, e| (eff, to_span(e.span())))
        .or_not()
        .then_ignore(kw("function"))
        .then(func_name_p())
        .then(params_p())
        .then(
            choice((just("->").padded().to(true), just(':').padded().to(false))).try_map(
                |ok, span| {
                    if ok {
                        Ok(())
                    } else {
                        Err(Rich::custom(span, "use '->' for return types (not ':')"))
                    }
                },
            ),
        )
        .then(ty_p().padded())
        .then_ignore(just(';').padded().labelled("';'"))
        .map_with(|((((eff_opt, name), params), _arrow_ok), ret), e| {
            let (effect, effect_span) = eff_opt
                .map(|(eff, span)| (eff, Some(span)))
                .unwrap_or((Effect::None, None));
            TraitMethod {
                effect,
                effect_span,
                name,
                params,
                ret,
                span: to_span(e.span()),
            }
        })
}

pub(crate) fn trait_p<'a>() -> impl Parser<'a, &'a str, TraitDecl, ErrTy<'a>> {
    let methods = trait_method_p().repeated().collect::<Vec<_>>();

    kw("trait")
        .ignore_then(ident_p().map_with(|name, e| (name, to_span(e.span()))))
        .then(type_params_p().or_not())
        .then(methods.delimited_by(just('{').padded(), just('}').padded()))
        .map_with(|(((name, name_span), type_params), methods), e| TraitDecl {
            name,
            name_span,
            type_params: type_params.unwrap_or_default(),
            methods,
            span: to_span(e.span()),
        })
}
