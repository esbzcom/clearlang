use anyhow::Result;
use clg_ast::{Expr, ParamKind, Span, Type};
use std::collections::{HashMap, HashSet};

use super::super::{
    base_type, base_types_match, binding_compatible, is_resource_type, refinement_loss,
    substitute_type, type_param_names, unify_type_params, AliasMap, BoundsMap, FnSig, LocalBinding,
    TraitEnv, TypeDefs, TypeSubst, RETURN_KEY,
};
use super::calls::{type_collection_call, type_trait_call};
use super::literals::{
    literal_can_coerce_unsigned, unsigned_literal_fits_value, unsigned_literal_range_error,
    unsigned_literal_value,
};
use super::traits::ensure_trait_bound;
use super::{expr_span, type_of, ResourceTracker};
use crate::errors::TyperError;

#[allow(clippy::too_many_arguments)]
pub(super) fn type_call_expr<'a>(
    callee: &str,
    args: &'a [Expr],
    explicit_type_args: &[Type],
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
    expected: Option<&Type>,
    span: Span,
) -> Result<Type> {
    let explicit_type_args_len = explicit_type_args.len();
    let reject_explicit_type_args = |name: &str| -> Result<Type> {
        Err(TyperError::type_arg_count_mismatch(name, 0, explicit_type_args_len, Some(span)).into())
    };
    let is_collection_callee = callee.starts_with("std::list::")
        || callee.starts_with("std::set::")
        || callee.starts_with("std::map::")
        || callee.starts_with("std::array::")
        || callee.starts_with("std::slice::");

    if explicit_type_args_len > 0 && is_collection_callee {
        return reject_explicit_type_args(callee);
    }

    if args.is_empty() {
        if let Some(exp) = expected {
            let exp_base = base_type(exp, aliases)?;
            match (callee, exp_base) {
                ("std::list::new", Type::List(_))
                | ("std::set::new", Type::Set(_))
                | ("std::map::new", Type::Map(_, _)) => {
                    return Ok(exp.clone());
                }
                _ => {}
            }
        }
    }
    // Phase 4.6 - Collections signatures (type-only)
    if let Some(t) = type_collection_call(
        callee,
        args,
        env,
        tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        type_params,
        bounds,
        depth,
        span,
    )? {
        return Ok(t);
    }
    if callee == "U8" || callee == "U64" || callee == "U128" || callee == "U256" {
        if explicit_type_args_len > 0 {
            return reject_explicit_type_args(callee);
        }
        if args.len() != 1 {
            return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
        }
        let target_ty = match callee {
            "U8" => Type::U8,
            "U64" => Type::U64,
            "U128" => Type::U128,
            "U256" => Type::U256,
            _ => Type::U64,
        };
        let mut local_tracker = tracker.clone();
        let arg_ty = type_of(
            &args[0],
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
        if arg_ty != target_ty {
            if matches!(arg_ty, Type::Int) {
                if let Some(value) = unsigned_literal_value(&args[0]) {
                    if !unsigned_literal_fits_value(&target_ty, value) {
                        let sp = expr_span(&args[0]);
                        return Err(TyperError::unsigned_literal_out_of_range(
                            target_ty.clone(),
                            value,
                            sp,
                        )
                        .into());
                    }
                } else {
                    let sp = expr_span(&args[0]);
                    return Err(TyperError::unsigned_cast_invalid(callee, arg_ty, sp).into());
                }
            } else {
                let sp = expr_span(&args[0]);
                return Err(TyperError::unsigned_cast_invalid(callee, arg_ty, sp).into());
            }
        }
        *tracker = local_tracker;
        return Ok(target_ty);
    }
    // Phase 4.5 - ADT constructors (partial): Some(T) infers Option<T>
    if callee == "Some" {
        if explicit_type_args_len > 0 {
            return reject_explicit_type_args(callee);
        }
        if args.len() != 1 {
            return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
        }
        let mut local_tracker = tracker.clone();
        let t0 = type_of(
            &args[0],
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
        if is_resource_type(&t0, aliases, type_defs)? {
            if let Expr::Var(arg_name, arg_span) = &args[0] {
                local_tracker.consume_var(arg_name, *arg_span)?;
            }
        }
        *tracker = local_tracker;
        return Ok(Type::Option(Box::new(t0)));
    }
    if callee == "None" {
        if explicit_type_args_len > 0 {
            return reject_explicit_type_args(callee);
        }
        if !args.is_empty() {
            return Err(TyperError::arity_mismatch(callee, 0, args.len(), span).into());
        }
        let ret_binding = env
            .get(RETURN_KEY)
            .cloned()
            .ok_or_else(|| TyperError::option_ctor_missing_return(span))?;
        return match ret_binding.ty {
            Type::Option(inner) => Ok(Type::Option(inner)),
            other => Err(TyperError::none_return_required(other, span).into()),
        };
    }
    if callee == "Ok" {
        if explicit_type_args_len > 0 {
            return reject_explicit_type_args(callee);
        }
        if args.len() != 1 {
            return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
        }
        let mut local_tracker = tracker.clone();
        let arg_ty = type_of(
            &args[0],
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
        if is_resource_type(&arg_ty, aliases, type_defs)? {
            if let Expr::Var(arg_name, arg_span) = &args[0] {
                local_tracker.consume_var(arg_name, *arg_span)?;
            }
        }
        let ret_binding = env
            .get(RETURN_KEY)
            .cloned()
            .ok_or_else(|| TyperError::result_ctor_missing_return(span))?;
        let result_ty = match ret_binding.ty {
            Type::Result(ok_ty, err_ty) => {
                if arg_ty != *ok_ty {
                    let sp = expr_span(&args[0]);
                    Err(TyperError::ok_argument_mismatch((*ok_ty).clone(), arg_ty, sp).into())
                } else {
                    Ok(Type::Result(ok_ty, err_ty))
                }
            }
            other => Err(TyperError::ok_return_required(other, span).into()),
        };
        if result_ty.is_ok() {
            *tracker = local_tracker;
        }
        return result_ty;
    }
    if callee == "Err" {
        if explicit_type_args_len > 0 {
            return reject_explicit_type_args(callee);
        }
        if args.len() != 1 {
            return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
        }
        let mut local_tracker = tracker.clone();
        let arg_ty = type_of(
            &args[0],
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
        if is_resource_type(&arg_ty, aliases, type_defs)? {
            if let Expr::Var(arg_name, arg_span) = &args[0] {
                local_tracker.consume_var(arg_name, *arg_span)?;
            }
        }
        let ret_binding = env
            .get(RETURN_KEY)
            .cloned()
            .ok_or_else(|| TyperError::result_ctor_missing_return(span))?;
        let result_ty = match ret_binding.ty {
            Type::Result(ok_ty, err_ty) => {
                if arg_ty != *err_ty {
                    let sp = expr_span(&args[0]);
                    Err(TyperError::err_argument_mismatch((*err_ty).clone(), arg_ty, sp).into())
                } else {
                    Ok(Type::Result(ok_ty, err_ty))
                }
            }
            other => Err(TyperError::err_return_required(other, span).into()),
        };
        if result_ty.is_ok() {
            *tracker = local_tracker;
        }
        return result_ty;
    }
    if explicit_type_args_len > 0 {
        if let Some((trait_name, _)) = callee.rsplit_once("::") {
            if trait_env.traits.contains_key(trait_name) {
                return reject_explicit_type_args(callee);
            }
        }
    }
    if let Some(trait_ty) = type_trait_call(
        callee,
        args,
        env,
        tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        type_params,
        bounds,
        depth,
        span,
    )? {
        return Ok(trait_ty);
    }
    if let Some(binding) = env.get(callee) {
        if explicit_type_args_len > 0 {
            return reject_explicit_type_args(callee);
        }
        if let Type::Fn {
            params: fn_params,
            ret,
        } = base_type(&binding.ty, aliases)?
        {
            if fn_params.len() != args.len() {
                return Err(
                    TyperError::arity_mismatch(callee, fn_params.len(), args.len(), span).into(),
                );
            }
            let mut local_tracker = tracker.clone();
            let mut arg_types: Vec<Type> = Vec::with_capacity(args.len());
            for (expected_param_ty, arg) in fn_params.iter().zip(args.iter()) {
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
                    Some(expected_param_ty),
                )?;
                arg_types.push(at);
            }
            for (i, ((expected_param_ty, found_arg_ty), arg_expr)) in fn_params
                .iter()
                .zip(arg_types.iter())
                .zip(args.iter())
                .enumerate()
            {
                if !base_types_match(expected_param_ty, found_arg_ty, aliases)? {
                    if literal_can_coerce_unsigned(expected_param_ty, found_arg_ty, arg_expr) {
                        continue;
                    }
                    if let Some(err) =
                        unsigned_literal_range_error(expected_param_ty, found_arg_ty, arg_expr)
                    {
                        return Err(err.into());
                    }
                    let sp = expr_span(arg_expr);
                    return Err(TyperError::arg_type_mismatch(
                        i,
                        callee,
                        expected_param_ty.clone(),
                        found_arg_ty.clone(),
                        sp,
                    )
                    .into());
                }
                if !binding_compatible(expected_param_ty, found_arg_ty, aliases)? {
                    if literal_can_coerce_unsigned(expected_param_ty, found_arg_ty, arg_expr) {
                        continue;
                    }
                    if let Some(err) =
                        unsigned_literal_range_error(expected_param_ty, found_arg_ty, arg_expr)
                    {
                        return Err(err.into());
                    }
                    let sp = expr_span(arg_expr);
                    if refinement_loss(expected_param_ty, found_arg_ty, aliases) {
                        return Err(TyperError::refinement_loss(
                            expected_param_ty.clone(),
                            found_arg_ty.clone(),
                            sp,
                        )
                        .into());
                    }
                    return Err(TyperError::arg_type_mismatch(
                        i,
                        callee,
                        expected_param_ty.clone(),
                        found_arg_ty.clone(),
                        sp,
                    )
                    .into());
                }
            }
            *tracker = local_tracker;
            return Ok((*ret).clone());
        }
    }
    if let Some((enum_name, variant_name)) = callee.rsplit_once("::") {
        if let Some(enum_info) = type_defs.enums.get(enum_name) {
            if explicit_type_args_len > 0 {
                return reject_explicit_type_args(callee);
            }
            let Some(variant_def) = enum_info.variants.get(variant_name) else {
                let label = format!("{enum_name}::{variant_name}");
                return Err(TyperError::unknown_enum_variant(&label, span).into());
            };
            if args.len() != variant_def.fields.len() {
                let label = format!("{enum_name}::{variant_name}");
                return Err(TyperError::enum_variant_arity_mismatch(
                    &label,
                    variant_def.fields.len(),
                    args.len(),
                    span,
                )
                .into());
            }
            let mut local_tracker = tracker.clone();
            let enum_params = type_param_names(&enum_info.decl.type_params);
            let mut subst: TypeSubst = HashMap::new();
            let mut found_types: Vec<Type> = Vec::with_capacity(args.len());
            for arg in args.iter() {
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
                found_types.push(at);
            }
            for (expected, found) in variant_def.fields.iter().zip(found_types.iter()) {
                unify_type_params(expected, found, &enum_params, &mut subst, aliases)?;
            }
            let mut args_vec: Vec<Type> = Vec::with_capacity(enum_info.decl.type_params.len());
            for param in &enum_info.decl.type_params {
                let Some(arg_ty) = subst.get(&param.name) else {
                    return Err(TyperError::cannot_infer_type_params(enum_name, span).into());
                };
                args_vec.push(arg_ty.clone());
            }
            for (i, (expected, arg)) in variant_def.fields.iter().zip(args.iter()).enumerate() {
                let expected = substitute_type(expected, &subst);
                let at = found_types
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
                    let label = format!("{enum_name}::{variant_name}");
                    return Err(
                        TyperError::arg_type_mismatch(i, &label, expected.clone(), at, sp).into(),
                    );
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
                        return Err(TyperError::refinement_loss(expected.clone(), at, sp).into());
                    }
                    let label = format!("{enum_name}::{variant_name}");
                    return Err(
                        TyperError::arg_type_mismatch(i, &label, expected.clone(), at, sp).into(),
                    );
                }
            }
            *tracker = local_tracker;
            return Ok(Type::Named {
                name: enum_name.to_string(),
                args: args_vec,
            });
        }
    }
    let FnSig {
        params,
        ret,
        type_params: callee_params,
        bounds: callee_bounds,
        ..
    } = fns
        .get(callee)
        .cloned()
        .ok_or_else(|| TyperError::unknown_function(callee, span))?;
    if params.len() != args.len() {
        return Err(TyperError::arity_mismatch(callee, params.len(), args.len(), span).into());
    }

    if explicit_type_args_len > 0 && callee_params.is_empty() {
        return reject_explicit_type_args(callee);
    }

    let mut local_tracker = tracker.clone();
    let mut borrowed: Vec<String> = Vec::new();
    let mut subst: TypeSubst = HashMap::new();
    if !callee_params.is_empty() && explicit_type_args_len > 0 {
        if explicit_type_args_len != callee_params.len() {
            return Err(TyperError::type_arg_count_mismatch(
                callee,
                callee_params.len(),
                explicit_type_args_len,
                Some(span),
            )
            .into());
        }
        for (name, ty) in callee_params.iter().zip(explicit_type_args.iter()) {
            subst.insert(name.clone(), ty.clone());
        }
        for bound in &callee_bounds {
            let Some(bound_ty) = subst.get(&bound.param) else {
                return Err(TyperError::cannot_infer_type_params(callee, span).into());
            };
            ensure_trait_bound(
                bound_ty,
                bound.trait_name.as_str(),
                type_params,
                bounds,
                trait_env,
                aliases,
                span,
            )?;
        }
    }

    let explicit_param_types: Vec<Type> = if !callee_params.is_empty() && explicit_type_args_len > 0
    {
        let mut out = Vec::with_capacity(params.len());
        for param in &params {
            out.push(substitute_type(&param.ty, &subst));
        }
        out
    } else {
        Vec::new()
    };
    let mut arg_types: Vec<Type> = Vec::with_capacity(args.len());
    for (i, (p, a)) in params.iter().zip(args.iter()).enumerate() {
        let arg_expected = if !explicit_param_types.is_empty() {
            explicit_param_types.get(i)
        } else if callee_params.is_empty() {
            Some(&p.ty)
        } else {
            None
        };
        let at = type_of(
            a,
            env,
            &mut local_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth + 1,
            arg_expected,
        )?;
        arg_types.push(at.clone());
        let resource_check_ty = if !explicit_param_types.is_empty() {
            explicit_param_types[i].clone()
        } else if callee_params.is_empty() {
            p.ty.clone()
        } else {
            at.clone()
        };
        if is_resource_type(&resource_check_ty, aliases, type_defs)? {
            match p.kind {
                ParamKind::Consume => {
                    if let Expr::Var(arg_name, arg_span) = a {
                        local_tracker.consume_var(arg_name, *arg_span)?;
                    }
                }
                ParamKind::Borrow => {
                    if let Expr::Var(arg_name, arg_span) = a {
                        local_tracker.borrow_var(arg_name, *arg_span)?;
                        borrowed.push(arg_name.clone());
                    }
                }
            }
        }
    }

    if !callee_params.is_empty() && explicit_type_args_len == 0 {
        let callee_param_set: HashSet<String> = callee_params.iter().cloned().collect();
        for (param, arg_ty) in params.iter().zip(arg_types.iter()) {
            unify_type_params(&param.ty, arg_ty, &callee_param_set, &mut subst, aliases)?;
        }
        for name in &callee_params {
            if !subst.contains_key(name) {
                return Err(TyperError::cannot_infer_type_params(callee, span).into());
            }
        }
        for bound in &callee_bounds {
            let Some(bound_ty) = subst.get(&bound.param) else {
                return Err(TyperError::cannot_infer_type_params(callee, span).into());
            };
            ensure_trait_bound(
                bound_ty,
                bound.trait_name.as_str(),
                type_params,
                bounds,
                trait_env,
                aliases,
                span,
            )?;
        }
    }

    for (i, (p, a)) in params.iter().zip(args.iter()).enumerate() {
        let at = arg_types.get(i).cloned().unwrap_or_else(|| p.ty.clone());
        let expected = if callee_params.is_empty() {
            p.ty.clone()
        } else {
            substitute_type(&p.ty, &subst)
        };
        if !base_types_match(&expected, &at, aliases)? {
            if literal_can_coerce_unsigned(&expected, &at, a) {
                continue;
            }
            if let Some(err) = unsigned_literal_range_error(&expected, &at, a) {
                return Err(err.into());
            }
            let sp = expr_span(a);
            return Err(TyperError::arg_type_mismatch(i, callee, expected, at, sp).into());
        }
        if !binding_compatible(&expected, &at, aliases)? {
            if literal_can_coerce_unsigned(&expected, &at, a) {
                continue;
            }
            if let Some(err) = unsigned_literal_range_error(&expected, &at, a) {
                return Err(err.into());
            }
            let sp = expr_span(a);
            if refinement_loss(&expected, &at, aliases) {
                return Err(TyperError::refinement_loss(expected, at, sp).into());
            }
            return Err(TyperError::arg_type_mismatch(i, callee, expected, at, sp).into());
        }
        // Resource tracking already handled during arg type inference.
    }
    for name in borrowed {
        local_tracker.release_borrow(&name)?;
    }
    *tracker = local_tracker;
    if callee_params.is_empty() {
        Ok(ret)
    } else {
        Ok(substitute_type(&ret, &subst))
    }
}
