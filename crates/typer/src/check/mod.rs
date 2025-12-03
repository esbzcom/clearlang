mod expr;
mod intrinsics;

pub(crate) use self::expr::show_ty;
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
use clg_ast::{Block, Effect, Expr, Func, Param, ParamKind, Program, Span, Stmt, Type};
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
struct AliasDef {
    base: Type,
    predicate: Expr,
    binder: Option<String>,
    span: Span,
}

type AliasMap = HashMap<String, AliasDef>;

fn ensure_no_resource_collections(ty: &Type, span: Option<Span>) -> Result<()> {
    if let Some(offending) = find_resource_collection(ty) {
        return Err(TyperError::resource_in_collection(offending, span).into());
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
    let alias_map = build_alias_map(ast)?;
    let mut fns: HashMap<&str, FnSig> = HashMap::new();

    let builtins = builtin_sigs();
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
                    params: resolve_params(&f.params, &alias_map)?,
                    ret: resolve_aliases(&f.ret, &alias_map, &mut Vec::new())?,
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
    let mut fn_indices: HashMap<&str, u32> = HashMap::new();
    for (i, f) in ast.funcs.iter().enumerate() {
        fn_indices.insert(f.name.as_str(), i as u32);
    }
    // Stable intrinsic order
    let intrinsic_order = ["std::str::len", "std::str::eq", "std::str::concat"];
    let mut intrinsic_defs: Vec<clg_ir::Function> = Vec::new();
    for name in intrinsic_order.iter() {
        if used_intrinsics.contains(*name) {
            let idx = (ast.funcs.len() + intrinsic_defs.len()) as u32;
            fn_indices.insert(name, idx);
            // Define IR function signature for the intrinsic
            let (params, ret) = match *name {
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

    let mut module = Module::default();
    for f in &ast.funcs {
        module.funcs.push(lower_func(f, &fns, &fn_indices)?);
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
    let alias_map = build_alias_map(ast)?;
    let mut fns: HashMap<&str, FnSig> = HashMap::new();

    let builtins = builtin_sigs();
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
                    params: resolve_params(&f.params, &alias_map)?,
                    ret: resolve_aliases(&f.ret, &alias_map, &mut Vec::new())?,
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
    let alias_map = build_alias_map(ast)?;
    let mut fns: HashMap<&str, FnSig> = HashMap::new();

    let builtins = builtin_sigs();
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
                    params: resolve_params(&f.params, &alias_map)?,
                    ret: resolve_aliases(&f.ret, &alias_map, &mut Vec::new())?,
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
    let mut fn_indices: HashMap<&str, u32> = HashMap::new();
    for (i, f) in ast.funcs.iter().enumerate() {
        fn_indices.insert(f.name.as_str(), i as u32);
    }
    let intrinsic_order = ["std::str::len", "std::str::eq", "std::str::concat"];
    let mut intrinsic_defs: Vec<clg_ir::Function> = Vec::new();
    for name in intrinsic_order.iter() {
        if used_intrinsics.contains(*name) {
            let idx = (ast.funcs.len() + intrinsic_defs.len()) as u32;
            fn_indices.insert(name, idx);
            let (params, ret) = match *name {
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

    let mut module = Module::default();
    for f in &ast.funcs {
        module.funcs.push(lower_func(f, &fns, &fn_indices)?);
    }
    module.funcs.extend(intrinsic_defs);
    let vcs = generate_vcs(ast);
    Ok(TypecheckOutput { ir: module, vcs })
}

fn check_func<'a>(f: &'a Func, fns: &HashMap<&'a str, FnSig>, aliases: &AliasMap) -> Result<()> {
    let mut env: HashMap<&str, LocalBinding> = HashMap::new();
    let ret_ty = resolve_aliases(&f.ret, aliases, &mut Vec::new())?;
    env.insert(
        RETURN_KEY,
        LocalBinding {
            ty: ret_ty.clone(),
            kind: ParamKind::Borrow,
        },
    );
    let mut tracker = ResourceTracker::new();
    for p in &f.params {
        let resolved_ty = resolve_aliases(&p.ty, aliases, &mut Vec::new())?;
        if env
            .insert(
                p.name.as_str(),
                LocalBinding {
                    ty: resolved_ty.clone(),
                    kind: p.kind,
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_parameter(&p.name).into());
        }
        tracker.register_param(p.name.as_str(), p.kind, &resolved_ty);
    }

    if let Effect::Io = f.effect {
        return Err(TyperError::effect_not_supported(f.effect).into());
    }
    let allowed_effect = level_from_effect(f.effect);

    for req in &f.requires {
        let mut req_tracker = tracker.clone();
        let ty = type_of(&req.expr, &env, &mut req_tracker, fns, 0)?;
        if ty != Type::Bool {
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
        let ty = type_of(&ens.expr, &ensure_env, &mut ensure_tracker, fns, 0)?;
        if ty != Type::Bool {
            return Err(TyperError::contract_not_bool("ensure", ty, ens.span).into());
        }
        max_effect(&ens.expr, fns, EffectLevel::Pure)?;
    }

    let body_ty = type_of(&f.body, &env, &mut tracker, fns, 0)?;
    if body_ty != ret_ty {
        let sp = expr_span(&f.body);
        return Err(TyperError::return_type_mismatch(ret_ty.clone(), body_ty, sp).into());
    }
    if matches!(ret_ty, Type::Resource(_)) {
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
    let mut aliases: AliasMap = HashMap::new();
    // Collect resource names for conflict checks
    let mut resource_names: HashSet<&str> = HashSet::new();
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
        let mut visited = Vec::new();
        let resolved_base = resolve_aliases(&alias.base, &aliases, &mut visited)?;
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

fn resolve_params(params: &[Param], aliases: &AliasMap) -> Result<Vec<Param>> {
    let mut resolved = Vec::with_capacity(params.len());
    for p in params {
        let mut visiting = Vec::new();
        let ty = resolve_aliases(&p.ty, aliases, &mut visiting)?;
        resolved.push(Param {
            kind: p.kind,
            name: p.name.clone(),
            ty,
        });
    }
    Ok(resolved)
}

fn validate_alias_predicates(aliases: &AliasMap, fns: &HashMap<&str, FnSig>) -> Result<()> {
    for (name, def) in aliases {
        let mut env: HashMap<&str, LocalBinding> = HashMap::new();
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
        let pred_ty = type_of(&def.predicate, &env, &mut tracker, fns, 0)?;
        if pred_ty != Type::Bool {
            return Err(TyperError::alias_predicate_not_bool(name.as_str(), def.span).into());
        }
        max_effect(&def.predicate, fns, EffectLevel::Pure)?;
    }
    Ok(())
}
