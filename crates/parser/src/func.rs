use crate::expr::expr_p;
use crate::tokens::{func_name_p, ident_p, kw};
use crate::types::{effect_p, ty_p};
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{Contract, Effect, Func, Param, Span};

fn to_span(sp: chumsky::span::SimpleSpan<usize>) -> Span {
    Span {
        start: sp.start,
        end: sp.end,
    }
}

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
            just('(').padded().labelled("'('"),
            just(')').padded().labelled("')'"),
        )
}

pub(crate) fn func_p<'a>() -> impl Parser<'a, &'a str, Func, ErrTy<'a>> {
    let contract_block = expr_p()
        .delimited_by(just('{').padded(), just('}').padded())
        .map_with(|expr, e| Contract {
            span: to_span(e.span()),
            expr,
        })
        .boxed();

    #[derive(Debug)]
    enum Clause {
        Require(Contract),
        Ensure(Contract),
    }

    let clause_p = choice((
        kw("require")
            .ignore_then(contract_block.clone())
            .map(Clause::Require),
        kw("ensure")
            .ignore_then(contract_block.clone())
            .map(Clause::Ensure),
    ))
    .repeated()
    .collect::<Vec<_>>();

    effect_p()
        .map_with(|eff, e| (eff, to_span(e.span())))
        .or_not()
        .then_ignore(kw("function"))
        .then(func_name_p())
        .then(params_p())
        // Friendly hint: if ':' is used instead of '->' for return types, emit a targeted error
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
        .then(ty_p())
        .then(clause_p)
        .then(expr_p().delimited_by(just('{').padded(), just('}').padded()))
        .map(|((((((eff_opt, name), params), _arrow_ok), ret), clauses), body)| {
            let (effect, effect_span) = eff_opt
                .map(|(eff, span)| (eff, Some(span)))
                .unwrap_or((Effect::None, None));

            let mut requires = Vec::new();
            let mut ensures = Vec::new();
            for clause in clauses {
                match clause {
                    Clause::Require(c) => requires.push(c),
                    Clause::Ensure(c) => ensures.push(c),
                }
            }

            Func {
                effect,
                effect_span,
                name,
                params,
                ret,
                requires,
                ensures,
                body,
            }
        })
}
