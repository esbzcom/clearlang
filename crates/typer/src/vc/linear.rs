use clg_ast::{Block, Expr, Func, Span, Stmt, Type};
use std::collections::{BTreeSet, HashSet};

#[derive(Debug, Clone)]
pub(super) struct LinearControlObligation {
    pub span: Span,
    pub vars: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct LinearControlObligations {
    pub branches: Vec<LinearControlObligation>,
    pub loops: Vec<LinearControlObligation>,
}

pub(super) fn collect_linear_control_obligations(
    func: &Func,
    resource_names: &HashSet<&str>,
) -> LinearControlObligations {
    let tracked = tracked_linear_vars(func, resource_names);
    let mut out = LinearControlObligations::default();
    collect_expr(&func.body, &tracked, &mut out);
    out
}

fn tracked_linear_vars(func: &Func, resource_names: &HashSet<&str>) -> HashSet<String> {
    func.params
        .iter()
        .filter(|param| contains_resource_type(&param.ty, resource_names))
        .map(|param| param.name.clone())
        .collect()
}

fn contains_resource_type(ty: &Type, resource_names: &HashSet<&str>) -> bool {
    match ty {
        Type::Named { name, args } => {
            resource_names.contains(name.as_str())
                || args
                    .iter()
                    .any(|arg| contains_resource_type(arg, resource_names))
        }
        Type::Option(inner) | Type::List(inner) | Type::Set(inner) | Type::Slice(inner) => {
            contains_resource_type(inner, resource_names)
        }
        Type::Result(ok, err) | Type::Map(ok, err) => {
            contains_resource_type(ok, resource_names)
                || contains_resource_type(err, resource_names)
        }
        Type::Array(inner, _) => contains_resource_type(inner, resource_names),
        Type::Tuple(elements) => elements
            .iter()
            .any(|elem| contains_resource_type(elem, resource_names)),
        _ => false,
    }
}

fn collect_expr(expr: &Expr, tracked: &HashSet<String>, out: &mut LinearControlObligations) {
    match expr {
        Expr::Block { block } => collect_block(block, tracked, out),
        Expr::If {
            cond,
            then_br,
            else_br,
            span,
        } => {
            let mut vars = BTreeSet::new();
            collect_targets_expr(then_br, tracked, &mut vars);
            collect_targets_expr(else_br, tracked, &mut vars);
            if !vars.is_empty() {
                out.branches.push(LinearControlObligation {
                    span: *span,
                    vars: vars.into_iter().collect(),
                });
            }
            collect_expr(cond, tracked, out);
            collect_expr(then_br, tracked, out);
            collect_expr(else_br, tracked, out);
        }
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => {
            let mut vars = BTreeSet::new();
            for arm in arms {
                collect_targets_expr(&arm.expr, tracked, &mut vars);
            }
            if !vars.is_empty() {
                out.branches.push(LinearControlObligation {
                    span: *span,
                    vars: vars.into_iter().collect(),
                });
            }
            collect_expr(scrutinee, tracked, out);
            for arm in arms {
                collect_expr(&arm.expr, tracked, out);
            }
        }
        Expr::Bin { lhs, rhs, .. } => {
            collect_expr(lhs, tracked, out);
            collect_expr(rhs, tracked, out);
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_expr(elem, tracked, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_expr(&field.expr, tracked, out);
            }
        }
        Expr::FieldAccess { base, .. } => collect_expr(base, tracked, out),
        Expr::Index { base, index, .. } => {
            collect_expr(base, tracked, out);
            collect_expr(index, tracked, out);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                collect_expr(arg, tracked, out);
            }
        }
        Expr::Unary { expr, .. } | Expr::Try { expr, .. } | Expr::Return { expr, .. } => {
            collect_expr(expr, tracked, out);
        }
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
    }
}

