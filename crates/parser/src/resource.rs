use crate::expr::expr_p;
use crate::tokens::{ident_p, kw};
use crate::types::ty_p;
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{Block, Resource, ResourceField, Span, Stmt};

fn to_span(sp: chumsky::span::SimpleSpan<usize>) -> Span {
    Span {
        start: sp.start,
        end: sp.end,
    }
}

fn drop_block_p<'a>() -> impl Parser<'a, &'a str, Block, ErrTy<'a>> {
    let expr = expr_p().boxed();

    let let_stmt = kw("let")
        .ignore_then(ident_p())
        .then_ignore(just('=').padded())
        .then(expr.clone())
        .then_ignore(just(';').padded().labelled("';'"))
        .map_with(|(name, value), e| {
            let sp = e.span();
            Stmt::Let {
                name,
                expr: value,
                span: to_span(sp),
            }
        });

    let expr_stmt = expr
        .clone()
        .then_ignore(just(';').padded().labelled("';'"))
        .map_with(|value, e| {
            let sp = e.span();
            Stmt::Expr {
                expr: value,
                span: to_span(sp),
            }
        });

    let stmts = choice((let_stmt, expr_stmt)).repeated().collect::<Vec<_>>();

    let tail = expr.or_not();

    stmts
        .then(tail)
        .delimited_by(just('{').padded(), just('}').padded())
        .map_with(|(statements, tail), e| Block {
            statements,
            tail,
            span: to_span(e.span()),
        })
}

fn resource_fields_p<'a>() -> impl Parser<'a, &'a str, Vec<ResourceField>, ErrTy<'a>> {
    let field = ident_p()
        .then_ignore(just(':').padded())
        .then(ty_p())
        .then_ignore(just(';').padded().labelled("';'"))
        .map_with(|(name, ty), e| {
            let sp = e.span();
            ResourceField {
                name,
                ty,
                span: to_span(sp),
            }
        });

    field.repeated().collect::<Vec<_>>()
}

pub(crate) fn resource_p<'a>() -> impl Parser<'a, &'a str, Resource, ErrTy<'a>> {
    let body = resource_fields_p().then(kw("drop").labelled("drop").ignore_then(drop_block_p()));

    kw("resource")
        .ignore_then(ident_p().map_with(|name, e| (name, to_span(e.span()))))
        .then(body.delimited_by(just('{').padded(), just('}').padded()))
        .map_with(|((name, name_span), (fields, drop_block)), e| Resource {
            name,
            name_span,
            fields,
            drop_block,
            span: to_span(e.span()),
        })
}
