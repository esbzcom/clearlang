mod blocks;
mod control;
mod shared;

use chumsky::prelude::*;
use clg_ast::{BinOp, Expr, LambdaParam, MatchArm, MatchPat, Span, StructFieldInit, UnaryOp};

use crate::literals::{bool_lit, int_lit, str_lit};
use crate::path::path_name_p;
use crate::tokens::{ctor_name_p, ident_p, kw};
use crate::types::ty_p;
use crate::ErrTy;

use blocks::block_expr_p;
use control::{if_expr_p, if_let_expr_p, match_expr_p};
use shared::{bin_expr, span_of, to_span};

pub(crate) fn expr_p<'a>() -> impl Parser<'a, &'a str, Expr, ErrTy<'a>> {
    recursive(|expr| {
        let call_args = expr
            .clone()
            .separated_by(just(',').padded().labelled("comma"))
            .collect::<Vec<_>>()
            .delimited_by(
                just('(').padded().labelled("'('"),
                just(')').padded().labelled("')'"),
            );

        let call_type_args = ty_p()
            .padded()
            .separated_by(just(',').padded().labelled("comma"))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just('<').padded(), just('>').padded());

        let call_expr = path_name_p()
            .then(call_type_args.or_not())
            .then(call_args.clone())
            .map_with(|((name, type_args), args), e| {
                let sp: chumsky::span::SimpleSpan<usize> = e.span();
                Expr::Call {
                    callee: name,
                    type_args: type_args.unwrap_or_default(),
                    args,
                    span: Span {
                        start: sp.start,
                        end: sp.end,
                    },
                }
            });

        let unsigned_cast = choice((kw("U8"), kw("U64"), kw("U128"), kw("U256")))
            .then(call_args.clone())
            .map_with(|(name, args), e| {
                let sp: chumsky::span::SimpleSpan<usize> = e.span();
                Expr::Call {
                    callee: name.to_string(),
                    type_args: Vec::new(),
                    args,
                    span: Span {
                        start: sp.start,
                        end: sp.end,
                    },
                }
            });

        let ctor_call =
            ctor_name_p()
                .then(call_args.clone().or_not())
                .map_with(|(name, maybe_args), e| {
                    let args = maybe_args.unwrap_or_default();
                    let sp: chumsky::span::SimpleSpan<usize> = e.span();
                    Expr::Call {
                        callee: name,
                        type_args: Vec::new(),
                        args,
                        span: Span {
                            start: sp.start,
                            end: sp.end,
                        },
                    }
                });

        let var_expr = ident_p().map_with(|name, e| {
            let sp: chumsky::span::SimpleSpan<usize> = e.span();
            Expr::Var(
                name,
                Span {
                    start: sp.start,
                    end: sp.end,
                },
            )
        });

        let ret_expr = just("return")
            .padded()
            .ignore_then(expr.clone())
            .map_with(|e_inner, e| {
                let sp = e.span();
                Expr::Return {
                    expr: Box::new(e_inner),
                    span: Span {
                        start: sp.start,
                        end: sp.end,
                    },
                }
            });

        let block_expr = block_expr_p(expr.clone()).boxed();
        let if_let_expr = if_let_expr_p(expr.clone(), block_expr.clone()).boxed();
        let if_expr = if_expr_p(expr.clone(), block_expr.clone()).boxed();
        let match_expr = match_expr_p(expr.clone()).boxed();

        let contract_kw_hint = choice((kw("require"), kw("ensure"))).try_map(|word, span| {
            Err(Rich::custom(
                span,
                format!("keyword `{word}` must be followed by `{{ ... }}`"),
            ))
        });

        let array_lit = expr
            .clone()
            .separated_by(just(',').padded().labelled("comma"))
            .at_least(1)
            .collect::<Vec<_>>()
            .delimited_by(
                just('[').padded().labelled("'['"),
                just(']').padded().labelled("']'"),
            )
            .map_with(|elems, e| {
                let sp = e.span();
                Expr::ArrayLit {
                    elems,
                    span: Span {
                        start: sp.start,
                        end: sp.end,
                    },
                }
            });

        let tuple_lit = expr
            .clone()
            .separated_by(just(',').padded().labelled("comma"))
            .at_least(2)
            .collect::<Vec<_>>()
            .delimited_by(
                just('(').padded().labelled("'('"),
                just(')').padded().labelled("')'"),
            )
            .map_with(|elems, e| {
                let sp = e.span();
                Expr::TupleLit {
                    elems,
                    span: Span {
                        start: sp.start,
                        end: sp.end,
                    },
                }
            });

        let struct_field = ident_p()
            .then_ignore(just(':').padded())
            .then(expr.clone())
            .map_with(|(name, expr), e| StructFieldInit {
                name,
                expr,
                span: to_span(e.span()),
            });

        let struct_lit = path_name_p()
            .then(
                struct_field
                    .separated_by(just(',').padded().labelled("comma"))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just('{').padded(), just('}').padded()),
            )
            .map_with(|(name, fields), e| {
                let sp = e.span();
                Expr::StructLit {
                    name,
                    fields,
                    span: Span {
                        start: sp.start,
                        end: sp.end,
                    },
                }
            });

        let lambda_param = ident_p()
            .then_ignore(just(':').padded().labelled("':'"))
            .then(ty_p().padded())
            .map_with(|(name, ty), e| LambdaParam {
                name,
                ty,
                span: to_span(e.span()),
            });

        let typed_lambda_params = lambda_param
            .separated_by(just(',').padded().labelled("comma"))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(
                just('(').padded().labelled("'('"),
                just(')').padded().labelled("')'"),
            )
            .boxed();

        let untyped_lambda_params = ident_p()
            .separated_by(just(',').padded().labelled("comma"))
            .at_least(1)
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(
                just('(').padded().labelled("'('"),
                just(')').padded().labelled("')'"),
            )
            .boxed();

        let lambda_expr = typed_lambda_params
            .clone()
            .then_ignore(just("=>").padded().labelled("'=>'"))
            .then(expr.clone())
            .map_with(|(params, body), e| Expr::Lambda {
                params,
                body: Box::new(body),
                span: to_span(e.span()),
            });

        let untyped_lambda_hint = untyped_lambda_params
            .clone()
            .then_ignore(just("=>").padded().labelled("'=>'"))
            .then(expr.clone())
            .try_map(|_, span| {
                Err(Rich::custom(
                    span,
                    "lambda parameters require type annotations (`name: Type`)".to_string(),
                ))
            });

        let capture_list_hint = ident_p()
            .separated_by(just(',').padded().labelled("comma"))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(
                just('[').padded().labelled("'['"),
                just(']').padded().labelled("']'"),
            )
            .then(choice((
                typed_lambda_params.clone().map(|_| ()),
                untyped_lambda_params.clone().map(|_| ()),
            )))
            .then_ignore(just("=>").padded().labelled("'=>'"))
            .then(expr.clone())
            .try_map(|_, span| {
                Err(Rich::custom(
                    span,
                    "capture-list syntax is not supported in Phase 17".to_string(),
                ))
            });

        let atom_base = choice((
            contract_kw_hint,
            capture_list_hint,
            lambda_expr,
            untyped_lambda_hint,
            int_lit(),
            bool_lit(),
            str_lit(),
            array_lit,
            tuple_lit,
            struct_lit,
            block_expr.clone(),
            expr.clone().delimited_by(
                just('(').padded().labelled("'('"),
                just(')').padded().labelled(")'"),
            ),
            ret_expr,
            match_expr,
            if_let_expr,
            if_expr,
            ctor_call,
            unsigned_cast,
            call_expr,
            var_expr,
        ))
        .padded()
        .labelled("expression")
        .boxed();

        let atom = atom_base
            .clone()
            .then(
                just("??")
                    .rewind()
                    .to(None)
                    .or(just('?').padded().to(Some(())))
                    .or(empty().to(None)),
            )
            .map_with(|(expr_inner, maybe_try), e| {
                if maybe_try.is_some() {
                    let sp = e.span();
                    Expr::Try {
                        expr: Box::new(expr_inner),
                        span: Span {
                            start: sp.start,
                            end: sp.end,
                        },
                    }
                } else {
                    expr_inner
                }
            })
            .boxed();

        enum PostfixOp {
            Index(Expr),
            Field { name: String, span: Span },
        }

        let indexer = just('[')
            .padded()
            .ignore_then(expr.clone())
            .then_ignore(just(']').padded())
            .map(PostfixOp::Index);

        let field_access = just('.')
            .padded()
            .ignore_then(ident_p())
            .map_with(|name, e| PostfixOp::Field {
                name,
                span: to_span(e.span()),
            });

        let postfix = atom
            .clone()
            .foldl(
                choice((indexer, field_access)).repeated(),
                |base, op| match op {
                    PostfixOp::Index(index) => {
                        let (ls, _) = span_of(&base);
                        let (_, re) = span_of(&index);
                        Expr::Index {
                            base: Box::new(base),
                            index: Box::new(index),
                            span: Span { start: ls, end: re },
                        }
                    }
                    PostfixOp::Field { name, span } => {
                        let (ls, _) = span_of(&base);
                        Expr::FieldAccess {
                            base: Box::new(base),
                            field: name,
                            span: Span {
                                start: ls,
                                end: span.end,
                            },
                        }
                    }
                },
            )
            .boxed();

        let unary = just('!')
            .padded()
            .repeated()
            .collect::<Vec<_>>()
            .then(postfix.clone())
            .map_with(|(nots, expr), e| {
                let sp = e.span();
                nots.into_iter().fold(expr, |acc, _| Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(acc),
                    span: Span {
                        start: sp.start,
                        end: sp.end,
                    },
                })
            })
            .boxed();

        let mul = unary.clone().foldl(
            (one_of("*/").padded().then(unary.clone().boxed())).repeated(),
            |lhs, (op, rhs)| {
                let op = if op == '*' { BinOp::Mul } else { BinOp::Div };
                bin_expr(op, lhs, rhs)
            },
        );

        let add = mul.clone().foldl(
            (one_of("+-").padded().then(mul.clone().boxed())).repeated(),
            |lhs, (op, rhs)| {
                let op = if op == '+' { BinOp::Add } else { BinOp::Sub };
                bin_expr(op, lhs, rhs)
            },
        );

        let shift = add.clone().foldl(
            choice((just("<<").to(BinOp::Shl), just(">>").to(BinOp::Shr)))
                .padded()
                .then(add.clone().boxed())
                .repeated(),
            |lhs, (op, rhs)| bin_expr(op, lhs, rhs),
        );

        let bit_and = shift.clone().foldl(
            just('&')
                .then_ignore(just('&').not())
                .padded()
                .to(BinOp::BitAnd)
                .then(shift.clone().boxed())
                .repeated(),
            |lhs, (op, rhs)| bin_expr(op, lhs, rhs),
        );

        let bit_xor = bit_and.clone().foldl(
            just('^')
                .padded()
                .to(BinOp::BitXor)
                .then(bit_and.clone().boxed())
                .repeated(),
            |lhs, (op, rhs)| bin_expr(op, lhs, rhs),
        );

        let bit_or = bit_xor.clone().foldl(
            just('|')
                .then_ignore(just('|').not())
                .padded()
                .to(BinOp::BitOr)
                .then(bit_xor.clone().boxed())
                .repeated(),
            |lhs, (op, rhs)| bin_expr(op, lhs, rhs),
        );

        let cmp_op = choice((
            just("<=").to(BinOp::Le),
            just(">=").to(BinOp::Ge),
            just("==").to(BinOp::Eq),
            just("!=").to(BinOp::Neq),
            just('<').to(BinOp::Lt),
            just('>').to(BinOp::Gt),
        ))
        .padded();

        let comparison = bit_or
            .clone()
            .then(cmp_op.then(bit_or.clone()).or_not())
            .map(|(lhs, opt)| match opt {
                Some((op, rhs)) => bin_expr(op, lhs, rhs),
                None => lhs,
            });

        let logical_and = comparison.clone().foldl(
            just("&&")
                .padded()
                .to(BinOp::And)
                .then(comparison.clone().boxed())
                .repeated(),
            |lhs, (op, rhs)| bin_expr(op, lhs, rhs),
        );

        let logical_or = logical_and.clone().foldl(
            just("||")
                .padded()
                .to(BinOp::Or)
                .then(logical_and.clone().boxed())
                .repeated(),
            |lhs, (op, rhs)| bin_expr(op, lhs, rhs),
        );

        logical_or.clone().foldl(
            just("??")
                .padded()
                .then(logical_or.clone().boxed())
                .repeated(),
            |lhs, (_, rhs)| {
                let (ls, _) = span_of(&lhs);
                let (_, re) = span_of(&rhs);
                let bind = format!("__coalesce_tmp{}", ls);
                let bind_span = Span { start: ls, end: ls };
                Expr::Match {
                    scrutinee: Box::new(lhs),
                    arms: vec![
                        MatchArm {
                            pat: MatchPat::Some(bind.clone()),
                            expr: Expr::Var(bind, bind_span),
                        },
                        MatchArm {
                            pat: MatchPat::None,
                            expr: rhs,
                        },
                    ],
                    span: Span { start: ls, end: re },
                }
            },
        )
    })
    .padded()
    .labelled("expression")
}
