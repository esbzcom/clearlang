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
pub(crate) struct FnSig<'a> {
    pub params: &'a [Param],
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
    validate_no_resource_collections(ast)?;
    let mut fns: HashMap<&str, FnSig> = HashMap::new();

    let builtins = builtin_sigs();
    for (name, params, ret, eff) in &builtins {
        fns.insert(
            name.as_str(),
            FnSig {
                params: &params[..],
                ret: ret.clone(),
                effect: level_from_effect(*eff),
            },
        );
    }

    for f in &ast.funcs {
        if fns
            .insert(
                f.name.as_str(),
                FnSig {
                    params: &f.params,
                    ret: f.ret.clone(),
                    effect: level_from_effect(f.effect),
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    for f in &ast.funcs {
        check_func(f, &fns).with_context(|| format!("in function `{}`", f.name))?;
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
    validate_no_resource_collections(ast)?;
    let mut fns: HashMap<&str, FnSig> = HashMap::new();

    let builtins = builtin_sigs();
    for (name, params, ret, eff) in &builtins {
        fns.insert(
            name.as_str(),
            FnSig {
                params: &params[..],
                ret: ret.clone(),
                effect: level_from_effect(*eff),
            },
        );
    }

    for f in &ast.funcs {
        if fns
            .insert(
                f.name.as_str(),
                FnSig {
                    params: &f.params,
                    ret: f.ret.clone(),
                    effect: level_from_effect(f.effect),
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    for f in &ast.funcs {
        check_func(f, &fns).with_context(|| format!("in function `{}`", f.name))?;
    }
    for f in &ast.funcs {
        enforce_totality(f)?;
    }
    Ok(())
}

fn check_func<'a>(f: &'a Func, fns: &HashMap<&'a str, FnSig<'a>>) -> Result<()> {
    let mut env: HashMap<&str, LocalBinding> = HashMap::new();
    env.insert(
        RETURN_KEY,
        LocalBinding {
            ty: f.ret.clone(),
            kind: ParamKind::Borrow,
        },
    );
    let mut tracker = ResourceTracker::new();
    for p in &f.params {
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
        tracker.register_param(p.name.as_str(), p.kind, &p.ty);
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
    if body_ty != f.ret {
        let sp = expr_span(&f.body);
        return Err(TyperError::return_type_mismatch(f.ret.clone(), body_ty, sp).into());
    }
    if matches!(f.ret, Type::Resource(_)) {
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
