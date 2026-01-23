mod expr;
mod intrinsics;

pub(crate) use self::expr::{infer_expr_type, show_ty};
use self::expr::{consume_var_expr, expr_span, max_effect, type_of, ResourceTracker};
use self::intrinsics::collect_used_intrinsics;
use crate::builtins::builtin_sigs;
use crate::errors::TyperError;
use crate::guards::{
    collect_mut_calls, collect_mut_guards, guard_callee_for_kind, MutCall, MutGuardKey,
};
use crate::lower::lower_func;
use crate::vc::{generate_vcs, VerificationCondition};
use anyhow::{Context, Result};
use clg_ast::{
    BinOp, Block, Effect, Expr, Func, Param, ParamKind, Program, Span, Stmt, Type, UnaryOp,
};
use clg_ir::Module;
use std::collections::{HashMap, HashSet};

const RETURN_KEY: &str = "$return";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum EffectLevel {
    Pure,
    Mut,
    Io,
}

impl EffectLevel {
    fn join(self, other: EffectLevel) -> EffectLevel {
        if self >= other {
            self
        } else {
            other
        }
    }
}

pub(super) fn effect_label(level: EffectLevel) -> &'static str {
    match level {
        EffectLevel::Pure => "pure",
        EffectLevel::Mut => "mut",
        EffectLevel::Io => "io",
    }
}

#[derive(Clone)]
pub(crate) struct AliasDef {
    base: Type,
    predicate: Expr,
    binder: Option<String>,
    span: Span,
}

pub(crate) type AliasMap = HashMap<String, AliasDef>;

fn ensure_no_resource_collections(ty: &Type, span: Option<Span>) -> Result<()> {
    if let Some(offending) = find_resource_collection(ty) {
        return Err(TyperError::resource_in_collection(offending, span).into());
    }
    Ok(())
}

fn ensure_supported_type(ty: &Type, span: Option<Span>) -> Result<()> {
    match ty {
        Type::U128 | Type::U256 => Ok(()),
        Type::Option(inner) | Type::List(inner) | Type::Set(inner) => {
            ensure_supported_type(inner, span)
        }
        Type::Result(ok, err) | Type::Map(ok, err) => {
            ensure_supported_type(ok, span)?;
            ensure_supported_type(err, span)
        }
        _ => Ok(()),
    }
}

fn validate_supported_types(program: &Program) -> Result<()> {
    for res in &program.resources {
        for field in &res.fields {
            ensure_supported_type(&field.ty, Some(field.span))?;
        }
    }
    for alias in &program.refined_aliases {
        ensure_supported_type(&alias.base, Some(alias.span))?;
    }
    for func in &program.funcs {
        ensure_supported_type(&func.ret, None)?;
        for param in &func.params {
            ensure_supported_type(&param.ty, None)?;
        }
    }
    Ok(())
}

fn find_resource_collection(ty: &Type) -> Option<Type> {
    match ty {
        Type::List(inner) | Type::Set(inner) => {
            if contains_resource(inner) {
                Some(ty.clone())
            } else {
                None
            }
        }
        Type::Map(key, val) => {
            if contains_resource(key) || contains_resource(val) {
                Some(ty.clone())
            } else {
                None
            }
        }
        Type::Option(inner) => find_resource_collection(inner),
        Type::Result(ok, err) => {
            find_resource_collection(ok).or_else(|| find_resource_collection(err))
        }
        _ => None,
    }
}

fn contains_resource(ty: &Type) -> bool {
    match ty {
        Type::Resource(_) => true,
        Type::Option(inner) | Type::List(inner) | Type::Set(inner) => contains_resource(inner),
        Type::Result(ok, err) | Type::Map(ok, err) => {
            contains_resource(ok) || contains_resource(err)
        }
        _ => false,
    }
}

fn contains_named_resource(ty: &Type, resource_names: &HashSet<&str>) -> bool {
    match ty {
        Type::Resource(name) => resource_names.contains(name.as_str()),
        Type::Option(inner) | Type::List(inner) | Type::Set(inner) => {
            contains_named_resource(inner, resource_names)
        }
        Type::Result(ok, err) | Type::Map(ok, err) => {
            contains_named_resource(ok, resource_names)
                || contains_named_resource(err, resource_names)
        }
        _ => false,
    }
}

fn validate_no_resource_collections(program: &Program) -> Result<()> {
    for res in &program.resources {
        for field in &res.fields {
            ensure_no_resource_collections(&field.ty, Some(field.span))?;
        }
    }
    for func in &program.funcs {
        ensure_no_resource_collections(&func.ret, None)?;
        for param in &func.params {
            ensure_no_resource_collections(&param.ty, None)?;
        }
    }
    Ok(())
}

