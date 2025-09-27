use crate::literals::{bool_lit, int_lit, str_lit};
use crate::path::path_name_p;
use crate::tokens::{ctor_name_p, ident_p};
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{BinOp, Expr, MatchArm, MatchPat, Span, UnaryOp};

#[derive(Clone)]
enum IfLetKind {
    OptionSome(String),
    ResultOk(String),
    ResultErr(String),
}

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

        // Prefer parsing a namespaced call when parentheses follow a path
        let call_expr = path_name_p()
            .then(call_args.clone())
            .map_with(|(name, args), e| {
                let sp: chumsky::span::SimpleSpan<usize> = e.span();
                Expr::Call {
                    callee: name,
                    args,
                    span: Span {
                        start: sp.start,
                        end: sp.end,
                    },
                }
            });

        // Constructors: Some(x), Ok(x), Err(e); None (with or without parentheses)
        let ctor_call =
            ctor_name_p()
                .then(call_args.clone().or_not())
                .map_with(|(name, maybe_args), e| {
                    let args = maybe_args.unwrap_or_default();
                    let sp: chumsky::span::SimpleSpan<usize> = e.span();
                    Expr::Call {
                        callee: name,
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

        // return expression
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

        // block expression: { expr }
        let block_expr = expr
            .clone()
            .delimited_by(just('{').padded(), just('}').padded())
            .boxed();

        let if_let_pat = choice((
            just("Some")
                .padded()
                .ignore_then(just('(').padded())
                .ignore_then(ident_p())
                .then_ignore(just(')').padded())
                .map(IfLetKind::OptionSome),
            just("Ok")
                .padded()
                .ignore_then(just('(').padded())
                .ignore_then(ident_p())
                .then_ignore(just(')').padded())
                .map(IfLetKind::ResultOk),
            just("Err")
                .padded()
                .ignore_then(just('(').padded())
                .ignore_then(ident_p())
                .then_ignore(just(')').padded())
                .map(IfLetKind::ResultErr),
        ));

        let if_let_expr = just("if")
            .padded()
            .ignore_then(just("let").padded())
            .ignore_then(if_let_pat)
            .then_ignore(just('=').padded())
            .then(expr.clone())
            .then(block_expr.clone())
            .then_ignore(just("else").padded())
            .then(block_expr.clone())
            .map_with(|(((pat_kind, scrutinee), then_br), else_br), e| {
                let sp = e.span();
                let (success_pat, failure_pat) = match pat_kind {
                    IfLetKind::OptionSome(name) => (MatchPat::Some(name), MatchPat::None),
                    IfLetKind::ResultOk(name) => {
                        let tmp = format!("__iflet_tmp{}", sp.start);
                        (MatchPat::Ok(name), MatchPat::Err(tmp))
                    }
                    IfLetKind::ResultErr(name) => {
                        let tmp = format!("__iflet_tmp{}", sp.start);
                        (MatchPat::Err(name), MatchPat::Ok(tmp))
                    }
                };
                let arms = vec![
                    MatchArm {
                        pat: success_pat,
                        expr: then_br,
                    },
                    MatchArm {
                        pat: failure_pat,
                        expr: else_br,
                    },
                ];
                Expr::Match {
                    scrutinee: Box::new(scrutinee),
                    arms,
                    span: Span {
                        start: sp.start,
                        end: sp.end,
                    },
                }
            });

        // if/else expression: if cond { then } (else if cond { then })* else { else }
        // Build nested If nodes from right to left to preserve associativity.
        let elif_kw = just("else")
            .padded()
            .ignore_then(just("if").padded())
            .to(());
        let if_head = just("if")
            .padded()
            .ignore_then(expr.clone())
            .then(block_expr.clone());
        let elif_chain = (elif_kw.ignore_then(expr.clone()).then(block_expr.clone()))
            .repeated()
            .collect::<Vec<_>>();
        let if_expr = if_head
            .then(elif_chain)
            .then_ignore(just("else").padded())
            .then(block_expr.clone())
            .map_with(|(((cond0, then0), mut elifs), else_br), e| {
                let sp = e.span();
                // Start from final else branch and fold the chain right-to-left
                let mut acc = else_br;
                while let Some((c, t)) = elifs.pop() {
                    acc = Expr::If {
                        cond: Box::new(c),
                        then_br: Box::new(t),
                        else_br: Box::new(acc),
                        span: Span {
                            start: sp.start,
                            end: sp.end,
                        },
                    };
                }
                Expr::If {
                    cond: Box::new(cond0),
                    then_br: Box::new(then0),
                    else_br: Box::new(acc),
                    span: Span {
                        start: sp.start,
                        end: sp.end,
                    },
                }
            });

        // match expression: match <expr> { <pat> => <expr>, ... }
        let some_pat = just("Some")
            .padded()
            .ignore_then(just('(').padded())
            .ignore_then(ident_p())
            .then_ignore(just(')').padded())
            .map(MatchPat::Some);
        let none_pat = just("None").padded().to(MatchPat::None);
        let ok_pat = just("Ok")
            .padded()
            .ignore_then(just('(').padded())
            .ignore_then(ident_p())
            .then_ignore(just(')').padded())
            .map(MatchPat::Ok);
        let err_pat = just("Err")
            .padded()
            .ignore_then(just('(').padded())
            .ignore_then(ident_p())
            .then_ignore(just(')').padded())
            .map(MatchPat::Err);
        let pat = choice((some_pat, none_pat, ok_pat, err_pat)).labelled("match pattern");
        let arm = pat
            .then_ignore(just("=>").padded())
            .then(expr.clone())
            .map(|(p, e)| MatchArm { pat: p, expr: e });
        let arms = arm
            .separated_by(just(',').padded())
            .collect::<Vec<_>>()
            .delimited_by(just('{').padded(), just('}').padded());
        let match_expr = just("match")
            .padded()
            .ignore_then(expr.clone())
            .then(arms)
            .map_with(|(scrut, arms), e| {
                let sp = e.span();
                Expr::Match {
                    scrutinee: Box::new(scrut),
                    arms,
                    span: Span {
                        start: sp.start,
                        end: sp.end,
                    },
                }
            });

        let atom_base = choice((
            int_lit(),
            bool_lit(),
            str_lit(),
            expr.clone().delimited_by(
                just('(').padded().labelled("'('"),
                just(')').padded().labelled(")'"),
            ),
            ret_expr,
            match_expr,
            if_let_expr,
            if_expr,
            ctor_call,
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

        // Helper to compute a span for composite expressions
        fn span_of(e: &Expr) -> (usize, usize) {
            match e {
                Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => {
                    (sp.start, sp.end)
                }
                Expr::Bin { span, .. }
                | Expr::Call { span, .. }
                | Expr::Match { span, .. }
                | Expr::Return { span, .. }
                | Expr::If { span, .. }
                | Expr::Unary { span, .. }
                | Expr::Try { span, .. } => (span.start, span.end),
            }
        }

        let unary = just('!')
            .padded()
            .repeated()
            .collect::<Vec<_>>()
            .then(atom.clone())
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

        // multiplicative (*, /)
        let mul = unary.clone().foldl(
            (one_of("*/").padded().then(unary.clone().boxed())).repeated(),
            |lhs, (op, rhs)| {
                let op = if op == '*' { BinOp::Mul } else { BinOp::Div };
                let (ls, _) = span_of(&lhs);
                let (_, re) = span_of(&rhs);
                Expr::Bin {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: Span { start: ls, end: re },
                }
            },
        );

        // additive (+, -)
        let add = mul.clone().foldl(
            (one_of("+-").padded().then(mul.clone().boxed())).repeated(),
            |lhs, (op, rhs)| {
                let op = if op == '+' { BinOp::Add } else { BinOp::Sub };
                let (ls, _) = span_of(&lhs);
                let (_, re) = span_of(&rhs);
                Expr::Bin {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: Span { start: ls, end: re },
                }
            },
        );

        // comparison: ==, !=, <, <=, >, >= (left associative but we only allow single comparison chain)
        let cmp_op = choice((
            just("<=").to(BinOp::Le),
            just(">=").to(BinOp::Ge),
            just("==").to(BinOp::Eq),
            just("!=").to(BinOp::Neq),
            just('<').to(BinOp::Lt),
            just('>').to(BinOp::Gt),
        ))
        .padded();

        let comparison = add
            .clone()
            .then(cmp_op.then(add.clone()).or_not())
            .map(|(lhs, opt)| match opt {
                Some((op, rhs)) => {
                    let (ls, _) = span_of(&lhs);
                    let (_, re) = span_of(&rhs);
                    Expr::Bin {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                        span: Span { start: ls, end: re },
                    }
                }
                None => lhs,
            });

        let logical_and = comparison.clone().foldl(
            just("&&")
                .padded()
                .to(BinOp::And)
                .then(comparison.clone().boxed())
                .repeated(),
            |lhs, (op, rhs)| {
                let (ls, _) = span_of(&lhs);
                let (_, re) = span_of(&rhs);
                Expr::Bin {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: Span { start: ls, end: re },
                }
            },
        );

        let logical_or = logical_and.clone().foldl(
            just("||")
                .padded()
                .to(BinOp::Or)
                .then(logical_and.clone().boxed())
                .repeated(),
            |lhs, (op, rhs)| {
                let (ls, _) = span_of(&lhs);
                let (_, re) = span_of(&rhs);
                Expr::Bin {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: Span { start: ls, end: re },
                }
            },
        );

        let coalesce = logical_or.clone().foldl(
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
        );

        coalesce
    })
    .padded()
    .labelled("expression")
}
