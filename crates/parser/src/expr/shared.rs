use chumsky::span::SimpleSpan;
use clg_ast::{BinOp, Expr, Span};

pub(super) fn to_span(sp: SimpleSpan<usize>) -> Span {
    Span {
        start: sp.start,
        end: sp.end,
    }
}

pub(super) fn span_of(expr: &Expr) -> (usize, usize) {
    match expr {
        Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => {
            (sp.start, sp.end)
        }
        Expr::ArrayLit { span, .. }
        | Expr::TupleLit { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. } => (span.start, span.end),
        Expr::Block { block } => (block.span.start, block.span.end),
        Expr::Bin { span, .. }
        | Expr::Call { span, .. }
        | Expr::Match { span, .. }
        | Expr::Return { span, .. }
        | Expr::If { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Try { span, .. }
        | Expr::Lambda { span, .. } => (span.start, span.end),
    }
}

pub(super) fn bin_expr(op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
    let (ls, _) = span_of(&lhs);
    let (_, re) = span_of(&rhs);
    Expr::Bin {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
        span: Span { start: ls, end: re },
    }
}