fn enforce_totality(func: &Func) -> Result<()> {
    if matches!(func.effect, Effect::None | Effect::Pure) {
        check_totality_expr(&func.body, &func.name)?;
    }
    Ok(())
}

fn check_totality_expr(expr: &Expr, self_name: &str) -> Result<()> {
    match expr {
        Expr::Block { block } => check_totality_block(block, self_name)?,
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            check_totality_expr(cond, self_name)?;
            check_totality_expr(then_br, self_name)?;
            check_totality_expr(else_br, self_name)?;
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            check_totality_expr(scrutinee, self_name)?;
            for arm in arms {
                check_totality_expr(&arm.expr, self_name)?;
            }
        }
        Expr::Unary { expr, .. } | Expr::Return { expr, .. } | Expr::Try { expr, .. } => {
            check_totality_expr(expr, self_name)?;
        }
        Expr::Bin { lhs, rhs, .. } => {
            check_totality_expr(lhs, self_name)?;
            check_totality_expr(rhs, self_name)?;
        }
        Expr::Call { callee, args, span } => {
            if callee == self_name {
                return Err(TyperError::recursion_requires_measure(callee, *span).into());
            }
            for arg in args {
                check_totality_expr(arg, self_name)?;
            }
        }
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
    }
    Ok(())
}

fn check_totality_block(block: &Block, self_name: &str) -> Result<()> {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                check_totality_expr(expr.as_ref(), self_name)?;
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => {
                if variant.is_none() {
                    return Err(TyperError::while_variant_required(*span).into());
                }
                check_totality_expr(cond.as_ref(), self_name)?;
                check_totality_expr(invariant.as_ref(), self_name)?;
                if let Some(v) = variant {
                    check_totality_expr(v.as_ref(), self_name)?;
                    if let Expr::Int(_, sp) = v.as_ref() {
                        return Err(TyperError::variant_not_decreasing(*sp).into());
                    }
                }
                check_totality_block(body.as_ref(), self_name)?;
            }
        }
    }
    if let Some(tail) = &block.tail {
        check_totality_expr(tail.as_ref(), self_name)?;
    }
    Ok(())
}
fn level_from_effect(effect: Effect) -> EffectLevel {
    match effect {
        Effect::None | Effect::Pure => EffectLevel::Pure,
        Effect::Mut => EffectLevel::Mut,
        Effect::Io => EffectLevel::Io,
    }
}

#[derive(Clone)]
pub(crate) struct FnSig {
    pub params: Vec<Param>,
    pub ret: Type,
    pub effect: EffectLevel,
}

#[derive(Clone)]
pub(crate) struct LocalBinding {
    pub ty: Type,
    #[allow(dead_code)]
    pub kind: ParamKind,
}

pub struct TypecheckOutput {
    pub ir: Module,
    pub vcs: Vec<VerificationCondition>,
}

