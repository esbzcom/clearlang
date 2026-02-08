use anyhow::Result;
use clg_ast::{Func, ParamKind, Type};
use std::collections::HashMap;

use super::expr::{self, consume_var_expr, expr_span, max_effect, type_of, ResourceTracker};
use super::mut_guards::enforce_mut_guards;
use super::totality::level_from_effect;
use super::type_params::validate_bounds;
use super::{
    base_type, base_types_match, binding_compatible, is_resource_type, refinement_loss,
    substitute_type, type_param_names, AliasMap, EffectLevel, FnSig, ImplInfo, LocalBinding,
    TypeDefs, TypeSubst, RETURN_KEY,
};
use crate::errors::TyperError;
use crate::guards::collect_mut_guards;

pub(super) fn check_func<'a>(
    f: &'a Func,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &super::TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
) -> Result<()> {
    let type_params = type_param_names(&f.type_params);
    let bounds = validate_bounds(&f.where_bounds, &type_params, trait_env)?;
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
        let is_resource = is_resource_type(&resolved_ty, aliases, type_defs)?;
        tracker.register_param(p.name.as_str(), p.kind, is_resource);
    }

    let allowed_effect = level_from_effect(f.effect);

    for req in &f.requires {
        let mut req_tracker = tracker.clone();
        let ty = type_of(
            &req.expr,
            &env,
            &mut req_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            &type_params,
            &bounds,
            0,
            None,
        )?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("require", ty, req.span).into());
        }
        max_effect(&req.expr, fns, trait_env, EffectLevel::Pure)?;
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
        let ty = type_of(
            &ens.expr,
            &ensure_env,
            &mut ensure_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            &type_params,
            &bounds,
            0,
            None,
        )?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("ensure", ty, ens.span).into());
        }
        max_effect(&ens.expr, fns, trait_env, EffectLevel::Pure)?;
    }

    let body_ty = type_of(
        &f.body,
        &env,
        &mut tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        &type_params,
        &bounds,
        0,
        Some(&ret_ty),
    )?;
    if !binding_compatible(&ret_ty, &body_ty, aliases)? {
        let sp = expr_span(&f.body);
        let allow_unsigned_literal = expr::literal_can_coerce_unsigned(&ret_ty, &body_ty, &f.body);
        if !allow_unsigned_literal {
            if let Some(err) = expr::unsigned_literal_range_error(&ret_ty, &body_ty, &f.body) {
                return Err(err.into());
            }
            if base_types_match(&ret_ty, &body_ty, aliases)? && refinement_loss(&ret_ty, &body_ty, aliases)
            {
                return Err(TyperError::refinement_loss(ret_ty.clone(), body_ty, sp).into());
            }
            return Err(TyperError::return_type_mismatch(ret_ty.clone(), body_ty, sp).into());
        }
    }
    if is_resource_type(&ret_ty, aliases, type_defs)? {
        consume_var_expr(&mut tracker, &f.body)?;
    }
    tracker.ensure_consumed()?;
    if allowed_effect >= EffectLevel::Mut {
        enforce_mut_guards(&f.body, &guard_keys)?;
    }
    max_effect(&f.body, fns, trait_env, allowed_effect)?;
    Ok(())
}

pub(super) fn check_impl_method<'a>(
    method: &'a Func,
    imp: &ImplInfo<'a>,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &super::TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
) -> Result<()> {
    let type_params = type_param_names(&imp.decl.type_params);
    let bounds = validate_bounds(&imp.decl.where_bounds, &type_params, trait_env)?;
    let mut subst: TypeSubst = HashMap::new();
    subst.insert("Self".to_string(), imp.decl.for_type.clone());

    let ret_ty = substitute_type(&method.ret, &subst);
    let mut env: HashMap<&str, LocalBinding> = HashMap::with_capacity(method.params.len() + 1);
    env.insert(
        RETURN_KEY,
        LocalBinding {
            ty: ret_ty.clone(),
            kind: ParamKind::Borrow,
        },
    );
    let mut tracker = ResourceTracker::with_capacity(method.params.len());
    for param in &method.params {
        let param_ty = substitute_type(&param.ty, &subst);
        let resolved_ty = base_type(&param_ty, aliases)?;
        if env
            .insert(
                param.name.as_str(),
                LocalBinding {
                    ty: param_ty.clone(),
                    kind: param.kind,
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_parameter(&param.name).into());
        }
        let is_resource = is_resource_type(&resolved_ty, aliases, type_defs)?;
        tracker.register_param(param.name.as_str(), param.kind, is_resource);
    }

    let allowed_effect = level_from_effect(method.effect);

    for req in &method.requires {
        let mut req_tracker = tracker.clone();
        let ty = type_of(
            &req.expr,
            &env,
            &mut req_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            &type_params,
            &bounds,
            0,
            None,
        )?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("require", ty, req.span).into());
        }
        max_effect(&req.expr, fns, trait_env, EffectLevel::Pure)?;
    }

    let mut ensure_env = env.clone();
    if !ensure_env.contains_key("result") {
        ensure_env.insert(
            "result",
            LocalBinding {
                ty: ret_ty.clone(),
                kind: ParamKind::Borrow,
            },
        );
    }
    for ens in &method.ensures {
        let mut ensure_tracker = tracker.clone();
        let ty = type_of(
            &ens.expr,
            &ensure_env,
            &mut ensure_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            &type_params,
            &bounds,
            0,
            None,
        )?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("ensure", ty, ens.span).into());
        }
        max_effect(&ens.expr, fns, trait_env, EffectLevel::Pure)?;
    }

    let body_ty = type_of(
        &method.body,
        &env,
        &mut tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        &type_params,
        &bounds,
        0,
        Some(&ret_ty),
    )?;
    if !binding_compatible(&ret_ty, &body_ty, aliases)? {
        let sp = expr_span(&method.body);
        let allow_unsigned_literal =
            expr::literal_can_coerce_unsigned(&ret_ty, &body_ty, &method.body);
        if !allow_unsigned_literal {
            if let Some(err) = expr::unsigned_literal_range_error(&ret_ty, &body_ty, &method.body)
            {
                return Err(err.into());
            }
            if base_types_match(&ret_ty, &body_ty, aliases)? && refinement_loss(&ret_ty, &body_ty, aliases)
            {
                return Err(TyperError::refinement_loss(ret_ty.clone(), body_ty, sp).into());
            }
            return Err(TyperError::return_type_mismatch(ret_ty.clone(), body_ty, sp).into());
        }
    }
    if is_resource_type(&ret_ty, aliases, type_defs)? {
        consume_var_expr(&mut tracker, &method.body)?;
    }
    tracker.ensure_consumed()?;
    if allowed_effect >= EffectLevel::Mut {
        let guard_keys = collect_mut_guards(&method.requires);
        enforce_mut_guards(&method.body, &guard_keys)?;
    }
    max_effect(&method.body, fns, trait_env, allowed_effect)?;
    Ok(())
}
