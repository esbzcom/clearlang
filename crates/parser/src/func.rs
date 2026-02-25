use crate::expr::expr_p;
use crate::generics::{type_params_with_bounds_p, where_bounds_p};
use crate::tokens::{func_name_p, ident_p, kw};
use crate::types::{effect_p, ty_p};
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{
    Contract, Effect, Expr, Func, Param, ParamKind, RefinedAlias, Span, TraitBound, Type,
};

fn to_span(sp: chumsky::span::SimpleSpan<usize>) -> Span {
    Span {
        start: sp.start,
        end: sp.end,
    }
}

#[derive(Debug, Clone)]
struct ParsedParam {
    kind: ParamKind,
    name: String,
    ty: Type,
    inline_refinement: Option<(Expr, Span)>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedFunc {
    pub func: Func,
    pub inline_aliases: Vec<RefinedAlias>,
}

fn inline_alias_name(function_name: &str, slot: &str, disambiguator: usize) -> String {
    format!(
        "__clg$inline_ref${}${}$s{}",
        function_name, slot, disambiguator
    )
}

fn alias_type_args(type_params: &[clg_ast::TypeParam]) -> Vec<Type> {
    type_params
        .iter()
        .map(|tp| Type::Named {
            name: tp.name.clone(),
            args: Vec::new(),
        })
        .collect()
}

fn param_p<'a>() -> impl Parser<'a, &'a str, ParsedParam, ErrTy<'a>> {
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
        .then(
            kw("where")
                .ignore_then(expr_p().padded())
                .map_with(|predicate, e| (predicate, to_span(e.span())))
                .or_not(),
        )
        .map(
            |(((consume_kw, name), ty), inline_refinement)| ParsedParam {
                kind: if consume_kw.is_some() {
                    ParamKind::Consume
                } else {
                    ParamKind::Borrow
                },
                name,
                ty,
                inline_refinement,
            },
        )
        .padded()
}

fn params_p<'a>() -> impl Parser<'a, &'a str, Vec<ParsedParam>, ErrTy<'a>> {
    param_p()
        .separated_by(just(',').padded().labelled("comma"))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(
            just('(').padded().labelled("'('"),
            just(')').padded().labelled("')'"),
        )
}

enum SignatureWhereClause {
    Bounds(Vec<TraitBound>),
    ReturnRefinement(Expr, Span),
}

pub(crate) fn func_p<'a>() -> impl Parser<'a, &'a str, ParsedFunc, ErrTy<'a>> {
    let return_refinement_p = kw("where")
        .ignore_then(kw("result").then_ignore(just(':').padded().rewind().not()).or_not())
        .rewind()
        .try_map(|marker, span| {
            if marker.is_some() {
                Ok(())
            } else {
                Err(Rich::custom(
                    span,
                    "not a return refinement binder".to_string(),
                ))
            }
        })
        .ignore_then(
            kw("where")
                .ignore_then(expr_p().padded())
                .map_with(|predicate, e| {
                    SignatureWhereClause::ReturnRefinement(predicate, to_span(e.span()))
                }),
        );

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
        .then(type_params_with_bounds_p().or_not())
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
        .then(
            choice((
                return_refinement_p,
                where_bounds_p().map(SignatureWhereClause::Bounds),
            ))
            .or_not(),
        )
        .then(clause_p)
        .then(expr_p())
        .map(
            |(
                (
                    (
                        (((((eff_opt, name), type_params), params), _arrow_ok), ret),
                        sig_where_clause,
                    ),
                    clauses,
                ),
                body,
            )| {
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

                let (type_params, mut bounds) = type_params.unwrap_or_default();
                let mut ret_refinement = None;
                if let Some(where_clause) = sig_where_clause {
                    match where_clause {
                        SignatureWhereClause::Bounds(mut extra_bounds) => {
                            bounds.append(&mut extra_bounds);
                        }
                        SignatureWhereClause::ReturnRefinement(predicate, span) => {
                            ret_refinement = Some((predicate, span));
                        }
                    }
                }

                let type_param_names: Vec<String> =
                    type_params.iter().map(|tp| tp.name.clone()).collect();
                let type_args = alias_type_args(&type_params);

                let mut params_out = Vec::with_capacity(params.len());
                let mut inline_aliases = Vec::new();
                for (idx, param) in params.into_iter().enumerate() {
                    if let Some((predicate, span)) = param.inline_refinement {
                        let alias_name = inline_alias_name(
                            &name,
                            &format!("$param${}${}", idx, param.name),
                            span.start,
                        );
                        inline_aliases.push(RefinedAlias {
                            is_exported: false,
                            name: alias_name.clone(),
                            name_span: span,
                            type_params: type_param_names.clone(),
                            base: param.ty.clone(),
                            binder: Some(param.name.clone()),
                            predicate,
                            span,
                        });
                        params_out.push(Param {
                            kind: param.kind,
                            name: param.name,
                            ty: Type::Named {
                                name: alias_name,
                                args: type_args.clone(),
                            },
                        });
                    } else {
                        params_out.push(Param {
                            kind: param.kind,
                            name: param.name,
                            ty: param.ty,
                        });
                    }
                }

                let mut ret_out = ret;
                if let Some((predicate, span)) = ret_refinement {
                    let alias_name = inline_alias_name(&name, "$ret", span.start);
                    inline_aliases.push(RefinedAlias {
                        is_exported: false,
                        name: alias_name.clone(),
                        name_span: span,
                        type_params: type_param_names,
                        base: ret_out.clone(),
                        binder: Some("result".to_string()),
                        predicate,
                        span,
                    });
                    ret_out = Type::Named {
                        name: alias_name,
                        args: type_args,
                    };
                }

                let func = Func {
                    is_exported: false,
                    effect,
                    effect_span,
                    name,
                    type_params,
                    params: params_out,
                    ret: ret_out,
                    where_bounds: bounds,
                    requires,
                    ensures,
                    body,
                };

                ParsedFunc {
                    func,
                    inline_aliases,
                }
            },
        )
}
