use clg_ast::{Block, Expr, Program, Stmt};
use std::collections::HashSet;

pub(super) fn collect_used_intrinsics(ast: &Program) -> HashSet<&'static str> {
    let mut set: HashSet<&'static str> = HashSet::with_capacity(8);
    walk_calls(ast, &mut |callee| {
        let normalized = crate::guards::canonical_collection_alias_callee(callee);
        match normalized {
        "std::bytes::len" => {
            set.insert("std::bytes::len");
        }
        "std::bytes::is_empty" => {
            set.insert("std::bytes::len");
        }
        "std::bytes::eq" => {
            set.insert("std::bytes::eq");
        }
        "std::bytes::equals" => {
            set.insert("std::bytes::eq");
        }
        "std::bytes::eq_ct" => {
            set.insert("std::bytes::eq_ct");
        }
        "std::bytes::equals_ct" => {
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
        "std::env::chain_id" => {
            set.insert("std::env::chain_id");
        }
        "std::env::random" => {
            set.insert("std::env::random");
        }
        "std::crypto::hash" => {
            set.insert("std::crypto::hash");
        }
        "std::crypto::sha256" => {
            set.insert("std::crypto::hash");
        }
        "std::crypto::hmac" => {
            set.insert("std::crypto::hmac");
        }
        "std::crypto::hmac_sha256" => {
            set.insert("std::crypto::hmac");
        }
        "std::crypto::verify" => {
            set.insert("std::crypto::verify");
        }
        "std::str::len" => {
            set.insert("std::str::len");
        }
        "std::str::is_empty" => {
            set.insert("std::str::len");
        }
        "std::str::eq" => {
            set.insert("std::str::eq");
        }
        "std::str::equals" => {
            set.insert("std::str::eq");
        }
        "std::str::concat" => {
            set.insert("std::str::concat");
        }
        "std::str::starts_with" => {
            set.insert("std::str::starts_with");
        }
        "std::str::ends_with" => {
            set.insert("std::str::ends_with");
        }
        "std::str::contains" => {
            set.insert("std::str::contains");
        }
        "std::str_pattern::matches" => {
            set.insert("std::str::contains");
        }
        "std::str::to_bytes" => {
            set.insert("std::bytes::from_string");
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
        "std::map::contains"
        | "std::map::get"
        | "std::map::insert"
        | "std::map::remove"
        | "std::map::insert_take"
        | "std::map::remove_take" => {
            set.insert("std::str::eq");
            set.insert("std::bytes::eq");
        }
        _ => {}
    }});
    set
}

pub(super) fn collect_called_functions(ast: &Program) -> HashSet<String> {
    let mut out = HashSet::new();
    walk_calls(ast, &mut |callee| {
        out.insert(callee.to_string());
    });
    out
}

fn walk_calls(ast: &Program, on_call: &mut dyn FnMut(&str)) {
    fn walk_block(block: &Block, on_call: &mut dyn FnMut(&str)) {
        for stmt in &block.statements {
            match stmt {
                Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                    walk_expr(expr.as_ref(), on_call);
                }
                Stmt::While {
                    cond,
                    invariant,
                    variant,
                    body,
                    ..
                } => {
                    walk_expr(cond.as_ref(), on_call);
                    walk_expr(invariant.as_ref(), on_call);
                    if let Some(v) = variant {
                        walk_expr(v.as_ref(), on_call);
                    }
                    walk_block(body.as_ref(), on_call);
                }
            }
        }
        if let Some(tail) = &block.tail {
            walk_expr(tail.as_ref(), on_call);
        }
    }
    fn walk_expr(e: &Expr, on_call: &mut dyn FnMut(&str)) {
        match e {
            Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
            Expr::Return { expr, .. } => walk_expr(expr, on_call),
            Expr::Unary { expr, .. } | Expr::Try { expr, .. } => walk_expr(expr, on_call),
            Expr::Bin { lhs, rhs, .. } => {
                walk_expr(lhs, on_call);
                walk_expr(rhs, on_call);
            }
            Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
                for elem in elems {
                    walk_expr(elem, on_call);
                }
            }
            Expr::StructLit { fields, .. } => {
                for field in fields {
                    walk_expr(&field.expr, on_call);
                }
            }
            Expr::FieldAccess { base, .. } => {
                walk_expr(base, on_call);
            }
            Expr::Index { base, index, .. } => {
                walk_expr(base, on_call);
                walk_expr(index, on_call);
            }
            Expr::If {
                cond,
                then_br,
                else_br,
                ..
            } => {
                walk_expr(cond, on_call);
                walk_expr(then_br, on_call);
                walk_expr(else_br, on_call);
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                walk_expr(scrutinee, on_call);
                for arm in arms {
                    walk_expr(&arm.expr, on_call);
                }
            }
            Expr::Block { block } => walk_block(block, on_call),
            Expr::Lambda { body, .. } => walk_expr(body, on_call),
            Expr::Call { callee, args, .. } => {
                on_call(callee.as_str());
                for a in args {
                    walk_expr(a, on_call);
                }
            }
        }
    }

    for f in &ast.funcs {
        for req in &f.requires {
            walk_expr(&req.expr, on_call);
        }
        for ens in &f.ensures {
            walk_expr(&ens.expr, on_call);
        }
        walk_expr(&f.body, on_call);
    }
}