pub fn check_with_vcs(ast: &Program) -> Result<TypecheckOutput> {
    if std::env::var("CLG_DISABLE_TOTALITY").is_ok() {
        return fast_path_without_totality(ast);
    }
    validate_no_resource_collections(ast)?;
    validate_supported_types(ast)?;
    let alias_map = build_alias_map(ast)?;
    let builtins = builtin_sigs();
    let mut fns: HashMap<&str, FnSig> = HashMap::with_capacity(builtins.len() + ast.funcs.len());
    for (name, params, ret, eff) in &builtins {
        fns.insert(
            name.as_str(),
            FnSig {
                params: params.clone(),
                ret: ret.clone(),
                effect: level_from_effect(*eff),
            },
        );
    }

    for f in &ast.funcs {
        if alias_map.contains_key(f.name.as_str()) {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
        if fns
            .insert(
                f.name.as_str(),
                FnSig {
                    params: f.params.clone(),
                    ret: f.ret.clone(),
                    effect: level_from_effect(f.effect),
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    validate_alias_predicates(&alias_map, &fns)?;

    for f in &ast.funcs {
        check_func(f, &fns, &alias_map).with_context(|| format!("in function `{}`", f.name))?;
    }
    for f in &ast.funcs {
        enforce_totality(f)?;
    }

    // Collect used intrinsics
    let used_intrinsics = collect_used_intrinsics(ast);

    // Order of function indices: all user-defined first, then intrinsics used (stable order)
    let mut fn_indices: HashMap<&str, u32> =
        HashMap::with_capacity(ast.funcs.len() + used_intrinsics.len());
    for (i, f) in ast.funcs.iter().enumerate() {
        fn_indices.insert(f.name.as_str(), i as u32);
    }
    // Stable intrinsic order
    let intrinsic_order = [
        "std::bytes::len",
        "std::bytes::eq",
        "std::bytes::concat",
        "std::bytes::from_string",
        "std::bytes::to_string",
        "std::wasi::print",
        "std::env::time",
        "std::env::random",
        "std::str::len",
        "std::str::eq",
        "std::str::concat",
    ];
    let mut intrinsic_defs: Vec<clg_ir::Function> = Vec::with_capacity(used_intrinsics.len());
    for name in intrinsic_order.iter() {
        if used_intrinsics.contains(*name) {
            let idx = (ast.funcs.len() + intrinsic_defs.len()) as u32;
            fn_indices.insert(name, idx);
            // Define IR function signature for the intrinsic
            let (params, ret) = match *name {
                "std::bytes::len" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::bytes::eq" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::bytes::concat" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Int),
                ),
                "std::bytes::from_string" => {
                    (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int))
                }
                "std::bytes::to_string" => {
                    (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int))
                }
                "std::wasi::print" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::env::time" => (vec![], Some(clg_ir::IrType::Int)),
                "std::env::random" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::str::len" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::str::eq" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::str::concat" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Int),
                ),
                _ => (vec![], None),
            };
            intrinsic_defs.push(clg_ir::Function {
                name: (*name).to_string(),
                params,
                ret,
                body: vec![],
            });
        }
    }

    let mut module = Module {
        funcs: Vec::with_capacity(ast.funcs.len() + intrinsic_defs.len()),
    };
    for f in &ast.funcs {
        module
            .funcs
            .push(lower_func(f, &fns, &fn_indices, &alias_map)?);
    }
    // Append intrinsic function declarations at the end
    module.funcs.extend(intrinsic_defs);
    let vcs = generate_vcs(ast);
    Ok(TypecheckOutput { ir: module, vcs })
}

pub fn check(ast: &Program) -> Result<Module> {
    Ok(check_with_vcs(ast)?.ir)
}

