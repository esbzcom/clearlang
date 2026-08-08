use anyhow::Result;
use clg_ast::{Block, Expr, MatchPat, Stmt, Type};
use std::collections::HashMap;

use super::super::{effect_label, level_from_effect, EffectLevel, FnSig, TraitEnv};
use crate::errors::TyperError;

type FnValueEffects<'a> = HashMap<&'a str, Option<EffectLevel>>;

pub(crate) fn max_effect<'a>(
    e: &'a Expr,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    allowed: EffectLevel,
) -> Result<EffectLevel> {
    let fn_locals: FnValueEffects<'a> = HashMap::new();
    max_effect_with_locals(e, fns, trait_env, allowed, &fn_locals)
}

fn max_effect_with_locals<'a>(
    e: &'a Expr,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    allowed: EffectLevel,
    fn_locals: &FnValueEffects<'a>,
) -> Result<EffectLevel> {
    fn block_effect<'a>(
        block: &'a Block,
        fns: &HashMap<&'a str, FnSig>,
        trait_env: &TraitEnv<'a>,
        allowed: EffectLevel,
        fn_locals: &FnValueEffects<'a>,
    ) -> Result<EffectLevel> {
        let mut eff = EffectLevel::Pure;
        let mut block_locals = fn_locals.clone();
        for stmt in &block.statements {
            match stmt {
                Stmt::Let { name, expr, .. } => {
                    eff = eff.join(max_effect_with_locals(
                        expr.as_ref(),
                        fns,
                        trait_env,
                        allowed,
                        &block_locals,
                    )?);
                    match infer_fn_binding_effect(expr.as_ref(), fns, trait_env, &block_locals)? {
                        Some(binding_effect) => {
                            block_locals.insert(name.as_str(), binding_effect);
                        }
                        None => {
                            block_locals.remove(name.as_str());
                        }
                    }
                }
                Stmt::Expr { expr, .. } => {
                    eff = eff.join(max_effect_with_locals(
                        expr.as_ref(),
                        fns,
                        trait_env,
                        allowed,
                        &block_locals,
                    )?);
                }
                Stmt::While {
                    cond,
                    invariant,
                    variant,
                    body,
                    ..
                } => {
                    eff = eff.join(max_effect_with_locals(
                        cond.as_ref(),
                        fns,
                        trait_env,
                        allowed,
                        &block_locals,
                    )?);
                    eff = eff.join(max_effect_with_locals(
                        invariant.as_ref(),
                        fns,
                        trait_env,
                        allowed,
                        &block_locals,
                    )?);
                    if let Some(v) = variant {
                        eff = eff.join(max_effect_with_locals(
                            v.as_ref(),
                            fns,
                            trait_env,
                            allowed,
                            &block_locals,
                        )?);
                    }
                    eff = eff.join(block_effect(
                        body.as_ref(),
                        fns,
                        trait_env,
                        allowed,
                        &block_locals,
                    )?);
                }
            }
        }
        if let Some(tail) = &block.tail {
            eff = eff.join(max_effect_with_locals(
                tail.as_ref(),
                fns,
                trait_env,
                allowed,
                &block_locals,
            )?);
        }
        Ok(eff)
    }
    match e {
        Expr::Block { block } => block_effect(block, fns, trait_env, allowed, fn_locals),
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {
            Ok(EffectLevel::Pure)
        }
        Expr::Return { expr, .. } => {
            max_effect_with_locals(expr, fns, trait_env, allowed, fn_locals)
        }
        Expr::Unary { expr, .. } => {
            max_effect_with_locals(expr, fns, trait_env, allowed, fn_locals)
        }
        Expr::Bin { lhs, rhs, .. } => {
            let left = max_effect_with_locals(lhs, fns, trait_env, allowed, fn_locals)?;
            let right = max_effect_with_locals(rhs, fns, trait_env, allowed, fn_locals)?;
            Ok(left.join(right))
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            let mut eff = EffectLevel::Pure;
            for elem in elems {
                eff = eff.join(max_effect_with_locals(
                    elem, fns, trait_env, allowed, fn_locals,
                )?);
            }
            Ok(eff)
        }
        Expr::StructLit { fields, .. } => {
            let mut eff = EffectLevel::Pure;
            for field in fields {
                eff = eff.join(max_effect_with_locals(
                    &field.expr,
                    fns,
                    trait_env,
                    allowed,
                    fn_locals,
                )?);
            }
            Ok(eff)
        }
        Expr::FieldAccess { base, .. } => {
            max_effect_with_locals(base, fns, trait_env, allowed, fn_locals)
        }
        Expr::Index { base, index, .. } => {
            let base_eff = max_effect_with_locals(base, fns, trait_env, allowed, fn_locals)?;
            let index_eff = max_effect_with_locals(index, fns, trait_env, allowed, fn_locals)?;
            Ok(base_eff.join(index_eff))
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            let cond_eff = max_effect_with_locals(cond, fns, trait_env, allowed, fn_locals)?;
            let then_eff = max_effect_with_locals(then_br, fns, trait_env, allowed, fn_locals)?;
            let else_eff = max_effect_with_locals(else_br, fns, trait_env, allowed, fn_locals)?;
            Ok(cond_eff.join(then_eff).join(else_eff))
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            let mut eff = max_effect_with_locals(scrutinee, fns, trait_env, allowed, fn_locals)?;
            for arm in arms {
                let mut arm_locals = fn_locals.clone();
                match &arm.pat {
                    MatchPat::Some(name) | MatchPat::Ok(name) | MatchPat::Err(name) => {
                        arm_locals.remove(name.as_str());
                    }
                    MatchPat::EnumVariant { binders, .. } => {
                        for binder in binders {
                            arm_locals.remove(binder.as_str());
                        }
                    }
                    MatchPat::None | MatchPat::Wildcard => {}
                }
                eff = eff.join(max_effect_with_locals(
                    &arm.expr,
                    fns,
                    trait_env,
                    allowed,
                    &arm_locals,
                )?);
            }
            Ok(eff)
        }
        Expr::Try { expr, .. } => max_effect_with_locals(expr, fns, trait_env, allowed, fn_locals),
        Expr::Lambda { params, body, .. } => {
            let mut lambda_locals = fn_locals.clone();
            for param in params {
                if matches!(param.ty, Type::Fn { .. }) {
                    lambda_locals.insert(param.name.as_str(), None);
                } else {
                    lambda_locals.remove(param.name.as_str());
                }
            }
            max_effect_with_locals(body, fns, trait_env, allowed, &lambda_locals)
        }
        Expr::Call {
            callee,
            args,
            type_args: _,
            span,
        } => {
            if callee == "__clg_old" {
                return args.iter().try_fold(EffectLevel::Pure, |effect, arg| {
                    Ok(effect.join(max_effect_with_locals(
                        arg, fns, trait_env, allowed, fn_locals,
                    )?))
                });
            }
            if callee.starts_with("__clg_state_write$") {
                if allowed < EffectLevel::Mut {
                    return Err(TyperError::effect_required("state write", "mut", *span).into());
                }
                let mut eff = EffectLevel::Mut;
                for arg in args {
                    eff = eff.join(max_effect_with_locals(
                        arg, fns, trait_env, allowed, fn_locals,
                    )?);
                }
                return Ok(eff);
            }
            if callee.starts_with("__clg_event_emit$") {
                if allowed < EffectLevel::Mut {
                    return Err(TyperError::effect_required("event emission", "mut", *span).into());
                }
                let mut eff = EffectLevel::Mut;
                for arg in args {
                    eff = eff.join(max_effect_with_locals(
                        arg, fns, trait_env, allowed, fn_locals,
                    )?);
                }
                return Ok(eff);
            }
            if callee.starts_with("__clg_external_call$") {
                if allowed < EffectLevel::Mut {
                    return Err(TyperError::effect_required("external call", "mut", *span).into());
                }
                let mut eff = EffectLevel::Mut;
                for arg in args {
                    eff = eff.join(max_effect_with_locals(
                        arg, fns, trait_env, allowed, fn_locals,
                    )?);
                }
                return Ok(eff);
            }
            let mut eff = EffectLevel::Pure;
            for arg in args {
                eff = eff.join(max_effect_with_locals(
                    arg, fns, trait_env, allowed, fn_locals,
                )?);
            }
            let call_eff = call_effect(callee, fns, trait_env, fn_locals);
            if call_eff > allowed {
                return Err(
                    TyperError::effect_required(callee, effect_label(call_eff), *span).into(),
                );
            }
            Ok(eff.join(call_eff))
        }
    }
}

