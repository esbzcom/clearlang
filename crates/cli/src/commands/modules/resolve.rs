use std::collections::{HashMap, HashSet};

use clg_ast::{Block, Expr, Func, MatchArm, MatchPat, Stmt, TraitBound, Type};

use super::{qualify_name, ImportEnv, ModuleUnit, ResolveCtx};

pub(super) fn resolve_program(module: &ModuleUnit, env: &ImportEnv) -> clg_ast::Program {
    let prefix = super::target_prefix(module);
    let local_values = module
        .local_values
        .iter()
        .map(|name| (name.clone(), qualify_name(prefix, name)))
        .collect::<HashMap<_, _>>();
    let local_types = module
        .local_types
        .iter()
        .map(|name| (name.clone(), qualify_name(prefix, name)))
        .collect::<HashMap<_, _>>();

    let ctx = ResolveCtx {
        prefix,
        local_values: &local_values,
        local_types: &local_types,
        imported_values: &env.imported_values,
        imported_types: &env.imported_types,
        module_aliases: &env.module_aliases,
    };

    let mut program = module.program.clone();
    program.module = None;
    program.imports.clear();

    for alias in &mut program.refined_aliases {
        alias.name = qualify_name(prefix, &alias.name);
        let mut params = HashSet::new();
        for name in &alias.type_params {
            params.insert(name.clone());
        }
        resolve_type(&mut alias.base, &ctx, &params);
        resolve_expr(&mut alias.predicate, &ctx, &params);
    }

    for res in &mut program.resources {
        res.name = qualify_name(prefix, &res.name);
        let params = HashSet::new();
        for field in &mut res.fields {
            resolve_type(&mut field.ty, &ctx, &params);
        }
        resolve_block(&mut res.drop_block, &ctx, &params);
    }

    for strukt in &mut program.structs {
        strukt.name = qualify_name(prefix, &strukt.name);
        let params = type_param_set(&strukt.type_params);
        for field in &mut strukt.fields {
            resolve_type(&mut field.ty, &ctx, &params);
        }
    }

    for enm in &mut program.enums {
        enm.name = qualify_name(prefix, &enm.name);
        let params = type_param_set(&enm.type_params);
        for variant in &mut enm.variants {
            for field in &mut variant.fields {
                resolve_type(field, &ctx, &params);
            }
        }
    }

    for tr in &mut program.traits {
        tr.name = qualify_name(prefix, &tr.name);
        let mut params = type_param_set(&tr.type_params);
        params.insert("Self".to_string());
        for method in &mut tr.methods {
            for param in &mut method.params {
                resolve_type(&mut param.ty, &ctx, &params);
            }
            resolve_type(&mut method.ret, &ctx, &params);
        }
    }

    for imp in &mut program.impls {
        imp.trait_name = resolve_type_name(&imp.trait_name, &ctx);
        let mut params = type_param_set(&imp.type_params);
        params.insert("Self".to_string());
        resolve_type(&mut imp.for_type, &ctx, &params);
        for bound in &mut imp.where_bounds {
            resolve_trait_bound(bound, &ctx);
        }
        for method in &mut imp.methods {
            resolve_func(method, &ctx, &params, false);
        }
    }

    for func in &mut program.funcs {
        resolve_func(func, &ctx, &type_param_set(&func.type_params), true);
    }

    program
}

fn resolve_func(func: &mut Func, ctx: &ResolveCtx<'_>, params: &HashSet<String>, rename: bool) {
    if rename {
        func.name = qualify_name(ctx.prefix, &func.name);
    }
    for param in &mut func.params {
        resolve_type(&mut param.ty, ctx, params);
    }
    resolve_type(&mut func.ret, ctx, params);
    for bound in &mut func.where_bounds {
        resolve_trait_bound(bound, ctx);
    }
    for req in &mut func.requires {
        resolve_expr(&mut req.expr, ctx, params);
    }
    resolve_expr(&mut func.body, ctx, params);
    for ens in &mut func.ensures {
        resolve_expr(&mut ens.expr, ctx, params);
    }
}

fn resolve_trait_bound(bound: &mut TraitBound, ctx: &ResolveCtx<'_>) {
    bound.trait_name = resolve_type_name(&bound.trait_name, ctx);
}

fn resolve_block(block: &mut Block, ctx: &ResolveCtx<'_>, params: &HashSet<String>) {
    for stmt in &mut block.statements {
        resolve_stmt(stmt, ctx, params);
    }
    if let Some(tail) = &mut block.tail {
        resolve_expr(tail, ctx, params);
    }
}

fn resolve_stmt(stmt: &mut Stmt, ctx: &ResolveCtx<'_>, params: &HashSet<String>) {
    match stmt {
        Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
            resolve_expr(expr, ctx, params);
        }
        Stmt::While {
            cond,
            invariant,
            variant,
            body,
            ..
        } => {
            resolve_expr(cond, ctx, params);
            resolve_expr(invariant, ctx, params);
            if let Some(v) = variant {
                resolve_expr(v, ctx, params);
            }
            resolve_block(body, ctx, params);
        }
    }
}