pub fn type_check_only(ast: &Program) -> Result<()> {
    if std::env::var("CLG_DISABLE_TOTALITY").is_ok() {
        return fast_path_without_totality(ast).map(|_| ());
    }
    validate_no_resource_collections(ast)?;
    validate_supported_types(ast)?;
    let alias_map = build_alias_map(ast)?;
    let builtins = builtin_sigs();
    let mut fns: HashMap<&str, FnSig> = HashMap::with_capacity(builtins.len() + ast.funcs.len());
    for (name, params, ret, eff) in &builtins {
        fns.insert(
            name.as_str(),
            FnSig {
                params: params.clone(),
                ret: ret.clone(),
                effect: level_from_effect(*eff),
            },
        );
    }

    for f in &ast.funcs {
        if alias_map.contains_key(f.name.as_str()) {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
        if fns
            .insert(
                f.name.as_str(),
                FnSig {
                    params: f.params.clone(),
                    ret: f.ret.clone(),
                    effect: level_from_effect(f.effect),
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    validate_alias_predicates(&alias_map, &fns)?;

    for f in &ast.funcs {
        check_func(f, &fns, &alias_map).with_context(|| format!("in function `{}`", f.name))?;
    }
    for f in &ast.funcs {
        enforce_totality(f)?;
    }
    Ok(())
}

fn fast_path_without_totality(ast: &Program) -> Result<TypecheckOutput> {
    validate_no_resource_collections(ast)?;
    validate_supported_types(ast)?;
    let alias_map = build_alias_map(ast)?;
    let builtins = builtin_sigs();
    let mut fns: HashMap<&str, FnSig> = HashMap::with_capacity(builtins.len() + ast.funcs.len());
    for (name, params, ret, eff) in &builtins {
        fns.insert(
            name.as_str(),
            FnSig {
                params: params.clone(),
                ret: ret.clone(),
                effect: level_from_effect(*eff),
            },
        );
    }

    for f in &ast.funcs {
        if alias_map.contains_key(f.name.as_str()) {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
        if fns
            .insert(
                f.name.as_str(),
                FnSig {
                    params: f.params.clone(),
                    ret: f.ret.clone(),
                    effect: level_from_effect(f.effect),
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    validate_alias_predicates(&alias_map, &fns)?;

    for f in &ast.funcs {
        check_func(f, &fns, &alias_map).with_context(|| format!("in function `{}`", f.name))?;
    }

    let used_intrinsics = collect_used_intrinsics(ast);
    let mut fn_indices: HashMap<&str, u32> =
        HashMap::with_capacity(ast.funcs.len() + used_intrinsics.len());
    for (i, f) in ast.funcs.iter().enumerate() {
        fn_indices.insert(f.name.as_str(), i as u32);
    }
    let intrinsic_order = [
        "std::bytes::len",
        "std::bytes::eq",
        "std::bytes::concat",
        "std::bytes::from_string",
        "std::bytes::to_string",
        "std::wasi::print",
        "std::env::time",
        "std::env::random",
        "std::str::len",
        "std::str::eq",
        "std::str::concat",
    ];
    let mut intrinsic_defs: Vec<clg_ir::Function> = Vec::with_capacity(used_intrinsics.len());
    for name in intrinsic_order.iter() {
        if used_intrinsics.contains(*name) {
            let idx = (ast.funcs.len() + intrinsic_defs.len()) as u32;
            fn_indices.insert(name, idx);
            let (params, ret) = match *name {
                "std::bytes::len" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::bytes::eq" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::bytes::concat" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Int),
                ),
                "std::bytes::from_string" => {
                    (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int))
                }
                "std::bytes::to_string" => {
                    (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int))
                }
                "std::wasi::print" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::env::time" => (vec![], Some(clg_ir::IrType::Int)),
                "std::env::random" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::str::len" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::str::eq" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::str::concat" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Int),
                ),
                _ => (vec![], None),
            };
            intrinsic_defs.push(clg_ir::Function {
                name: (*name).to_string(),
                params,
                ret,
                body: vec![],
            });
        }
    }

    let mut module = Module {
        funcs: Vec::with_capacity(ast.funcs.len() + intrinsic_defs.len()),
    };
    for f in &ast.funcs {
        module
            .funcs
            .push(lower_func(f, &fns, &fn_indices, &alias_map)?);
    }
    module.funcs.extend(intrinsic_defs);
    let vcs = generate_vcs(ast);
    Ok(TypecheckOutput { ir: module, vcs })
}

fn check_func<'a>(f: &'a Func, fns: &HashMap<&'a str, FnSig>, aliases: &AliasMap) -> Result<()> {
    let mut env: HashMap<&str, LocalBinding> = HashMap::with_capacity(f.params.len() + 1);
    let ret_ty = f.ret.clone();
    env.insert(
        RETURN_KEY,
        LocalBinding {
            ty: ret_ty.clone(),
            kind: ParamKind::Borrow,
        },
    );
    let mut tracker = ResourceTracker::with_capacity(f.params.len());
    for p in &f.params {
        let resolved_ty = base_type(&p.ty, aliases)?;
        if env
            .insert(
                p.name.as_str(),
                LocalBinding {
                    ty: p.ty.clone(),
                    kind: p.kind,
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_parameter(&p.name).into());
        }
        tracker.register_param(p.name.as_str(), p.kind, &resolved_ty);
    }

    let allowed_effect = level_from_effect(f.effect);

    for req in &f.requires {
        let mut req_tracker = tracker.clone();
        let ty = type_of(&req.expr, &env, &mut req_tracker, fns, aliases, 0)?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("require", ty, req.span).into());
        }
        max_effect(&req.expr, fns, EffectLevel::Pure)?;
    }

    let guard_keys = collect_mut_guards(&f.requires);

    let mut ensure_env = env.clone();
    if !ensure_env.contains_key("result") {
        ensure_env.insert(
            "result",
            LocalBinding {
                ty: f.ret.clone(),
                kind: ParamKind::Borrow,
            },
        );
    }
    for ens in &f.ensures {
        let mut ensure_tracker = tracker.clone();
        let ty = type_of(&ens.expr, &ensure_env, &mut ensure_tracker, fns, aliases, 0)?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("ensure", ty, ens.span).into());
        }
        max_effect(&ens.expr, fns, EffectLevel::Pure)?;
    }

    let body_ty = type_of(&f.body, &env, &mut tracker, fns, aliases, 0)?;
    if !binding_compatible(&ret_ty, &body_ty, aliases)? {
        let sp = expr_span(&f.body);
        let allow_unsigned_literal = expr::literal_can_coerce_unsigned(&ret_ty, &body_ty, &f.body);
        if !allow_unsigned_literal {
            if base_types_match(&ret_ty, &body_ty, aliases)?
                && refinement_loss(&ret_ty, &body_ty, aliases)
            {
                return Err(TyperError::refinement_loss(ret_ty.clone(), body_ty, sp).into());
            }
            return Err(TyperError::return_type_mismatch(ret_ty.clone(), body_ty, sp).into());
        }
    }
    if is_resource_type(&ret_ty, aliases)? {
        consume_var_expr(&mut tracker, &f.body)?;
    }
    tracker.ensure_consumed()?;
    if allowed_effect >= EffectLevel::Mut {
        enforce_mut_guards(&f.body, &guard_keys)?;
    }
    max_effect(&f.body, fns, allowed_effect)?;
    Ok(())
}
fn enforce_mut_guards(expr: &Expr, guards: &HashSet<MutGuardKey>) -> Result<()> {
    let mut calls: Vec<MutCall> = Vec::new();
    collect_mut_calls(expr, &mut calls);
    for call in calls {
        let guard_name = guard_callee_for_kind(call.kind);
        let arg_name = match call.target {
            Some(name) => name,
            None => {
                return Err(TyperError::mut_guard_requires_variable(
                    call.callee.as_str(),
                    guard_name,
                    call.span,
                )
                .into());
            }
        };
        let key = MutGuardKey {
            kind: call.kind,
            target: arg_name.clone(),
        };
        if !guards.contains(&key) {
            return Err(TyperError::mut_guard_missing(
                call.callee.as_str(),
                guard_name,
                &arg_name,
                call.span,
            )
            .into());
        }
    }
    Ok(())
}

