use anyhow::Result;
use clg_ast::{Block, Expr, Stmt};
use std::collections::HashMap;

use super::super::{effect_label, level_from_effect, EffectLevel, FnSig, TraitEnv};
use crate::errors::TyperError;

pub(crate) fn max_effect<'a>(
    e: &'a Expr,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    allowed: EffectLevel,
) -> Result<EffectLevel> {
    fn block_effect<'a>(
        block: &'a Block,
        fns: &HashMap<&'a str, FnSig>,
        trait_env: &TraitEnv<'a>,
        allowed: EffectLevel,
    ) -> Result<EffectLevel> {
        let mut eff = EffectLevel::Pure;
        for stmt in &block.statements {
            match stmt {
                Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                    eff = eff.join(max_effect(expr.as_ref(), fns, trait_env, allowed)?);
                }
                Stmt::While {
                    cond,
                    invariant,
                    variant,
                    body,
                    ..
                } => {
                    eff = eff.join(max_effect(cond.as_ref(), fns, trait_env, allowed)?);
                    eff = eff.join(max_effect(invariant.as_ref(), fns, trait_env, allowed)?);
                    if let Some(v) = variant {
                        eff = eff.join(max_effect(v.as_ref(), fns, trait_env, allowed)?);
                    }
                    eff = eff.join(block_effect(body.as_ref(), fns, trait_env, allowed)?);
                }
            }
        }
        if let Some(tail) = &block.tail {
            eff = eff.join(max_effect(tail.as_ref(), fns, trait_env, allowed)?);
        }
        Ok(eff)
    }
    match e {
        Expr::Block { block } => block_effect(block, fns, trait_env, allowed),
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {
            Ok(EffectLevel::Pure)
        }
        Expr::Return { expr, .. } => max_effect(expr, fns, trait_env, allowed),
        Expr::Unary { expr, .. } => max_effect(expr, fns, trait_env, allowed),
        Expr::Bin { lhs, rhs, .. } => {
            let left = max_effect(lhs, fns, trait_env, allowed)?;
            let right = max_effect(rhs, fns, trait_env, allowed)?;
            Ok(left.join(right))
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            let mut eff = EffectLevel::Pure;
            for elem in elems {
                eff = eff.join(max_effect(elem, fns, trait_env, allowed)?);
            }
            Ok(eff)
        }
        Expr::StructLit { fields, .. } => {
            let mut eff = EffectLevel::Pure;
            for field in fields {
                eff = eff.join(max_effect(&field.expr, fns, trait_env, allowed)?);
            }
            Ok(eff)
        }
        Expr::FieldAccess { base, .. } => max_effect(base, fns, trait_env, allowed),
        Expr::Index { base, index, .. } => {
            let base_eff = max_effect(base, fns, trait_env, allowed)?;
            let index_eff = max_effect(index, fns, trait_env, allowed)?;
            Ok(base_eff.join(index_eff))
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            let cond_eff = max_effect(cond, fns, trait_env, allowed)?;
            let then_eff = max_effect(then_br, fns, trait_env, allowed)?;
            let else_eff = max_effect(else_br, fns, trait_env, allowed)?;
            Ok(cond_eff.join(then_eff).join(else_eff))
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            let mut eff = max_effect(scrutinee, fns, trait_env, allowed)?;
            for arm in arms {
                eff = eff.join(max_effect(&arm.expr, fns, trait_env, allowed)?);
            }
            Ok(eff)
        }
        Expr::Try { expr, .. } => max_effect(expr, fns, trait_env, allowed),
        Expr::Call { callee, args, span } => {
            let mut eff = EffectLevel::Pure;
            for arg in args {
                eff = eff.join(max_effect(arg, fns, trait_env, allowed)?);
            }
            let call_eff = call_effect(callee, fns, trait_env);
            if call_eff > allowed {
                return Err(
                    TyperError::effect_required(callee, effect_label(call_eff), *span).into(),
                );
            }
            Ok(eff.join(call_eff))
        }
    }
}

fn call_effect(callee: &str, fns: &HashMap<&str, FnSig>, trait_env: &TraitEnv<'_>) -> EffectLevel {
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
    fns.get(callee)
        .map(|sig| sig.effect)
        .unwrap_or(EffectLevel::Pure)
}

fn builtin_effect(callee: &str) -> Option<EffectLevel> {
    match callee {
        "std::list::push_mut"
        | "std::list::insert_mut"
        | "std::list::remove_mut"
        | "std::list::pop_mut"
        | "std::set::insert_mut"
        | "std::set::remove_mut"
        | "std::map::insert_mut"
        | "std::map::remove_mut" => Some(EffectLevel::Mut),
        // Ownership-transfer APIs stay pure-by-construction: linearity is enforced by
        // the type checker/resource tracker and VC obligations, while runtime behavior
        // reuses the same deterministic collection helpers.
        "std::list::push"
        | "std::list::insert"
        | "std::list::remove_take"
        | "std::map::insert_take"
        | "std::map::remove_take" => Some(EffectLevel::Pure),
        "std::wasi::print" | "std::env::time" | "std::env::random" => Some(EffectLevel::Io),
        _ => None,
    }
}