fn collect_block(block: &Block, tracked: &HashSet<String>, out: &mut LinearControlObligations) {
    let mut local = tracked.clone();
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                collect_expr(expr, &local, out);
                local.remove(name.as_str());
                if expr_may_be_linear(expr, &local) {
                    local.insert(name.clone());
                }
            }
            Stmt::Expr { expr, .. } => {
                collect_expr(expr, &local, out);
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => {
                let mut vars = BTreeSet::new();
                collect_targets_block(body, &local, &mut vars);
                if !vars.is_empty() {
                    out.loops.push(LinearControlObligation {
                        span: *span,
                        vars: vars.into_iter().collect(),
                    });
                }

                collect_expr(cond, &local, out);
                collect_expr(invariant, &local, out);
                if let Some(v) = variant {
                    collect_expr(v, &local, out);
                }
                collect_block(body, &local, out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_expr(tail, &local, out);
    }
}

fn collect_targets_block(block: &Block, tracked: &HashSet<String>, out: &mut BTreeSet<String>) {
    let mut local = tracked.clone();
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                collect_targets_expr(expr, &local, out);
                local.remove(name.as_str());
                if expr_may_be_linear(expr, &local) {
                    local.insert(name.clone());
                }
            }
            Stmt::Expr { expr, .. } => collect_targets_expr(expr, &local, out),
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                collect_targets_expr(cond, &local, out);
                collect_targets_expr(invariant, &local, out);
                if let Some(v) = variant {
                    collect_targets_expr(v, &local, out);
                }
                collect_targets_block(body, &local, out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_targets_expr(tail, &local, out);
    }
}

fn collect_targets_expr(expr: &Expr, tracked: &HashSet<String>, out: &mut BTreeSet<String>) {
    match expr {
        Expr::Call { callee, args, .. } => {
            if is_ownership_api(callee) {
                if let Some(target) = ownership_target(callee, args, tracked) {
                    out.insert(target.to_string());
                } else if let Some(first) = args.first() {
                    collect_tracked_vars_expr(first, tracked, out);
                }
            }
            for arg in args {
                collect_targets_expr(arg, tracked, out);
            }
        }
        Expr::Block { block } => collect_targets_block(block, tracked, out),
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_targets_expr(cond, tracked, out);
            collect_targets_expr(then_br, tracked, out);
            collect_targets_expr(else_br, tracked, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_targets_expr(scrutinee, tracked, out);
            for arm in arms {
                collect_targets_expr(&arm.expr, tracked, out);
            }
        }
        Expr::Bin { lhs, rhs, .. } => {
            collect_targets_expr(lhs, tracked, out);
            collect_targets_expr(rhs, tracked, out);
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_targets_expr(elem, tracked, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_targets_expr(&field.expr, tracked, out);
            }
        }
        Expr::FieldAccess { base, .. } => collect_targets_expr(base, tracked, out),
        Expr::Index { base, index, .. } => {
            collect_targets_expr(base, tracked, out);
            collect_targets_expr(index, tracked, out);
        }
        Expr::Unary { expr, .. } | Expr::Try { expr, .. } | Expr::Return { expr, .. } => {
            collect_targets_expr(expr, tracked, out);
        }
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
    }
}

fn collect_tracked_vars_block(
    block: &Block,
    tracked: &HashSet<String>,
    out: &mut BTreeSet<String>,
) {
    let mut local = tracked.clone();
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                collect_tracked_vars_expr(expr, &local, out);
                local.remove(name.as_str());
            }
            Stmt::Expr { expr, .. } => collect_tracked_vars_expr(expr, &local, out),
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                collect_tracked_vars_expr(cond, &local, out);
                collect_tracked_vars_expr(invariant, &local, out);
                if let Some(v) = variant {
                    collect_tracked_vars_expr(v, &local, out);
                }
                collect_tracked_vars_block(body, &local, out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_tracked_vars_expr(tail, &local, out);
    }
}

