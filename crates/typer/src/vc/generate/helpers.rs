use super::*;

#[derive(Default)]
struct AssumptionUsage {
    unsigned_types: BTreeSet<String>,
    bitwise_ops: BTreeSet<String>,
    crypto_intrinsics: BTreeSet<String>,
    primitive_dependencies: BTreeSet<String>,
    external_dependencies: BTreeSet<String>,
}

pub(super) fn collect_function_assumptions(
    func: &Func,
    dependencies: &AssumptionDependencies,
) -> Vec<AssumptionBoundary> {
    let mut usage = AssumptionUsage::default();
    for param in &func.params {
        collect_unsigned_from_type(&param.ty, &mut usage.unsigned_types);
    }
    collect_unsigned_from_type(&func.ret, &mut usage.unsigned_types);
    for req in &func.requires {
        collect_assumption_usage_expr(&req.expr, &mut usage, dependencies);
    }
    for ensure in &func.ensures {
        collect_assumption_usage_expr(&ensure.expr, &mut usage, dependencies);
    }
    collect_assumption_usage_expr(&func.body, &mut usage, dependencies);

    let mut assumptions = Vec::new();
    let uncovered_unsigned_symbols: Vec<String> = usage
        .unsigned_types
        .iter()
        .filter(|symbol| symbol.as_str() != "U64")
        .cloned()
        .collect();
    if !uncovered_unsigned_symbols.is_empty() {
        assumptions.push(AssumptionBoundary {
            id: ASSUMPTION_UNSIGNED_ID,
            category: AssumptionCategory::Unsigned,
            status: ASSUMPTION_STATUS_ASSUMED,
            message: ASSUMPTION_UNSIGNED_MESSAGE,
            symbols: uncovered_unsigned_symbols,
        });
    }
    let u64_bitwise_covered = usage.unsigned_types.contains("U64")
        && usage.unsigned_types.iter().all(|symbol| symbol == "U64");
    let uncovered_bitwise_symbols: Vec<String> = usage
        .bitwise_ops
        .iter()
        .filter(|symbol| !u64_bitwise_covered || !is_u64_covered_bitwise_symbol(symbol.as_str()))
        .cloned()
        .collect();
    if !uncovered_bitwise_symbols.is_empty() {
        assumptions.push(AssumptionBoundary {
            id: ASSUMPTION_BITWISE_ID,
            category: AssumptionCategory::Bitwise,
            status: ASSUMPTION_STATUS_ASSUMED,
            message: ASSUMPTION_BITWISE_MESSAGE,
            symbols: uncovered_bitwise_symbols,
        });
    }
    if !usage.crypto_intrinsics.is_empty() {
        assumptions.push(AssumptionBoundary {
            id: ASSUMPTION_CRYPTO_ID,
            category: AssumptionCategory::Crypto,
            status: ASSUMPTION_STATUS_ASSUMED,
            message: ASSUMPTION_CRYPTO_MESSAGE,
            symbols: usage.crypto_intrinsics.iter().cloned().collect(),
        });
    }
    if !usage.primitive_dependencies.is_empty() {
        assumptions.push(AssumptionBoundary {
            id: ASSUMPTION_PRIMITIVE_ID,
            category: AssumptionCategory::Primitive,
            status: ASSUMPTION_STATUS_ASSUMED,
            message: ASSUMPTION_PRIMITIVE_MESSAGE,
            symbols: usage.primitive_dependencies.iter().cloned().collect(),
        });
    }
    if !usage.external_dependencies.is_empty() {
        assumptions.push(AssumptionBoundary {
            id: ASSUMPTION_EXTERNAL_ID,
            category: AssumptionCategory::External,
            status: ASSUMPTION_STATUS_ASSUMED,
            message: ASSUMPTION_EXTERNAL_MESSAGE,
            symbols: usage.external_dependencies.iter().cloned().collect(),
        });
    }
    assumptions
}