fn build_alias_map(program: &Program) -> Result<AliasMap> {
    let mut aliases: AliasMap = HashMap::with_capacity(program.refined_aliases.len());
    // Collect resource names for conflict checks
    let mut resource_names: HashSet<&str> = HashSet::with_capacity(program.resources.len());
    for res in &program.resources {
        resource_names.insert(res.name.as_str());
    }

    for alias in &program.refined_aliases {
        if aliases.contains_key(alias.name.as_str()) {
            return Err(TyperError::duplicate_type(&alias.name, alias.name_span).into());
        }
        if resource_names.contains(alias.name.as_str()) {
            return Err(
                TyperError::type_conflicts_with_resource(&alias.name, alias.name_span).into(),
            );
        }
        let mut visited = Vec::with_capacity(program.refined_aliases.len());
        let resolved_base = resolve_aliases(&alias.base, &aliases, &mut visited)?;
        if contains_named_resource(&resolved_base, &resource_names) {
            return Err(TyperError::refined_resource_not_supported(
                alias.name.as_str(),
                alias.span,
            )
            .into());
        }
        ensure_no_resource_collections(&resolved_base, Some(alias.span))?;

        aliases.insert(
            alias.name.clone(),
            AliasDef {
                base: resolved_base,
                predicate: alias.predicate.clone(),
                binder: alias.binder.clone(),
                span: alias.span,
            },
        );
    }
    Ok(aliases)
}

fn resolve_aliases(ty: &Type, aliases: &AliasMap, visiting: &mut Vec<String>) -> Result<Type> {
    match ty {
        Type::Resource(name) => {
            if let Some(def) = aliases.get(name) {
                if visiting.iter().any(|n| n == name) {
                    return Err(TyperError::cyclic_alias(name, def.span).into());
                }
                visiting.push(name.clone());
                let resolved = resolve_aliases(&def.base, aliases, visiting)?;
                visiting.pop();
                Ok(resolved)
            } else {
                Ok(ty.clone())
            }
        }
        Type::Option(inner) => Ok(Type::Option(Box::new(resolve_aliases(
            inner, aliases, visiting,
        )?))),
        Type::Result(ok, err) => Ok(Type::Result(
            Box::new(resolve_aliases(ok, aliases, visiting)?),
            Box::new(resolve_aliases(err, aliases, visiting)?),
        )),
        Type::List(inner) => Ok(Type::List(Box::new(resolve_aliases(
            inner, aliases, visiting,
        )?))),
        Type::Set(inner) => Ok(Type::Set(Box::new(resolve_aliases(
            inner, aliases, visiting,
        )?))),
        Type::Map(k, v) => Ok(Type::Map(
            Box::new(resolve_aliases(k, aliases, visiting)?),
            Box::new(resolve_aliases(v, aliases, visiting)?),
        )),
        _ => Ok(ty.clone()),
    }
}

fn alias_name<'a>(ty: &'a Type, aliases: &'a AliasMap) -> Option<&'a str> {
    match ty {
        Type::Resource(name) if aliases.contains_key(name.as_str()) => Some(name.as_str()),
        _ => None,
    }
}

