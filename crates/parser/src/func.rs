use crate::expr::expr_p;
use crate::tokens::{func_name_p, ident_p, kw};
use crate::types::{effect_p, ty_p};
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{Contract, Effect, Func, Param, ParamKind, Span};

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

pub(crate) fn func_p<'a>() -> impl Parser<'a, &'a str, Func, ErrTy<'a>> {
    let contract_block = just('{')
        .padded()
        .labelled("'{'")
        .ignore_then(expr_p())
        .then_ignore(just('}').padded().labelled("'}'"))
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

    let require_kw = kw("require").then_ignore(just('{').padded().labelled("'{'").rewind());
    let ensure_kw = kw("ensure").then_ignore(just('{').padded().labelled("'{'").rewind());

    let require_clause = require_kw.ignore_then(contract_block.clone().or_not().try_map(
        |maybe, span| match maybe {
            Some(contract) => Ok(contract),
            None => Err(Rich::custom(span, "expected '{' to start contract block")),
        },
    ));
    let ensure_clause = ensure_kw.ignore_then(contract_block.clone().or_not().try_map(
        |maybe, span| match maybe {
            Some(contract) => Ok(contract),
            None => Err(Rich::custom(span, "expected '{' to start contract block")),
        },
    ));

    let clause_p = choice((
        require_clause.map(Clause::Require),
        ensure_clause.map(Clause::Ensure),
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
        .then(ty_p().padded())
        .then(clause_p)
        .then(expr_p())
        .map(
            |((((((eff_opt, name), params), _arrow_ok), ret), clauses), body)| {
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
            },
        )
}
