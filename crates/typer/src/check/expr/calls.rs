use anyhow::Result;
use clg_ast::{Expr, ParamKind, Span, Type};
use std::collections::{HashMap, HashSet};

use super::super::{
    base_type, base_types_match, binding_compatible, contains_named_resource, is_resource_type,
    refinement_loss, substitute_type, unify_type_params, AliasMap, BoundsMap, FnSig, LocalBinding,
    TraitEnv, TypeDefs, TypeSubst,
};
use super::literals::{ensure_int, literal_can_coerce_unsigned, unsigned_literal_range_error};
use super::traits::ensure_trait_bound;
use super::{expr_span, type_of, ResourceTracker};
use crate::errors::TyperError;

mod collections;
pub(super) use self::collections::type_collection_call;

#[allow(clippy::too_many_arguments)]
pub(super) fn type_trait_call<'a>(
    callee: &str,
    args: &'a [Expr],
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
    span: Span,
) -> Result<Option<Type>> {
    let Some((trait_name, method_name)) = callee.rsplit_once("::") else {
        return Ok(None);
    };
    let Some(trait_info) = trait_env.traits.get(trait_name) else {
        return Ok(None);
    };
    let Some(method) = trait_info.methods.get(method_name) else {
        return Err(TyperError::unknown_function(callee, span).into());
    };
    if method.params.len() != args.len() {
        return Err(
            TyperError::arity_mismatch(callee, method.params.len(), args.len(), span).into(),
        );
    }
    let mut local_tracker = tracker.clone();
    let mut borrowed: Vec<String> = Vec::new();
    let mut arg_types: Vec<Type> = Vec::with_capacity(args.len());
    for (param, arg) in method.params.iter().zip(args.iter()) {
        let at = type_of(
            arg,
            env,
            &mut local_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth + 1,
            None,
        )?;
        arg_types.push(at.clone());
        if is_resource_type(&at, aliases, type_defs)? {
            match param.kind {
                ParamKind::Consume => {
                    if let Expr::Var(arg_name, arg_span) = arg {
                        local_tracker.consume_var(arg_name, *arg_span)?;
                    }
                }
                ParamKind::Borrow => {
                    if let Expr::Var(arg_name, arg_span) = arg {
                        local_tracker.borrow_var(arg_name, *arg_span)?;
                        borrowed.push(arg_name.clone());
                    }
                }
            }
        }
    }

    let mut subst: TypeSubst = HashMap::new();
    let mut self_params: HashSet<String> = HashSet::with_capacity(1);
    self_params.insert("Self".to_string());
    for (param, arg_ty) in method.params.iter().zip(arg_types.iter()) {
        unify_type_params(&param.ty, arg_ty, &self_params, &mut subst, aliases)?;
    }
    let Some(self_ty) = subst.get("Self").cloned() else {
        return Err(TyperError::cannot_infer_type_params(callee, span).into());
    };
    ensure_trait_bound(
        &self_ty,
        trait_name,
        type_params,
        bounds,
        trait_env,
        aliases,
        span,
    )?;

    for (i, (param, arg)) in method.params.iter().zip(args.iter()).enumerate() {
        let expected = substitute_type(&param.ty, &subst);
        let at = arg_types
            .get(i)
            .cloned()
            .unwrap_or_else(|| expected.clone());
        if !base_types_match(&expected, &at, aliases)? {
            if literal_can_coerce_unsigned(&expected, &at, arg) {
                continue;
            }
            if let Some(err) = unsigned_literal_range_error(&expected, &at, arg) {
                return Err(err.into());
            }
            let sp = expr_span(arg);
            return Err(TyperError::arg_type_mismatch(i, callee, expected, at, sp).into());
        }
        if !binding_compatible(&expected, &at, aliases)? {
            if literal_can_coerce_unsigned(&expected, &at, arg) {
                continue;
            }
            if let Some(err) = unsigned_literal_range_error(&expected, &at, arg) {
                return Err(err.into());
            }
            let sp = expr_span(arg);
            if refinement_loss(&expected, &at, aliases) {
                return Err(TyperError::refinement_loss(expected, at, sp).into());
            }
            return Err(TyperError::arg_type_mismatch(i, callee, expected, at, sp).into());
        }
    }

    for name in borrowed {
        local_tracker.release_borrow(&name)?;
    }
    *tracker = local_tracker;
    Ok(Some(substitute_type(&method.ret, &subst)))
}