fn resolve_expr(expr: &mut Expr, ctx: &ResolveCtx<'_>, params: &HashSet<String>) {
    match expr {
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                resolve_expr(elem, ctx, params);
            }
        }
        Expr::StructLit { name, fields, .. } => {
            *name = resolve_type_name(name, ctx);
            for field in fields {
                resolve_expr(&mut field.expr, ctx, params);
            }
        }
        Expr::FieldAccess { base, .. } => resolve_expr(base, ctx, params),
        Expr::Block { block } => resolve_block(block, ctx, params),
        Expr::Bin { lhs, rhs, .. } => {
            resolve_expr(lhs, ctx, params);
            resolve_expr(rhs, ctx, params);
        }
        Expr::Call { callee, args, .. } => {
            *callee = resolve_callee(callee, ctx);
            for arg in args {
                resolve_expr(arg, ctx, params);
            }
        }
        Expr::Return { expr, .. } | Expr::Unary { expr, .. } | Expr::Try { expr, .. } => {
            resolve_expr(expr, ctx, params);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            resolve_expr(scrutinee, ctx, params);
            for arm in arms {
                resolve_arm(arm, ctx, params);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            resolve_expr(cond, ctx, params);
            resolve_expr(then_br, ctx, params);
            resolve_expr(else_br, ctx, params);
        }
        Expr::Index { base, index, .. } => {
            resolve_expr(base, ctx, params);
            resolve_expr(index, ctx, params);
        }
        Expr::Lambda {
            params: lambda_params,
            body,
            ..
        } => {
            for param in lambda_params {
                resolve_type(&mut param.ty, ctx, params);
            }
            resolve_expr(body, ctx, params);
        }
    }
}

fn resolve_arm(arm: &mut MatchArm, ctx: &ResolveCtx<'_>, params: &HashSet<String>) {
    resolve_pattern(&mut arm.pat, ctx, params);
    resolve_expr(&mut arm.expr, ctx, params);
}

fn resolve_pattern(pat: &mut MatchPat, ctx: &ResolveCtx<'_>, _params: &HashSet<String>) {
    match pat {
        MatchPat::EnumVariant { enum_name, .. } => {
            *enum_name = resolve_type_name(enum_name, ctx);
        }
        MatchPat::Some(_)
        | MatchPat::None
        | MatchPat::Ok(_)
        | MatchPat::Err(_)
        | MatchPat::Wildcard => {}
    }
}

fn resolve_type(ty: &mut Type, ctx: &ResolveCtx<'_>, params: &HashSet<String>) {
    match ty {
        Type::Named { name, args } => {
            if !params.contains(name) && name.as_str() != "Self" {
                *name = resolve_type_name(name, ctx);
            }
            for arg in args {
                resolve_type(arg, ctx, params);
            }
        }
        Type::Option(inner) | Type::Array(inner, _) | Type::Slice(inner) => {
            resolve_type(inner, ctx, params);
        }
        Type::Result(ok, err) | Type::Map(ok, err) => {
            resolve_type(ok, ctx, params);
            resolve_type(err, ctx, params);
        }
        Type::List(inner) | Type::Set(inner) => resolve_type(inner, ctx, params),
        Type::Tuple(elems) => {
            for elem in elems {
                resolve_type(elem, ctx, params);
            }
        }
        Type::Fn {
            params: fn_params,
            ret,
        } => {
            for param in fn_params {
                resolve_type(param, ctx, params);
            }
            resolve_type(ret, ctx, params);
        }
        Type::Int
        | Type::U8
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::Bool
        | Type::String
        | Type::Bytes => {}
    }
}

fn resolve_type_name(name: &str, ctx: &ResolveCtx<'_>) -> String {
    if let Some(idx) = name.find("::") {
        let first = &name[..idx];
        if let Some(module) = ctx.module_aliases.get(first) {
            let rest = &name[idx + 2..];
            return format!("{}::{}", module, rest);
        }
        return name.to_string();
    }
    if let Some(q) = ctx.local_types.get(name) {
        return q.clone();
    }
    if let Some(q) = ctx.imported_types.get(name) {
        return q.clone();
    }
    name.to_string()
}

fn resolve_callee(name: &str, ctx: &ResolveCtx<'_>) -> String {
    if let Some(idx) = name.find("::") {
        let first = &name[..idx];
        if let Some(module) = ctx.module_aliases.get(first) {
            let rest = &name[idx + 2..];
            return format!("{}::{}", module, rest);
        }
        if let Some((enum_path, variant)) = name.rsplit_once("::") {
            let resolved_enum = resolve_type_name(enum_path, ctx);
            if resolved_enum != enum_path {
                return format!("{}::{}", resolved_enum, variant);
            }
        }
        return name.to_string();
    }
    if let Some(q) = ctx.local_values.get(name) {
        return q.clone();
    }
    if let Some(q) = ctx.imported_values.get(name) {
        return q.clone();
    }
    name.to_string()
}

fn type_param_set(params: &[clg_ast::TypeParam]) -> HashSet<String> {
    let mut out = HashSet::with_capacity(params.len());
    for param in params {
        out.insert(param.name.clone());
    }
    out
}