fn base_type(ty: &Type, aliases: &AliasMap) -> Result<Type> {
    let mut visiting = Vec::new();
    resolve_aliases(ty, aliases, &mut visiting)
}

fn base_types_match(expected: &Type, actual: &Type, aliases: &AliasMap) -> Result<bool> {
    Ok(base_type(expected, aliases)? == base_type(actual, aliases)?)
}

fn is_resource_type(ty: &Type, aliases: &AliasMap) -> Result<bool> {
    Ok(matches!(base_type(ty, aliases)?, Type::Resource(_)))
}

fn refinement_loss(expected: &Type, actual: &Type, aliases: &AliasMap) -> bool {
    match (expected, actual) {
        (Type::Option(exp), Type::Option(act)) => refinement_loss(exp, act, aliases),
        (Type::Result(exp_ok, exp_err), Type::Result(act_ok, act_err)) => {
            refinement_loss(exp_ok, act_ok, aliases) || refinement_loss(exp_err, act_err, aliases)
        }
        (Type::List(exp), Type::List(act)) => refinement_loss(exp, act, aliases),
        (Type::Set(exp), Type::Set(act)) => refinement_loss(exp, act, aliases),
        (Type::Map(exp_k, exp_v), Type::Map(act_k, act_v)) => {
            refinement_loss(exp_k, act_k, aliases) || refinement_loss(exp_v, act_v, aliases)
        }
        _ => alias_name(actual, aliases).is_some() && alias_name(expected, aliases).is_none(),
    }
}

fn binding_compatible(expected: &Type, actual: &Type, aliases: &AliasMap) -> Result<bool> {
    if expected == actual {
        return Ok(true);
    }
    if alias_name(expected, aliases).is_some() && alias_name(actual, aliases).is_none() {
        return base_types_match(expected, actual, aliases);
    }
    Ok(false)
}

#[derive(Clone, Copy)]
struct IntBound {
    value: i64,
    inclusive: bool,
}

enum IntConstraint {
    Lower(IntBound),
    Upper(IntBound),
    Eq(i64),
    Neq(i64),
}

#[derive(Default)]
struct IntConstraintSet {
    lower: Option<IntBound>,
    upper: Option<IntBound>,
    eq: Option<i64>,
    neq: HashSet<i64>,
    unsat: bool,
}

impl IntConstraintSet {
    fn apply(&mut self, constraint: IntConstraint) {
        if self.unsat {
            return;
        }
        let ok = match constraint {
            IntConstraint::Lower(bound) => self.apply_lower(bound),
            IntConstraint::Upper(bound) => self.apply_upper(bound),
            IntConstraint::Eq(value) => self.apply_eq(value),
            IntConstraint::Neq(value) => self.apply_neq(value),
        };
        if !ok {
            self.unsat = true;
        }
    }

    fn apply_eq(&mut self, value: i64) -> bool {
        if let Some(existing) = self.eq {
            if existing != value {
                return false;
            }
        }
        if !self.satisfies_bounds(value) {
            return false;
        }
        if self.neq.contains(&value) {
            return false;
        }
        self.eq = Some(value);
        true
    }

    fn apply_neq(&mut self, value: i64) -> bool {
        if let Some(eq) = self.eq {
            if eq == value {
                return false;
            }
        }
        self.neq.insert(value);
        if let Some(only) = self.single_value() {
            if only == value {
                return false;
            }
        }
        true
    }

    fn apply_lower(&mut self, bound: IntBound) -> bool {
        if let Some(eq) = self.eq {
            if !satisfies_lower(eq, bound) {
                return false;
            }
            return true;
        }
        match self.lower {
            None => self.lower = Some(bound),
            Some(existing) => {
                if stricter_lower(bound, existing) {
                    self.lower = Some(bound);
                }
            }
        }
        if !self.bounds_allow_values() {
            return false;
        }
        if let Some(only) = self.single_value() {
            if self.neq.contains(&only) {
                return false;
            }
        }
        true
    }

    fn apply_upper(&mut self, bound: IntBound) -> bool {
        if let Some(eq) = self.eq {
            if !satisfies_upper(eq, bound) {
                return false;
            }
            return true;
        }
        match self.upper {
            None => self.upper = Some(bound),
            Some(existing) => {
                if stricter_upper(bound, existing) {
                    self.upper = Some(bound);
                }
            }
        }
        if !self.bounds_allow_values() {
            return false;
        }
        if let Some(only) = self.single_value() {
            if self.neq.contains(&only) {
                return false;
            }
        }
        true
    }