fn collect_tracked_vars_expr(expr: &Expr, tracked: &HashSet<String>, out: &mut BTreeSet<String>) {
    match expr {
        Expr::Var(name, _) => {
            if tracked.contains(name.as_str()) {
                out.insert(name.clone());
            }
        }
        Expr::Block { block } => collect_tracked_vars_block(block, tracked, out),
        Expr::Call { args, .. } => {
            for arg in args {
                collect_tracked_vars_expr(arg, tracked, out);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_tracked_vars_expr(cond, tracked, out);
            collect_tracked_vars_expr(then_br, tracked, out);
            collect_tracked_vars_expr(else_br, tracked, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_tracked_vars_expr(scrutinee, tracked, out);
            for arm in arms {
                collect_tracked_vars_expr(&arm.expr, tracked, out);
            }
        }
        Expr::Bin { lhs, rhs, .. } => {
            collect_tracked_vars_expr(lhs, tracked, out);
            collect_tracked_vars_expr(rhs, tracked, out);
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_tracked_vars_expr(elem, tracked, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_tracked_vars_expr(&field.expr, tracked, out);
            }
        }
        Expr::FieldAccess { base, .. } => collect_tracked_vars_expr(base, tracked, out),
        Expr::Index { base, index, .. } => {
            collect_tracked_vars_expr(base, tracked, out);
            collect_tracked_vars_expr(index, tracked, out);
        }
        Expr::Unary { expr, .. } | Expr::Try { expr, .. } | Expr::Return { expr, .. } => {
            collect_tracked_vars_expr(expr, tracked, out);
        }
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => {}
    }
}

fn expr_may_be_linear(expr: &Expr, tracked: &HashSet<String>) -> bool {
    match expr {
        Expr::Var(name, _) => tracked.contains(name.as_str()),
        Expr::TupleLit { elems, .. } | Expr::ArrayLit { elems, .. } => {
            elems.iter().any(|elem| expr_may_be_linear(elem, tracked))
        }
        Expr::Call { callee, args, .. } => {
            if let Some(target) = ownership_target(callee, args, tracked) {
                return tracked.contains(target);
            }
            match callee.as_str() {
                "Some" | "Ok" | "Err" => args
                    .first()
                    .map(|arg| expr_may_be_linear(arg, tracked))
                    .unwrap_or(false),
                _ => args.iter().any(|arg| expr_may_be_linear(arg, tracked)),
            }
        }
        Expr::Index { base, .. } => expr_may_be_linear(base, tracked),
        Expr::If {
            then_br, else_br, ..
        } => expr_may_be_linear(then_br, tracked) || expr_may_be_linear(else_br, tracked),
        Expr::Match { arms, .. } => arms
            .iter()
            .any(|arm| expr_may_be_linear(&arm.expr, tracked)),
        Expr::Block { block } => block
            .tail
            .as_ref()
            .map(|tail| expr_may_be_linear(tail, tracked))
            .unwrap_or(false),
        Expr::Unary { expr, .. } | Expr::Try { expr, .. } | Expr::Return { expr, .. } => {
            expr_may_be_linear(expr, tracked)
        }
        Expr::Bin { lhs, rhs, .. } => {
            expr_may_be_linear(lhs, tracked) || expr_may_be_linear(rhs, tracked)
        }
        Expr::StructLit { fields, .. } => fields
            .iter()
            .any(|field| expr_may_be_linear(&field.expr, tracked)),
        Expr::FieldAccess { base, .. } => expr_may_be_linear(base, tracked),
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => false,
    }
}

fn ownership_target<'a>(
    callee: &'a str,
    args: &'a [Expr],
    tracked: &HashSet<String>,
) -> Option<&'a str> {
    if !is_ownership_api(callee) {
        return None;
    }
    match args.first() {
        Some(Expr::Var(name, _)) if tracked.contains(name.as_str()) => Some(name.as_str()),
        _ => None,
    }
}

fn is_ownership_api(callee: &str) -> bool {
    let normalized = match callee {
        "std::list::push_mut" => "std::list::push",
        "std::list::insert_mut" => "std::list::insert",
        "std::list::remove_mut" => "std::list::remove",
        "std::list::pop_mut" => "std::list::pop",
        "std::map::insert_mut" => "std::map::insert",
        "std::map::remove_mut" => "std::map::remove",
        other => other,
    };

    matches!(
        normalized,
        "std::list::push"
            | "std::list::insert"
            | "std::list::remove"
            | "std::list::remove_take"
            | "std::list::pop"
            | "std::map::insert"
            | "std::map::insert_take"
            | "std::map::remove"
            | "std::map::remove_take"
    )
}
