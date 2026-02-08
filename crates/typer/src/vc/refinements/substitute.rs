use clg_ast::{BinOp, Expr, Span, Stmt};

pub(crate) fn fold_conjunction(mut exprs: Vec<Expr>) -> Expr {
    let mut iter = exprs.drain(..);
    let first = iter.next().expect("exprs not empty");
    iter.fold(first, |lhs, rhs| {
        let span = merge_span(&lhs, &rhs);
        Expr::Bin {
            op: BinOp::And,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            span,
        }
    })
}

pub(super) fn substitute_binder(expr: &Expr, binder: &str, replacement: &Expr) -> Expr {
    match expr {
        Expr::Var(name, _) if name == binder => replacement.clone(),
        Expr::Var(_, _) | Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => expr.clone(),
        Expr::ArrayLit { elems, span } => Expr::ArrayLit {
            elems: elems
                .iter()
                .map(|e| substitute_binder(e, binder, replacement))
                .collect(),
            span: *span,
        },
        Expr::TupleLit { elems, span } => Expr::TupleLit {
            elems: elems
                .iter()
                .map(|e| substitute_binder(e, binder, replacement))
                .collect(),
            span: *span,
        },
        Expr::StructLit { name, fields, span } => Expr::StructLit {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|field| clg_ast::StructFieldInit {
                    name: field.name.clone(),
                    expr: substitute_binder(&field.expr, binder, replacement),
                    span: field.span,
                })
                .collect(),
            span: *span,
        },
        Expr::FieldAccess { base, field, span } => Expr::FieldAccess {
            base: Box::new(substitute_binder(base, binder, replacement)),
            field: field.clone(),
            span: *span,
        },
        Expr::Index { base, index, span } => Expr::Index {
            base: Box::new(substitute_binder(base, binder, replacement)),
            index: Box::new(substitute_binder(index, binder, replacement)),
            span: *span,
        },
        Expr::Bin { op, lhs, rhs, span } => Expr::Bin {
            op: *op,
            lhs: Box::new(substitute_binder(lhs, binder, replacement)),
            rhs: Box::new(substitute_binder(rhs, binder, replacement)),
            span: *span,
        },
        Expr::Unary { op, expr, span } => Expr::Unary {
            op: *op,
            expr: Box::new(substitute_binder(expr, binder, replacement)),
            span: *span,
        },
        Expr::Call { callee, args, span } => Expr::Call {
            callee: callee.clone(),
            args: args
                .iter()
                .map(|a| substitute_binder(a, binder, replacement))
                .collect(),
            span: *span,
        },
        Expr::Return { expr, span } => Expr::Return {
            expr: Box::new(substitute_binder(expr, binder, replacement)),
            span: *span,
        },
        Expr::Try { expr, span } => Expr::Try {
            expr: Box::new(substitute_binder(expr, binder, replacement)),
            span: *span,
        },
        Expr::Block { block } => Expr::Block {
            block: Box::new(substitute_block_binder(block, binder, replacement)),
        },
        Expr::Match { .. } | Expr::If { .. } => expr.clone(),
    }
}