fn collect_unsigned_from_type(ty: &Type, out: &mut BTreeSet<String>) {
    match ty {
        Type::U8 => {
            out.insert("U8".to_string());
        }
        Type::U64 => {
            out.insert("U64".to_string());
        }
        Type::U128 => {
            out.insert("U128".to_string());
        }
        Type::U256 => {
            out.insert("U256".to_string());
        }
        Type::Option(inner)
        | Type::List(inner)
        | Type::Set(inner)
        | Type::Array(inner, _)
        | Type::Slice(inner) => collect_unsigned_from_type(inner, out),
        Type::Result(ok, err) | Type::Map(ok, err) => {
            collect_unsigned_from_type(ok, out);
            collect_unsigned_from_type(err, out);
        }
        Type::Tuple(items) | Type::Named { args: items, .. } => {
            for item in items {
                collect_unsigned_from_type(item, out);
            }
        }
        Type::Fn { params, ret } => {
            for param in params {
                collect_unsigned_from_type(param, out);
            }
            collect_unsigned_from_type(ret, out);
        }
        Type::Int | Type::Bool | Type::String | Type::Bytes => {}
    }
}

fn collect_assumption_usage_block(
    block: &Block,
    out: &mut AssumptionUsage,
    dependencies: &AssumptionDependencies,
) {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                collect_assumption_usage_expr(expr, out, dependencies);
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                collect_assumption_usage_expr(cond, out, dependencies);
                collect_assumption_usage_expr(invariant, out, dependencies);
                if let Some(variant_expr) = variant {
                    collect_assumption_usage_expr(variant_expr, out, dependencies);
                }
                collect_assumption_usage_block(body, out, dependencies);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_assumption_usage_expr(tail, out, dependencies);
    }
}

fn collect_assumption_usage_expr(
    expr: &Expr,
    out: &mut AssumptionUsage,
    dependencies: &AssumptionDependencies,
) {
    match expr {
        Expr::Int(..) | Expr::Bool(..) | Expr::String(..) | Expr::Var(..) => {}
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_assumption_usage_expr(elem, out, dependencies);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_assumption_usage_expr(&field.expr, out, dependencies);
            }
        }
        Expr::FieldAccess { base, .. } => collect_assumption_usage_expr(base, out, dependencies),
        Expr::Block { block } => collect_assumption_usage_block(block, out, dependencies),
        Expr::Bin { op, lhs, rhs, .. } => {
            if let Some(symbol) = bitwise_symbol(*op) {
                out.bitwise_ops.insert(symbol.to_string());
            }
            collect_assumption_usage_expr(lhs, out, dependencies);
            collect_assumption_usage_expr(rhs, out, dependencies);
        }
        Expr::Call {
            callee,
            type_args,
            args,
            ..
        } => {
            for ty in type_args {
                collect_unsigned_from_type(ty, &mut out.unsigned_types);
            }
            for arg in args {
                collect_assumption_usage_expr(arg, out, dependencies);
            }
            if matches!(callee.as_str(), "U8" | "U64" | "U128" | "U256") {
                out.unsigned_types.insert(callee.clone());
            }
            if callee.starts_with("std::u64::") {
                out.unsigned_types.insert("U64".to_string());
            }
            if callee.starts_with("std::u128::") {
                out.unsigned_types.insert("U128".to_string());
            }
            if callee.starts_with("std::u256::") {
                out.unsigned_types.insert("U256".to_string());
            }
            if is_bitwise_assumption_intrinsic(callee) {
                out.bitwise_ops.insert(callee.clone());
            }
            if is_crypto_assumption_intrinsic(callee) {
                out.crypto_intrinsics.insert(callee.clone());
            }
            if dependencies.non_proved_primitives.contains(callee) {
                out.primitive_dependencies.insert(callee.clone());
            }
            if dependencies.external_dependencies.contains(callee) {
                out.external_dependencies.insert(callee.clone());
            }
        }
        Expr::Return { expr, .. } | Expr::Unary { expr, .. } | Expr::Try { expr, .. } => {
            collect_assumption_usage_expr(expr, out, dependencies);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_assumption_usage_expr(scrutinee, out, dependencies);
            for arm in arms {
                collect_assumption_usage_expr(&arm.expr, out, dependencies);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_assumption_usage_expr(cond, out, dependencies);
            collect_assumption_usage_expr(then_br, out, dependencies);
            collect_assumption_usage_expr(else_br, out, dependencies);
        }
        Expr::Index { base, index, .. } => {
            collect_assumption_usage_expr(base, out, dependencies);
            collect_assumption_usage_expr(index, out, dependencies);
        }
        Expr::Lambda { params, body, .. } => {
            for param in params {
                collect_unsigned_from_type(&param.ty, &mut out.unsigned_types);
            }
            collect_assumption_usage_expr(body, out, dependencies);
        }
    }
}

