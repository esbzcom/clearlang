use clg_ast::{Expr, Program};
use std::collections::HashSet;

pub(super) fn collect_used_intrinsics(ast: &Program) -> HashSet<&'static str> {
    let mut set: HashSet<&'static str> = HashSet::new();
    fn walk_expr(e: &Expr, set: &mut HashSet<&'static str>) {
        match e {
            Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
            Expr::Return { expr, .. } => walk_expr(expr, set),
            Expr::Unary { expr, .. } => walk_expr(expr, set),
            Expr::Bin { lhs, rhs, .. } => {
                walk_expr(lhs, set);
                walk_expr(rhs, set);
            }
            Expr::If {
                cond,
                then_br,
                else_br,
                ..
            } => {
                walk_expr(cond, set);
                walk_expr(then_br, set);
                walk_expr(else_br, set);
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                walk_expr(scrutinee, set);
                for arm in arms {
                    walk_expr(&arm.expr, set);
                }
            }
            Expr::Call { callee, args, .. } => {
                match callee.as_str() {
                    "std::str::len" => {
                        set.insert("std::str::len");
                    }
                    "std::str::eq" => {
                        set.insert("std::str::eq");
                    }
                    "std::str::concat" => {
                        set.insert("std::str::concat");
                    }
                    _ => {}
                }
                for a in args {
                    walk_expr(a, set);
                }
            }
        }
    }
    for f in &ast.funcs {
        walk_expr(&f.body, &mut set);
    }
    set
}