    fn satisfies_bounds(&self, value: i64) -> bool {
        if let Some(lower) = self.lower {
            if !satisfies_lower(value, lower) {
                return false;
            }
        }
        if let Some(upper) = self.upper {
            if !satisfies_upper(value, upper) {
                return false;
            }
        }
        true
    }

    fn bounds_allow_values(&self) -> bool {
        let min = match self.lower {
            None => None,
            Some(bound) => min_from_lower(bound),
        };
        if let Some(val) = min {
            let max = match self.upper {
                None => return true,
                Some(bound) => max_from_upper(bound),
            };
            if let Some(upper_val) = max {
                return val <= upper_val;
            }
            return false;
        }
        if let Some(bound) = self.upper {
            return max_from_upper(bound).is_some();
        }
        true
    }

    fn single_value(&self) -> Option<i64> {
        let lower = self.lower?;
        let upper = self.upper?;
        let min = min_from_lower(lower)?;
        let max = max_from_upper(upper)?;
        if min == max {
            Some(min)
        } else {
            None
        }
    }
}

fn stricter_lower(new: IntBound, existing: IntBound) -> bool {
    new.value > existing.value
        || (new.value == existing.value && !new.inclusive && existing.inclusive)
}

fn stricter_upper(new: IntBound, existing: IntBound) -> bool {
    new.value < existing.value
        || (new.value == existing.value && !new.inclusive && existing.inclusive)
}

fn satisfies_lower(value: i64, bound: IntBound) -> bool {
    value > bound.value || (value == bound.value && bound.inclusive)
}

fn satisfies_upper(value: i64, bound: IntBound) -> bool {
    value < bound.value || (value == bound.value && bound.inclusive)
}

fn min_from_lower(bound: IntBound) -> Option<i64> {
    if bound.inclusive {
        Some(bound.value)
    } else {
        bound.value.checked_add(1)
    }
}

fn max_from_upper(bound: IntBound) -> Option<i64> {
    if bound.inclusive {
        Some(bound.value)
    } else {
        bound.value.checked_sub(1)
    }
}

fn predicate_is_contradiction(expr: &Expr, binder: Option<&str>, base: &Type) -> bool {
    if let Some(value) = eval_const_bool(expr) {
        return !value;
    }
    if !matches!(base, Type::Int) {
        return false;
    }
    let Some(binder) = binder else { return false };
    let mut constraints = IntConstraintSet::default();
    if collect_constraints(expr, binder, &mut constraints) {
        return constraints.unsat;
    }
    false
}