fn bitwise_symbol(op: BinOp) -> Option<&'static str> {
    match op {
        BinOp::Shl => Some("<<"),
        BinOp::Shr => Some(">>"),
        BinOp::BitAnd => Some("&"),
        BinOp::BitXor => Some("^"),
        BinOp::BitOr => Some("|"),
        _ => None,
    }
}

fn is_crypto_assumption_intrinsic(callee: &str) -> bool {
    matches!(
        callee,
        "std::crypto::hash" | "std::crypto::hmac" | "std::crypto::verify" | "std::bytes::eq_ct"
    )
}

fn is_bitwise_assumption_intrinsic(callee: &str) -> bool {
    matches!(
        callee,
        "std::u64::rotl"
            | "std::u64::rotr"
            | "std::u64::to_bytes_le"
            | "std::u64::to_bytes_be"
            | "std::u64::from_bytes_le"
            | "std::u64::from_bytes_be"
    )
}

fn is_u64_covered_bitwise_symbol(symbol: &str) -> bool {
    matches!(
        symbol,
        "&" | "|" | "^" | "<<" | ">>" | "std::u64::rotl" | "std::u64::rotr"
    )
}

pub(super) fn u64_bounds_smt(term: &str) -> String {
    format!("(and (<= 0 {term}) (<= {term} {U64_MAX_SMT}))")
}

pub(super) fn append_smt_bounds(base: String, bounds: &[String]) -> String {
    if bounds.is_empty() {
        base
    } else {
        format!("(and {} {})", base, bounds.join(" "))
    }
}

pub(super) fn premise_from_obligation(obligation: &RefinementObligation) -> RefinementPremise {
    RefinementPremise {
        alias: obligation.alias.clone(),
        binder: obligation.binder.clone(),
        substitution: snapshot_expr(&obligation.substitution),
        predicate: snapshot_expr(&obligation.predicate),
        attachment: obligation.attachment.clone(),
    }
}

pub(super) fn u64_bitvector_bindings_smt(param_names: &[String]) -> String {
    if param_names.is_empty() {
        return String::new();
    }
    let mut lines = Vec::new();
    lines.push("; U64 bitvector bridge (phase 25.1.17)".to_string());
    for name in param_names {
        let bv_symbol = format!("clg.u64.bv.{}", name);
        lines.push(format!("(declare-const {} (_ BitVec 64))", bv_symbol));
        lines.push(format!(
            "(assert (= {} (clg.u64.to_int {})))",
            name, bv_symbol
        ));
    }
    lines.join("\n")
}

pub(super) fn obligation_uses_only_symbols(
    obligation: &RefinementObligation,
    allowed_symbols: &HashSet<String>,
) -> bool {
    let mut free_symbols = BTreeSet::new();
    let bound = HashSet::new();
    collect_free_symbols_expr(&obligation.substitution, &bound, &mut free_symbols);
    collect_free_symbols_expr(&obligation.predicate, &bound, &mut free_symbols);
    free_symbols
        .iter()
        .all(|symbol| allowed_symbols.contains(symbol))
}