fn infer_fn_binding_effect<'a>(
    expr: &'a Expr,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    fn_locals: &FnValueEffects<'a>,
) -> Result<Option<Option<EffectLevel>>> {
    match expr {
        Expr::Lambda { params, body, .. } => {
            let mut lambda_locals = fn_locals.clone();
            for param in params {
                if matches!(param.ty, Type::Fn { .. }) {
                    lambda_locals.insert(param.name.as_str(), None);
                } else {
                    lambda_locals.remove(param.name.as_str());
                }
            }
            let eff =
                max_effect_with_locals(body, fns, trait_env, EffectLevel::Io, &lambda_locals)?;
            Ok(Some(Some(eff)))
        }
        Expr::Var(name, _) => Ok(fn_locals.get(name.as_str()).copied()),
        _ => Ok(None),
    }
}

fn call_effect(
    callee: &str,
    fns: &HashMap<&str, FnSig>,
    trait_env: &TraitEnv<'_>,
    fn_locals: &FnValueEffects<'_>,
) -> EffectLevel {
    if let Some(effect) = fn_locals.get(callee) {
        return effect.unwrap_or(EffectLevel::Io);
    }
    if let Some(level) = builtin_effect(callee) {
        return level;
    }
    if let Some((trait_name, method_name)) = callee.rsplit_once("::") {
        if let Some(info) = trait_env.traits.get(trait_name) {
            if let Some(method) = info.methods.get(method_name) {
                return level_from_effect(method.effect);
            }
        }
    }
    fns.get(callee).map(|sig| sig.effect).unwrap_or_else(|| {
        if callee.contains("::") {
            EffectLevel::Pure
        } else {
            EffectLevel::Io
        }
    })
}

fn builtin_effect(callee: &str) -> Option<EffectLevel> {
    if crate::guards::mut_collection_kind(callee).is_some() {
        return Some(EffectLevel::Mut);
    }
    match callee {
        "Some" | "None" | "Ok" | "Err" | "U8" | "U64" | "U128" | "U256" => Some(EffectLevel::Pure),
        // Ownership-transfer APIs stay pure-by-construction: linearity is enforced by
        // the type checker/resource tracker and VC obligations, while runtime behavior
        // reuses the same deterministic collection helpers.
        "std::list::push"
        | "std::list::insert"
        | "std::list::insert_checked"
        | "std::list::remove_take"
        | "std::list::remove_checked"
        | "std::map::insert_take"
        | "std::map::remove_take" => Some(EffectLevel::Pure),
        "std::wasi::print" | "std::env::time" | "std::env::chain_id" | "std::env::random" => {
            Some(EffectLevel::Io)
        }
        _ => None,
    }
}
