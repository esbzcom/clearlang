use chumsky::prelude::*;
use crate::ErrTy;
use crate::tokens::{ident_p, func_name_p, kw};
use crate::types::{effect_p, ty_p};
use crate::expr::expr_p;
use lumi_ast::{Param, Func, Effect};

fn param_p<'a>() -> impl Parser<'a, &'a str, Param, ErrTy<'a>> {
    ident_p()
        .then_ignore(just(':').padded())
        .then(ty_p())
        .map(|(name, ty)| Param { name, ty })
        .padded()
}

fn params_p<'a>() -> impl Parser<'a, &'a str, Vec<Param>, ErrTy<'a>> {
    param_p()
        .separated_by(just(',').padded().labelled("comma"))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(
            just('(').padded().labelled("'('") ,
            just(')').padded().labelled("')'")
        )
}

pub(crate) fn func_p<'a>() -> impl Parser<'a, &'a str, Func, ErrTy<'a>> {
    effect_p()
        .or_not()
        .then_ignore(choice((kw("fn"), kw("function"))))
        .then(func_name_p())
        .then(params_p())
        .then_ignore(just("->").padded())
        .then(ty_p())
        .then(expr_p().delimited_by(just('{').padded(), just('}').padded()))
        .map(|((((eff_opt, name), params), ret), body)| Func {
            effect: eff_opt.unwrap_or(Effect::None),
            name,
            params,
            ret,
            body,
        })
}