fn collect_constraints(expr: &Expr, binder: &str, constraints: &mut IntConstraintSet) -> bool {
    if constraints.unsat {
        return true;
    }
    if let Some(value) = eval_const_bool(expr) {
        if !value {
            constraints.unsat = true;
        }
        return true;
    }
    match expr {
        Expr::Bin {
            op: BinOp::And,
            lhs,
            rhs,
            ..
        } => {
            let left_ok = collect_constraints(lhs, binder, constraints);
            if constraints.unsat {
                return true;
            }
            let right_ok = collect_constraints(rhs, binder, constraints);
            if constraints.unsat {
                return true;
            }
            left_ok && right_ok
        }
        Expr::Bin { op, lhs, rhs, .. } => {
            if let Some(constraint) = comparison_constraint(*op, lhs, rhs, binder) {
                constraints.apply(constraint);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

fn comparison_constraint(op: BinOp, lhs: &Expr, rhs: &Expr, binder: &str) -> Option<IntConstraint> {
    if let (Some(offset), Some(value)) = (extract_binder_offset(lhs, binder), eval_const_int(rhs)) {
        let target = value.checked_sub(offset)?;
        return constraint_for_comparison(op, target);
    }
    if let (Some(value), Some(offset)) = (eval_const_int(lhs), extract_binder_offset(rhs, binder)) {
        let target = value.checked_sub(offset)?;
        let inverted = invert_comparison(op)?;
        return constraint_for_comparison(inverted, target);
    }
    None
}

fn constraint_for_comparison(op: BinOp, value: i64) -> Option<IntConstraint> {
    match op {
        BinOp::Lt => Some(IntConstraint::Upper(IntBound {
            value,
            inclusive: false,
        })),
        BinOp::Le => Some(IntConstraint::Upper(IntBound {
            value,
            inclusive: true,
        })),
        BinOp::Gt => Some(IntConstraint::Lower(IntBound {
            value,
            inclusive: false,
        })),
        BinOp::Ge => Some(IntConstraint::Lower(IntBound {
            value,
            inclusive: true,
        })),
        BinOp::Eq => Some(IntConstraint::Eq(value)),
        BinOp::Neq => Some(IntConstraint::Neq(value)),
        _ => None,
    }
}

fn invert_comparison(op: BinOp) -> Option<BinOp> {
    match op {
        BinOp::Lt => Some(BinOp::Gt),
        BinOp::Le => Some(BinOp::Ge),
        BinOp::Gt => Some(BinOp::Lt),
        BinOp::Ge => Some(BinOp::Le),
        BinOp::Eq => Some(BinOp::Eq),
        BinOp::Neq => Some(BinOp::Neq),
        _ => None,
    }
}

fn extract_binder_offset(expr: &Expr, binder: &str) -> Option<i64> {
    match expr {
        Expr::Var(name, _) if name == binder => Some(0),
        Expr::Bin {
            op: BinOp::Add,
            lhs,
            rhs,
            ..
        } => {
            if let Some(offset) = extract_binder_offset(lhs, binder) {
                return offset.checked_add(eval_const_int(rhs)?);
            }
            if let Some(offset) = extract_binder_offset(rhs, binder) {
                return offset.checked_add(eval_const_int(lhs)?);
            }
            None
        }
        Expr::Bin {
            op: BinOp::Sub,
            lhs,
            rhs,
            ..
        } => {
            let offset = extract_binder_offset(lhs, binder)?;
            offset.checked_sub(eval_const_int(rhs)?)
        }
        _ => None,
    }
}

fn eval_const_int(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Int(value, _) => Some(*value),
        Expr::Bin { op, lhs, rhs, .. } => {
            let left = eval_const_int(lhs)?;
            let right = eval_const_int(rhs)?;
            match op {
                BinOp::Add => left.checked_add(right),
                BinOp::Sub => left.checked_sub(right),
                BinOp::Mul => left.checked_mul(right),
                BinOp::Div => {
                    if right == 0 {
                        None
                    } else {
                        left.checked_div(right)
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn eval_const_bool(expr: &Expr) -> Option<bool> {
    match expr {
        Expr::Bool(value, _) => Some(*value),
        Expr::Unary {
            op: UnaryOp::Not,
            expr,
            ..
        } => eval_const_bool(expr).map(|value| !value),
        Expr::Bin { op, lhs, rhs, .. } => match op {
            BinOp::And => Some(eval_const_bool(lhs)? && eval_const_bool(rhs)?),
            BinOp::Or => Some(eval_const_bool(lhs)? || eval_const_bool(rhs)?),
            BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let left = eval_const_int(lhs)?;
                let right = eval_const_int(rhs)?;
                Some(match op {
                    BinOp::Eq => left == right,
                    BinOp::Neq => left != right,
                    BinOp::Lt => left < right,
                    BinOp::Le => left <= right,
                    BinOp::Gt => left > right,
                    BinOp::Ge => left >= right,
                    _ => return None,
                })
            }
            _ => None,
        },
        _ => None,
    }
}

fn validate_alias_predicates(aliases: &AliasMap, fns: &HashMap<&str, FnSig>) -> Result<()> {
    for (name, def) in aliases {
        let mut env: HashMap<&str, LocalBinding> =
            HashMap::with_capacity(def.binder.as_ref().map(|_| 1).unwrap_or(0));
        if let Some(binder) = def.binder.as_ref() {
            env.insert(
                binder.as_str(),
                LocalBinding {
                    ty: def.base.clone(),
                    kind: ParamKind::Borrow,
                },
            );
        }
        let mut tracker = ResourceTracker::new();
        let pred_ty = type_of(&def.predicate, &env, &mut tracker, fns, aliases, 0)?;
        if base_type(&pred_ty, aliases)? != Type::Bool {
            return Err(TyperError::alias_predicate_not_bool(name.as_str(), def.span).into());
        }
        if let Err(err) = max_effect(&def.predicate, fns, EffectLevel::Pure) {
            if let Some(typer) = err.downcast_ref::<TyperError>() {
                if typer.code == "T401" {
                    return Err(TyperError::alias_predicate_impure(name.as_str(), def.span).into());
                }
            }
            return Err(err);
        }
        if predicate_is_contradiction(&def.predicate, def.binder.as_deref(), &def.base) {
            return Err(TyperError::alias_predicate_unsat(name.as_str(), def.span).into());
        }
    }
    Ok(())
}
