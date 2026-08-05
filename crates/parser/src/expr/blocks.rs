use chumsky::prelude::*;
use clg_ast::{Block, Expr, Stmt};

use crate::tokens::{ident_p, kw};
use crate::ErrTy;

use super::shared::to_span;

pub(super) fn block_expr_p<'a, P>(expr: P) -> impl Parser<'a, &'a str, Expr, ErrTy<'a>>
where
    P: Parser<'a, &'a str, Expr, ErrTy<'a>> + Clone + 'a,
{
    let block_core = recursive(|block_core| {
        let expr_inner = expr.clone().boxed();

        let let_stmt = kw("let")
            .ignore_then(ident_p())
            .then_ignore(just('=').padded())
            .then(expr_inner.clone())
            .then_ignore(just(';').padded().labelled("';'"))
            .map_with(|(name, value), e| {
                let sp = e.span();
                Stmt::Let {
                    name,
                    expr: Box::new(value),
                    span: to_span(sp),
                }
            })
            .boxed();

        let state_write = kw("state")
            .then_ignore(just('.').padded().labelled("'.'"))
            .then(ident_p())
            .then_ignore(just('=').padded())
            .then(expr_inner.clone())
            .then_ignore(just(';').padded().labelled("';'"))
            .map_with(|((_, field), value), e| {
                let span = to_span(e.span());
                Stmt::Expr {
                    expr: Box::new(Expr::Call {
                        callee: format!("__clg_state_write${field}"),
                        type_args: Vec::new(),
                        args: vec![value],
                        span,
                    }),
                    span,
                }
            })
            .boxed();

        let while_stmt = kw("while")
            .padded()
            .ignore_then(expr_inner.clone())
            .then_ignore(kw("invariant").padded())
            .then(
                expr_inner
                    .clone()
                    .delimited_by(just('{').padded(), just('}').padded()),
            )
            .then(
                kw("variant")
                    .padded()
                    .ignore_then(
                        expr_inner
                            .clone()
                            .delimited_by(just('{').padded(), just('}').padded()),
                    )
                    .or_not(),
            )
            .then(block_core.clone())
            .map_with(|(((cond, invariant), variant), body), e| {
                let sp = e.span();
                Stmt::While {
                    cond: Box::new(cond),
                    invariant: Box::new(invariant),
                    variant: variant.map(Box::new),
                    body: Box::new(body),
                    span: to_span(sp),
                }
            })
            .boxed();

        let expr_stmt = expr_inner
            .clone()
            .then_ignore(just(';').padded().labelled("';'"))
            .map_with(|value, e| {
                let sp = e.span();
                Stmt::Expr {
                    expr: Box::new(value),
                    span: to_span(sp),
                }
            })
            .boxed();

        let stmts = choice((let_stmt, state_write, while_stmt, expr_stmt))
            .repeated()
            .collect::<Vec<_>>();
        let tail = expr_inner.or_not();

        stmts
            .then(tail)
            .delimited_by(just('{').padded(), just('}').padded())
            .map_with(|(statements, tail), e| Block {
                statements,
                tail: tail.map(Box::new),
                span: to_span(e.span()),
            })
    })
    .boxed();

    block_core
        .map(|block| Expr::Block {
            block: Box::new(block),
        })
        .boxed()
}