fn collect_free_symbols_expr(
    expr: &Expr,
    bound_symbols: &HashSet<String>,
    out: &mut BTreeSet<String>,
) {
    match expr {
        Expr::Int(..) | Expr::Bool(..) | Expr::String(..) => {}
        Expr::Var(name, _) => {
            if !bound_symbols.contains(name) {
                out.insert(name.clone());
            }
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_free_symbols_expr(elem, bound_symbols, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_free_symbols_expr(&field.expr, bound_symbols, out);
            }
        }
        Expr::FieldAccess { base, .. } => collect_free_symbols_expr(base, bound_symbols, out),
        Expr::Index { base, index, .. } => {
            collect_free_symbols_expr(base, bound_symbols, out);
            collect_free_symbols_expr(index, bound_symbols, out);
        }
        Expr::Unary { expr, .. } | Expr::Try { expr, .. } | Expr::Return { expr, .. } => {
            collect_free_symbols_expr(expr, bound_symbols, out);
        }
        Expr::Bin { lhs, rhs, .. } => {
            collect_free_symbols_expr(lhs, bound_symbols, out);
            collect_free_symbols_expr(rhs, bound_symbols, out);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                collect_free_symbols_expr(arg, bound_symbols, out);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_free_symbols_expr(cond, bound_symbols, out);
            collect_free_symbols_expr(then_br, bound_symbols, out);
            collect_free_symbols_expr(else_br, bound_symbols, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_free_symbols_expr(scrutinee, bound_symbols, out);
            for arm in arms {
                let mut arm_bound = bound_symbols.clone();
                match &arm.pat {
                    clg_ast::MatchPat::Some(name)
                    | clg_ast::MatchPat::Ok(name)
                    | clg_ast::MatchPat::Err(name) => {
                        arm_bound.insert(name.clone());
                    }
                    clg_ast::MatchPat::None
                    | clg_ast::MatchPat::Wildcard
                    | clg_ast::MatchPat::EnumVariant { .. } => {}
                }
                collect_free_symbols_expr(&arm.expr, &arm_bound, out);
            }
        }
        Expr::Block { block } => {
            let mut local_bound = bound_symbols.clone();
            for stmt in &block.statements {
                match stmt {
                    Stmt::Let { name, expr, .. } => {
                        collect_free_symbols_expr(expr, &local_bound, out);
                        local_bound.insert(name.clone());
                    }
                    Stmt::Expr { expr, .. } => collect_free_symbols_expr(expr, &local_bound, out),
                    Stmt::While {
                        cond,
                        invariant,
                        variant,
                        body,
                        ..
                    } => {
                        collect_free_symbols_expr(cond, &local_bound, out);
                        collect_free_symbols_expr(invariant, &local_bound, out);
                        if let Some(variant_expr) = variant {
                            collect_free_symbols_expr(variant_expr, &local_bound, out);
                        }
                        collect_free_symbols_block(body, &local_bound, out);
                    }
                }
            }
            if let Some(tail) = &block.tail {
                collect_free_symbols_expr(tail, &local_bound, out);
            }
        }
        Expr::Lambda { params, body, .. } => {
            let mut lambda_bound = bound_symbols.clone();
            for param in params {
                lambda_bound.insert(param.name.clone());
            }
            collect_free_symbols_expr(body, &lambda_bound, out);
        }
    }
}

fn collect_free_symbols_block(
    block: &Block,
    bound_symbols: &HashSet<String>,
    out: &mut BTreeSet<String>,
) {
    let mut local_bound = bound_symbols.clone();
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                collect_free_symbols_expr(expr, &local_bound, out);
                local_bound.insert(name.clone());
            }
            Stmt::Expr { expr, .. } => collect_free_symbols_expr(expr, &local_bound, out),
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                collect_free_symbols_expr(cond, &local_bound, out);
                collect_free_symbols_expr(invariant, &local_bound, out);
                if let Some(variant_expr) = variant {
                    collect_free_symbols_expr(variant_expr, &local_bound, out);
                }
                collect_free_symbols_block(body, &local_bound, out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_free_symbols_expr(tail, &local_bound, out);
    }
}

pub(super) fn linear_branch_post_smt(scope: &str, idx: usize, vars: usize) -> (String, String) {
    linear_state_post_smt(scope, idx, vars, "then", "else")
}

pub(super) fn linear_loop_post_smt(scope: &str, idx: usize, vars: usize) -> (String, String) {
    linear_state_post_smt(scope, idx, vars, "before", "after")
}

fn linear_state_post_smt(
    scope: &str,
    idx: usize,
    vars: usize,
    left_label: &str,
    right_label: &str,
) -> (String, String) {
    if vars == 0 {
        return (String::new(), "true".to_string());
    }

    let mut declarations = Vec::new();
    let mut checks = Vec::new();
    for var_idx in 0..vars {
        let left = format!("cl.linear.{}.{}.{}.{}", scope, idx, var_idx, left_label);
        let right = format!("cl.linear.{}.{}.{}.{}", scope, idx, var_idx, right_label);
        declarations.push(format!("(declare-const {} Int)", left));
        declarations.push(format!("(declare-const {} Int)", right));
        checks.push(format!("(= {} {})", left, right));
        checks.push(format!("(>= {} 0)", left));
        checks.push(format!("(>= {} 0)", right));
    }
    let post = if checks.len() == 1 {
        checks[0].clone()
    } else {
        format!("(and {})", checks.join(" "))
    };
    (declarations.join("\n"), post)
}
