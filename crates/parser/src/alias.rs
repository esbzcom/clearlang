use crate::expr::expr_p;
use crate::tokens::{ident_p, kw};
use crate::types::ty_p;
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{Block, Expr, RefinedAlias, Span, Stmt};

fn to_span(sp: chumsky::span::SimpleSpan<usize>) -> Span {
    Span {
        start: sp.start,
        end: sp.end,
    }
}

fn type_params_p<'a>() -> impl Parser<'a, &'a str, Vec<String>, ErrTy<'a>> {
    ident_p()
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect()
        .delimited_by(just('<').padded(), just('>').padded())
}

pub(crate) fn refined_alias_p<'a>() -> impl Parser<'a, &'a str, RefinedAlias, ErrTy<'a>> {
    kw("type")
        .ignore_then(ident_p().map_with(|name, e| (name, to_span(e.span()))))
        .then(type_params_p().padded().or_not())
        .then_ignore(just('=').padded().labelled("'='"))
        .then(ty_p().padded())
        .then_ignore(kw("where").padded().labelled("where"))
        .then(expr_p().padded())
        .then_ignore(just(';').padded().labelled("';'"))
        .map_with(|((((name, name_span), type_params), base), predicate), e| {
            let binder = find_first_var(&predicate);
            RefinedAlias {
                name,
                name_span,
                type_params: type_params.unwrap_or_default(),
                base,
                binder,
                predicate,
                span: to_span(e.span()),
            }
        })
}

fn find_first_var(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Var(name, _) => Some(name.clone()),
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => None,
        Expr::Block { block } => find_first_var_in_block(block),
        Expr::Bin { lhs, rhs, .. } => find_first_var(lhs).or_else(|| find_first_var(rhs)),
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            elems.iter().find_map(find_first_var)
        }
        Expr::StructLit { fields, .. } => fields.iter().find_map(|f| find_first_var(&f.expr)),
        Expr::FieldAccess { base, .. } => find_first_var(base),
        Expr::Index { base, index, .. } => find_first_var(base).or_else(|| find_first_var(index)),
        Expr::Call { args, .. } => args.iter().find_map(find_first_var),
        Expr::Return { expr, .. } | Expr::Unary { expr, .. } | Expr::Try { expr, .. } => {
            find_first_var(expr)
        }
        Expr::Match {
            scrutinee, arms, ..
        } => find_first_var(scrutinee)
            .or_else(|| arms.iter().find_map(|arm| find_first_var(&arm.expr))),
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => find_first_var(cond)
            .or_else(|| find_first_var(then_br))
            .or_else(|| find_first_var(else_br)),
    }
}

fn find_first_var_in_block(block: &Block) -> Option<String> {
    for stmt in &block.statements {
        if let Some(name) = find_first_var_in_stmt(stmt) {
            return Some(name);
        }
    }
    if let Some(tail) = &block.tail {
        return find_first_var(tail);
    }
    None
}

fn find_first_var_in_stmt(stmt: &Stmt) -> Option<String> {
    match stmt {
        Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => find_first_var(expr),
        Stmt::While {
            cond,
            invariant,
            variant,
            body,
            ..
        } => find_first_var(cond)
            .or_else(|| find_first_var(invariant))
            .or_else(|| variant.as_ref().and_then(|v| find_first_var(v)))
            .or_else(|| find_first_var_in_block(body)),
    }
}