pub(crate) fn substitute_result(expr: &Expr, replacement: &Expr) -> Expr {
    match expr {
        Expr::Var(name, _) if name == "result" => replacement.clone(),
        Expr::Var(_, _) | Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => expr.clone(),
        Expr::ArrayLit { elems, span } => Expr::ArrayLit {
            elems: elems
                .iter()
                .map(|e| substitute_result(e, replacement))
                .collect(),
            span: *span,
        },
        Expr::TupleLit { elems, span } => Expr::TupleLit {
            elems: elems
                .iter()
                .map(|e| substitute_result(e, replacement))
                .collect(),
            span: *span,
        },
        Expr::StructLit { name, fields, span } => Expr::StructLit {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|field| clg_ast::StructFieldInit {
                    name: field.name.clone(),
                    expr: substitute_result(&field.expr, replacement),
                    span: field.span,
                })
                .collect(),
            span: *span,
        },
        Expr::FieldAccess { base, field, span } => Expr::FieldAccess {
            base: Box::new(substitute_result(base, replacement)),
            field: field.clone(),
            span: *span,
        },
        Expr::Index { base, index, span } => Expr::Index {
            base: Box::new(substitute_result(base, replacement)),
            index: Box::new(substitute_result(index, replacement)),
            span: *span,
        },
        Expr::Bin { op, lhs, rhs, span } => Expr::Bin {
            op: *op,
            lhs: Box::new(substitute_result(lhs, replacement)),
            rhs: Box::new(substitute_result(rhs, replacement)),
            span: *span,
        },
        Expr::Unary { op, expr, span } => Expr::Unary {
            op: *op,
            expr: Box::new(substitute_result(expr, replacement)),
            span: *span,
        },
        Expr::Call { callee, args, span } => Expr::Call {
            callee: callee.clone(),
            args: args
                .iter()
                .map(|a| substitute_result(a, replacement))
                .collect(),
            span: *span,
        },
        Expr::Return { expr, span } => Expr::Return {
            expr: Box::new(substitute_result(expr, replacement)),
            span: *span,
        },
        Expr::Try { expr, span } => Expr::Try {
            expr: Box::new(substitute_result(expr, replacement)),
            span: *span,
        },
        Expr::Block { block } => Expr::Block {
            block: Box::new(substitute_block(block, replacement)),
        },
        Expr::Match { .. } | Expr::If { .. } => expr.clone(),
    }
}

fn substitute_block_binder(
    block: &clg_ast::Block,
    binder: &str,
    replacement: &Expr,
) -> clg_ast::Block {
    let mut new_block = block.clone();
    new_block.statements = block
        .statements
        .iter()
        .map(|stmt| match stmt {
            Stmt::Let { name, expr, span } => Stmt::Let {
                name: name.clone(),
                expr: Box::new(substitute_binder(expr, binder, replacement)),
                span: *span,
            },
            Stmt::Expr { expr, span } => Stmt::Expr {
                expr: Box::new(substitute_binder(expr, binder, replacement)),
                span: *span,
            },
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => Stmt::While {
                cond: Box::new(substitute_binder(cond, binder, replacement)),
                invariant: Box::new(substitute_binder(invariant, binder, replacement)),
                variant: variant
                    .as_ref()
                    .map(|v| Box::new(substitute_binder(v, binder, replacement))),
                body: Box::new(substitute_block_binder(body, binder, replacement)),
                span: *span,
            },
        })
        .collect();
    new_block.tail = block
        .tail
        .as_ref()
        .map(|expr| Box::new(substitute_binder(expr, binder, replacement)));
    new_block
}

fn substitute_block(block: &clg_ast::Block, replacement: &Expr) -> clg_ast::Block {
    let mut new_block = block.clone();
    new_block.statements = block
        .statements
        .iter()
        .map(|stmt| match stmt {
            Stmt::Let { name, expr, span } => Stmt::Let {
                name: name.clone(),
                expr: Box::new(substitute_result(expr, replacement)),
                span: *span,
            },
            Stmt::Expr { expr, span } => Stmt::Expr {
                expr: Box::new(substitute_result(expr, replacement)),
                span: *span,
            },
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => Stmt::While {
                cond: Box::new(substitute_result(cond, replacement)),
                invariant: Box::new(substitute_result(invariant, replacement)),
                variant: variant
                    .as_ref()
                    .map(|v| Box::new(substitute_result(v, replacement))),
                body: Box::new(substitute_block(body, replacement)),
                span: *span,
            },
        })
        .collect();
    new_block.tail = block
        .tail
        .as_ref()
        .map(|expr| Box::new(substitute_result(expr, replacement)));
    new_block
}

fn merge_span(lhs: &Expr, rhs: &Expr) -> Span {
    let ls = span_of(lhs);
    let rs = span_of(rhs);
    Span {
        start: ls.start,
        end: rs.end,
    }
}

fn span_of(expr: &Expr) -> Span {
    match expr {
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
