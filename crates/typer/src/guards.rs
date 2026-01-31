use clg_ast::{Contract, Expr, MatchArm, Span, Stmt};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum MutCollectionKind {
    List,
    Set,
    Map,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct MutGuardKey {
    pub kind: MutCollectionKind,
    pub target: String,
}

#[derive(Clone, Debug)]
pub struct MutCall {
    pub callee: String,
    pub kind: MutCollectionKind,
    pub target: Option<String>,
    pub span: Span,
}

pub fn collect_mut_guards(contracts: &[Contract]) -> HashSet<MutGuardKey> {
    let mut guards = HashSet::new();
    for contract in contracts {
        collect_guards_from_expr(&contract.expr, &mut guards);
    }
    guards
}

fn collect_guards_from_expr(expr: &Expr, out: &mut HashSet<MutGuardKey>) {
    if let Some(key) = guard_key_from_guard_expr(expr) {
        out.insert(key);
    }
    match expr {
        Expr::Bin { lhs, rhs, .. } => {
            collect_guards_from_expr(lhs, out);
            collect_guards_from_expr(rhs, out);
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_guards_from_expr(elem, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_guards_from_expr(&field.expr, out);
            }
        }
        Expr::FieldAccess { base, .. } => {
            collect_guards_from_expr(base, out);
        }
        Expr::Index { base, index, .. } => {
            collect_guards_from_expr(base, out);
            collect_guards_from_expr(index, out);
        }
        Expr::Unary { expr, .. } | Expr::Return { expr, .. } | Expr::Try { expr, .. } => {
            collect_guards_from_expr(expr, out);
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_guards_from_expr(cond, out);
            collect_guards_from_expr(then_br, out);
            collect_guards_from_expr(else_br, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_guards_from_expr(scrutinee, out);
            for MatchArm { expr, .. } in arms {
                collect_guards_from_expr(expr, out);
            }
        }
        Expr::Block { block } => collect_guards_from_block(block, out),
        Expr::Call { args, .. } => {
            for arg in args {
                collect_guards_from_expr(arg, out);
            }
        }
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
    }
}

pub fn collect_mut_calls(expr: &Expr, out: &mut Vec<MutCall>) {
    match expr {
        Expr::Call { callee, args, span } => {
            if let Some(kind) = mut_collection_kind(callee.as_str()) {
                let target = args.first().and_then(guard_target);
                out.push(MutCall {
                    callee: callee.clone(),
                    kind,
                    target,
                    span: *span,
                });
            }
            for arg in args {
                collect_mut_calls(arg, out);
            }
        }
        Expr::Bin { lhs, rhs, .. } => {
            collect_mut_calls(lhs, out);
            collect_mut_calls(rhs, out);
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_mut_calls(elem, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_mut_calls(&field.expr, out);
            }
        }
        Expr::FieldAccess { base, .. } => {
            collect_mut_calls(base, out);
        }
        Expr::Index { base, index, .. } => {
            collect_mut_calls(base, out);
            collect_mut_calls(index, out);
        }
        Expr::Unary { expr, .. } | Expr::Return { expr, .. } | Expr::Try { expr, .. } => {
            collect_mut_calls(expr, out);
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_mut_calls(cond, out);
            collect_mut_calls(then_br, out);
            collect_mut_calls(else_br, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_mut_calls(scrutinee, out);
            for MatchArm { expr, .. } in arms {
                collect_mut_calls(expr, out);
            }
        }
        Expr::Block { block } => collect_calls_from_block(block, out),
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
    }
}

fn collect_guards_from_block(block: &clg_ast::Block, out: &mut HashSet<MutGuardKey>) {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                collect_guards_from_expr(expr.as_ref(), out);
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                collect_guards_from_expr(cond.as_ref(), out);
                collect_guards_from_expr(invariant.as_ref(), out);
                if let Some(v) = variant {
                    collect_guards_from_expr(v.as_ref(), out);
                }
                collect_guards_from_block(body.as_ref(), out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_guards_from_expr(tail.as_ref(), out);
    }
}

fn collect_calls_from_block(block: &clg_ast::Block, out: &mut Vec<MutCall>) {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                collect_mut_calls(expr.as_ref(), out);
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                collect_mut_calls(cond.as_ref(), out);
                collect_mut_calls(invariant.as_ref(), out);
                if let Some(v) = variant {
                    collect_mut_calls(v.as_ref(), out);
                }
                collect_calls_from_block(body.as_ref(), out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_mut_calls(tail.as_ref(), out);
    }
}

pub fn mut_collection_kind(callee: &str) -> Option<MutCollectionKind> {
    match callee {
        "std::list::push_mut"
        | "std::list::insert_mut"
        | "std::list::remove_mut"
        | "std::list::pop_mut" => Some(MutCollectionKind::List),
        "std::set::insert_mut" | "std::set::remove_mut" => Some(MutCollectionKind::Set),
        "std::map::insert_mut" | "std::map::remove_mut" => Some(MutCollectionKind::Map),
        _ => None,
    }
}

pub fn guard_kind_for_callee(callee: &str) -> Option<MutCollectionKind> {
    guard_collection_kind(callee)
}

pub fn guard_callee_for_kind(kind: MutCollectionKind) -> &'static str {
    match kind {
        MutCollectionKind::List => "std::list::can_mut",
        MutCollectionKind::Set => "std::set::can_mut",
        MutCollectionKind::Map => "std::map::can_mut",
    }
}

fn guard_key_from_guard_expr(expr: &Expr) -> Option<MutGuardKey> {
    match expr {
        Expr::Call { callee, args, .. } => {
            guard_collection_kind(callee.as_str()).and_then(|kind| {
                args.first()
                    .and_then(guard_target)
                    .map(|target| MutGuardKey { kind, target })
            })
        }
        _ => None,
    }
}

fn guard_collection_kind(callee: &str) -> Option<MutCollectionKind> {
    match callee {
        "std::list::can_mut" => Some(MutCollectionKind::List),
        "std::set::can_mut" => Some(MutCollectionKind::Set),
        "std::map::can_mut" => Some(MutCollectionKind::Map),
        _ => None,
    }
}

fn guard_target(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Var(name, _) => Some(name.clone()),
        _ => None,
    }
}
