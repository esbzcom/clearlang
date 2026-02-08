use clg_ast::{Expr, Span, Stmt};

use super::LoopObligation;

pub(super) fn collect_loops<'a>(expr: &'a Expr, out: &mut Vec<LoopObligation<'a>>) {
    match expr {
        Expr::Block { block } => collect_loops_block(block, out),
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_loops(cond, out);
            collect_loops(then_br, out);
            collect_loops(else_br, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_loops(scrutinee, out);
            for arm in arms {
                collect_loops(&arm.expr, out);
            }
        }
        Expr::Bin { lhs, rhs, .. } => {
            collect_loops(lhs, out);
            collect_loops(rhs, out);
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_loops(elem, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_loops(&field.expr, out);
            }
        }
        Expr::Index { base, index, .. } => {
            collect_loops(base, out);
            collect_loops(index, out);
        }
        Expr::FieldAccess { base, .. } => {
            collect_loops(base, out);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                collect_loops(arg, out);
            }
        }
        Expr::Unary { expr, .. } | Expr::Return { expr, .. } | Expr::Try { expr, .. } => {
            collect_loops(expr, out);
        }
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
    }
}

fn collect_loops_block<'a>(block: &'a clg_ast::Block, out: &mut Vec<LoopObligation<'a>>) {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => collect_loops(expr, out),
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                out.push(LoopObligation {
                    invariant,
                    variant: variant.as_deref(),
                });
                collect_loops(cond, out);
                collect_loops(invariant, out);
                if let Some(v) = variant {
                    collect_loops(v, out);
                }
                collect_loops_block(body, out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_loops(tail, out);
    }
}

pub(super) fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => *sp,
        Expr::ArrayLit { span, .. }
        | Expr::TupleLit { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. } => *span,
        Expr::Bin { span, .. }
        | Expr::Call { span, .. }
        | Expr::Match { span, .. }
        | Expr::Return { span, .. }
        | Expr::If { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Try { span, .. } => *span,
        Expr::Block { block } => block.span,
    }
}
