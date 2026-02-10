use clg_ast::{Block, Expr, Program, Stmt};
use std::collections::HashSet;

pub(super) fn collect_used_intrinsics(ast: &Program) -> HashSet<&'static str> {
    let mut set: HashSet<&'static str> = HashSet::with_capacity(8);
    fn walk_block(block: &Block, set: &mut HashSet<&'static str>) {
        for stmt in &block.statements {
            match stmt {
                Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                    walk_expr(expr.as_ref(), set);
                }
                Stmt::While {
                    cond,
                    invariant,
                    variant,
                    body,
                    ..
                } => {
                    walk_expr(cond.as_ref(), set);
                    walk_expr(invariant.as_ref(), set);
                    if let Some(v) = variant {
                        walk_expr(v.as_ref(), set);
                    }
                    walk_block(body.as_ref(), set);
                }
            }
        }
        if let Some(tail) = &block.tail {
            walk_expr(tail.as_ref(), set);
        }
    }
    fn walk_expr(e: &Expr, set: &mut HashSet<&'static str>) {
        match e {
            Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
            Expr::Return { expr, .. } => walk_expr(expr, set),
            Expr::Unary { expr, .. } | Expr::Try { expr, .. } => walk_expr(expr, set),
            Expr::Bin { lhs, rhs, .. } => {
                walk_expr(lhs, set);
                walk_expr(rhs, set);
            }
            Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
                for elem in elems {
                    walk_expr(elem, set);
                }
            }
            Expr::StructLit { fields, .. } => {
                for field in fields {
                    walk_expr(&field.expr, set);
                }
            }
            Expr::FieldAccess { base, .. } => {
                walk_expr(base, set);
            }
            Expr::Index { base, index, .. } => {
                walk_expr(base, set);
                walk_expr(index, set);
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
            Expr::Block { block } => walk_block(block, set),
            Expr::Call { callee, args, .. } => {
                match callee.as_str() {
                    "std::bytes::len" => {
                        set.insert("std::bytes::len");
                    }
                    "std::bytes::eq" => {
                        set.insert("std::bytes::eq");
                    }
                    "std::bytes::eq_ct" => {
                        set.insert("std::bytes::eq_ct");
                    }
                    "std::bytes::concat" => {
                        set.insert("std::bytes::concat");
                    }
                    "std::bytes::from_string" => {
                        set.insert("std::bytes::from_string");
                    }
                    "std::bytes::to_string" => {
                        set.insert("std::bytes::to_string");
                    }
                    "std::wasi::print" => {
                        set.insert("std::wasi::print");
                    }
                    "std::env::time" => {
                        set.insert("std::env::time");
                    }
                    "std::env::random" => {
                        set.insert("std::env::random");
                    }
                    "std::crypto::hash" => {
                        set.insert("std::crypto::hash");
                    }
                    "std::crypto::hmac" => {
                        set.insert("std::crypto::hmac");
                    }
                    "std::crypto::verify" => {
                        set.insert("std::crypto::verify");
                    }
                    "std::str::len" => {
                        set.insert("std::str::len");
                    }
                    "std::str::eq" => {
                        set.insert("std::str::eq");
                    }
                    "std::str::concat" => {
                        set.insert("std::str::concat");
                    }
                    "std::u64::rotl" => {
                        set.insert("std::u64::rotl");
                    }
                    "std::u64::rotr" => {
                        set.insert("std::u64::rotr");
                    }
                    "std::u64::to_bytes_le" => {
                        set.insert("std::u64::to_bytes_le");
                    }
                    "std::u64::to_bytes_be" => {
                        set.insert("std::u64::to_bytes_be");
                    }
                    "std::u64::from_bytes_le" => {
                        set.insert("std::u64::from_bytes_le");
                    }
                    "std::u64::from_bytes_be" => {
                        set.insert("std::u64::from_bytes_be");
                    }
                    "std::set::contains"
                    | "std::set::insert"
                    | "std::set::remove"
                    | "std::set::insert_mut"
                    | "std::set::remove_mut"
                    | "std::map::contains"
                    | "std::map::get"
                    | "std::map::insert"
                    | "std::map::remove"
                    | "std::map::insert_take"
                    | "std::map::remove_take"
                    | "std::map::insert_mut"
                    | "std::map::remove_mut" => {
                        set.insert("std::str::eq");
                        set.insert("std::bytes::eq");
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
        for req in &f.requires {
            walk_expr(&req.expr, &mut set);
        }
        for ens in &f.ensures {
            walk_expr(&ens.expr, &mut set);
        }
        walk_expr(&f.body, &mut set);
    }
    set
}
