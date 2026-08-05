use anyhow::Result;
use clg_ast::{Expr, ParamKind, Span, Type};
use std::collections::{HashMap, HashSet};

use super::super::super::{
    base_type, base_types_match, binding_compatible, find_resource_collection, is_resource_type,
    refinement_loss, substitute_type, type_param_names, unify_type_params, AliasMap, BoundsMap,
    FnSig, LocalBinding, TraitEnv, TypeDefs, TypeSubst, RETURN_KEY,
};
use super::super::literals::{
    ensure_int, int_literal_value, literal_can_coerce_unsigned, unsigned_literal_range_error,
};
use super::super::{expr_span, show_ty, ResourceTracker};
use super::{collect_lambda_captures, consume_var_expr, type_of};
use crate::errors::TyperError;

#[allow(clippy::too_many_arguments)]
pub(super) fn type_array_lit<'a>(
    elems: &'a [Expr],
    span: Span,
    expected: Option<&Type>,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
) -> Result<Type> {
    let mut local_tracker = tracker.clone();
    let first = elems
        .first()
        .ok_or_else(|| anyhow::anyhow!("array literal must have at least one element"))?;
    let elem_ty = type_of(
        first,
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
    if let Some(expected_ty) = expected {
        if let Type::Array(_, Some(len)) = base_type(expected_ty, aliases)? {
            if elems.len() as u32 != len {
                return Err(
                    TyperError::array_length_mismatch(len, elems.len() as u32, span).into(),
                );
            }
        }
    }
    for elem in elems.iter().skip(1) {
        let ety = type_of(
            elem,
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
        if !binding_compatible(&elem_ty, &ety, aliases)? {
            if literal_can_coerce_unsigned(&elem_ty, &ety, elem) {
                continue;
            }
            if let Some(err) = unsigned_literal_range_error(&elem_ty, &ety, elem) {
                return Err(err.into());
            }
            let sp = expr_span(elem);
            if refinement_loss(&elem_ty, &ety, aliases) {
                return Err(TyperError::refinement_loss(elem_ty.clone(), ety, sp).into());
            }
            return Err(TyperError::element_type_mismatch(elem_ty.clone(), ety, sp).into());
        }
    }
    let arr_ty = Type::Array(Box::new(elem_ty), Some(elems.len() as u32));
    if let Some(offending) = find_resource_collection(&arr_ty, &type_defs.resources, type_params) {
        return Err(TyperError::resource_in_collection(offending, Some(span)).into());
    }
    *tracker = local_tracker;
    Ok(arr_ty)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn type_tuple_lit<'a>(
    elems: &'a [Expr],
    span: Span,
    expected: Option<&Type>,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
) -> Result<Type> {
    let mut local_tracker = tracker.clone();
    let mut elem_tys = Vec::with_capacity(elems.len());
    let expected_elems = match expected {
        Some(Type::Tuple(expected_elems)) if expected_elems.len() == elems.len() => {
            Some(expected_elems)
        }
        _ => None,
    };
    for (idx, elem) in elems.iter().enumerate() {
        let elem_expected = expected_elems.and_then(|elems| elems.get(idx));
        let elem_ty = type_of(
            elem,
            env,
            &mut local_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth + 1,
            elem_expected,
        )?;
        if is_resource_type(&elem_ty, aliases, type_defs)? {
            consume_var_expr(&mut local_tracker, elem)?;
        }
        elem_tys.push(elem_ty);
    }
    let tuple_ty = Type::Tuple(elem_tys);
    if let Some(offending) = find_resource_collection(&tuple_ty, &type_defs.resources, type_params)
    {
        return Err(TyperError::resource_in_collection(offending, Some(span)).into());
    }
    *tracker = local_tracker;
    Ok(tuple_ty)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn type_struct_lit<'a>(
    name: &str,
    fields: &'a [clg_ast::StructFieldInit],
    span: Span,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
) -> Result<Type> {
    let Some(struct_info) = type_defs.structs.get(name) else {
        if type_defs.enums.contains_key(name)
            || type_defs.resources.contains(name)
            || aliases.contains_key(name)
        {
            return Err(TyperError::expected_struct(name, span).into());
        }
        return Err(TyperError::unknown_type(name, Some(span)).into());
    };
    let mut local_tracker = tracker.clone();
    let struct_params = type_param_names(&struct_info.decl.type_params);
    let mut subst: TypeSubst = HashMap::new();
    let mut seen: HashSet<&str> = HashSet::new();
    let mut found_types: HashMap<&str, Type> = HashMap::with_capacity(fields.len());
    for field in fields {
        let field_name = field.name.as_str();
        if !seen.insert(field_name) {
            return Err(TyperError::duplicate_struct_field(field_name, field.span).into());
        }
        let Some(field_def) = struct_info.fields.get(field_name) else {
            return Err(TyperError::unknown_struct_field(name, field_name, field.span).into());
        };
        let found = type_of(
            &field.expr,
            env,
            &mut local_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth + 1,
            Some(&field_def.ty),
        )?;
        unify_type_params(&field_def.ty, &found, &struct_params, &mut subst, aliases)?;
        found_types.insert(field_name, found);
    }
    for field in &struct_info.decl.fields {
        if !seen.contains(field.name.as_str()) {
            return Err(TyperError::missing_struct_field(name, field.name.as_str(), span).into());
        }
    }
    let mut args: Vec<Type> = Vec::with_capacity(struct_info.decl.type_params.len());
    for param in &struct_info.decl.type_params {
        let Some(arg_ty) = subst.get(&param.name) else {
            return Err(TyperError::cannot_infer_type_params(name, span).into());
        };
        args.push(arg_ty.clone());
    }
    for field in fields {
        let field_name = field.name.as_str();
        let Some(field_def) = struct_info.fields.get(field_name) else {
            continue;
        };
        let expected = substitute_type(&field_def.ty, &subst);
        let found = found_types
            .get(field_name)
            .cloned()
            .unwrap_or_else(|| expected.clone());
        if !base_types_match(&expected, &found, aliases)? {
            if literal_can_coerce_unsigned(&expected, &found, &field.expr) {
                continue;
            }
            if let Some(err) = unsigned_literal_range_error(&expected, &found, &field.expr) {
                return Err(err.into());
            }
            let sp = expr_span(&field.expr);
            return Err(TyperError::struct_field_type_mismatch(
                name, field_name, expected, found, sp,
            )
            .into());
        }
        if !binding_compatible(&expected, &found, aliases)? {
            if literal_can_coerce_unsigned(&expected, &found, &field.expr) {
                continue;
            }
            if let Some(err) = unsigned_literal_range_error(&expected, &found, &field.expr) {
                return Err(err.into());
            }
            let sp = expr_span(&field.expr);
            if refinement_loss(&expected, &found, aliases) {
                return Err(TyperError::refinement_loss(expected, found, sp).into());
            }
            return Err(TyperError::struct_field_type_mismatch(
                name, field_name, expected, found, sp,
            )
            .into());
        }
    }
    *tracker = local_tracker;
    Ok(Type::Named {
        name: name.to_string(),
        args,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn type_field_access<'a>(
    base: &'a Expr,
    field: &str,
    span: Span,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
) -> Result<Type> {
    let base_ty = type_of(
        base,
        env,
        tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        type_params,
        bounds,
        depth + 1,
        None,
    )?;
    let resolved = base_type(&base_ty, aliases)?;
    match resolved {
        Type::Named { name, args } => {
            if args.is_empty() {
                let state_name = name.strip_prefix("__clg_contract_state$");
                if let Some(contract_name) = state_name {
                    let state_fields = type_defs
                        .contract_states
                        .get(contract_name)
                        .expect("contract state type must have a field table");
                    let Some(field_def) = state_fields.get(field) else {
                        return Err(TyperError::unknown_struct_field("state", field, span).into());
                    };
                    return Ok(field_def.ty.clone());
                }
            }
            let Some(struct_info) = type_defs.structs.get(name.as_str()) else {
                return Err(TyperError::expected_struct(name.as_str(), span).into());
            };
            if struct_info.decl.type_params.len() != args.len() {
                return Err(TyperError::type_arg_count_mismatch(
                    name.as_str(),
                    struct_info.decl.type_params.len(),
                    args.len(),
                    Some(span),
                )
                .into());
            }
            let mut subst: TypeSubst = HashMap::new();
            for (param, arg) in struct_info.decl.type_params.iter().zip(args.iter()) {
                subst.insert(param.name.clone(), arg.clone());
            }
            let Some(field_def) = struct_info.fields.get(field) else {
                return Err(TyperError::unknown_struct_field(name.as_str(), field, span).into());
            };
            Ok(substitute_type(&field_def.ty, &subst))
        }
        other => {
            let rendered = show_ty(other);
            Err(TyperError::expected_struct(&rendered, span).into())
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn type_index_expr<'a>(
    base: &'a Expr,
    index: &'a Expr,
    span: Span,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
) -> Result<Type> {
    let mut local_tracker = tracker.clone();
    let base_ty = type_of(
        base,
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
    let idx_ty = type_of(
        index,
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
    ensure_int(idx_ty, aliases, "index", Some(expr_span(index)))?;
    let resolved = base_type(&base_ty, aliases)?;
    match resolved {
        Type::Array(inner, len) => {
            if let Some(idx) = int_literal_value(index) {
                if idx < 0 {
                    return Err(TyperError::array_index_out_of_bounds(expr_span(index)).into());
                }
                if let Some(len) = len {
                    if idx as u64 >= len as u64 {
                        return Err(TyperError::array_index_out_of_bounds(expr_span(index)).into());
                    }
                }
            }
            *tracker = local_tracker;
            Ok(*inner)
        }
        Type::Slice(inner) => {
            if let Some(idx) = int_literal_value(index) {
                if idx < 0 {
                    return Err(TyperError::array_index_out_of_bounds(expr_span(index)).into());
                }
            }
            *tracker = local_tracker;
            Ok(*inner)
        }
        Type::Tuple(elems) => {
            let Some(idx) = int_literal_value(index) else {
                return Err(TyperError::tuple_index_requires_constant(expr_span(index)).into());
            };
            if idx < 0 || idx as usize >= elems.len() {
                return Err(TyperError::array_index_out_of_bounds(expr_span(index)).into());
            }
            *tracker = local_tracker;
            Ok(elems[idx as usize].clone())
        }
        other => Err(TyperError::expected_collection("array, slice, or tuple", other, span).into()),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn type_try_expr<'a>(
    expr: &'a Expr,
    span: Span,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
) -> Result<Type> {
    let inner = type_of(
        expr,
        env,
        tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        type_params,
        bounds,
        depth + 1,
        None,
    )?;
    let ret_binding = env
        .get(RETURN_KEY)
        .cloned()
        .ok_or_else(|| TyperError::try_missing_return(span))?;
    let ret_ty = ret_binding.ty;
    match (inner, ret_ty) {
        (Type::Option(inner_ty), Type::Option(ret_inner)) => {
            let found = (*inner_ty).clone();
            let declared = (*ret_inner).clone();
            if found != declared {
                return Err(TyperError::try_option_inner_mismatch(declared, found, span).into());
            }
            Ok(found)
        }
        (Type::Option(_), ret_other) => {
            Err(TyperError::try_option_return_required(ret_other, span).into())
        }
        (Type::Result(ok_ty, err_ty), Type::Result(ret_ok, ret_err)) => {
            let ok_found = (*ok_ty).clone();
            let err_found = (*err_ty).clone();
            let ok_decl = (*ret_ok).clone();
            let err_decl = (*ret_err).clone();
            if ok_found != ok_decl || err_found != err_decl {
                return Err(TyperError::try_result_mismatch(
                    ok_decl, err_decl, ok_found, err_found, span,
                )
                .into());
            }
            Ok(ok_found)
        }
        (Type::Result(_, _), ret_other) => {
            Err(TyperError::try_result_return_required(ret_other, span).into())
        }
        (other, _) => Err(TyperError::try_input_not_option_result(other, span).into()),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn type_lambda_expr<'a>(
    params: &'a [clg_ast::LambdaParam],
    body: &'a Expr,
    span: Span,
    expected: Option<&Type>,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
) -> Result<Type> {
    let captures = collect_lambda_captures(params, body, env);
    for (name, capture_span) in captures {
        if let Some(binding) = env.get(name.as_str()) {
            if is_resource_type(&binding.ty, aliases, type_defs)? {
                return Err(TyperError::feature_not_supported(
                    &format!("capturing resource value `{}` in closures", name),
                    capture_span,
                )
                .into());
            }
        }
    }

    let expected_fn = if let Some(exp) = expected {
        match base_type(exp, aliases)? {
            Type::Fn {
                params: exp_params,
                ret: exp_ret,
            } => Some((exp_params, exp_ret)),
            _ => None,
        }
    } else {
        None
    };

    if let Some((exp_params, _)) = &expected_fn {
        if exp_params.len() != params.len() {
            return Err(
                TyperError::arity_mismatch("lambda", exp_params.len(), params.len(), span).into(),
            );
        }
        for (idx, (expected_param, found_param)) in exp_params.iter().zip(params.iter()).enumerate()
        {
            if !base_types_match(expected_param, &found_param.ty, aliases)? {
                return Err(TyperError::arg_type_mismatch(
                    idx,
                    "lambda",
                    expected_param.clone(),
                    found_param.ty.clone(),
                    found_param.span,
                )
                .into());
            }
        }
    }

    let mut lambda_env = env.clone();
    let mut seen_params: HashSet<&str> = HashSet::with_capacity(params.len());
    for param in params {
        if !seen_params.insert(param.name.as_str()) {
            return Err(TyperError::duplicate_parameter(param.name.as_str()).into());
        }
        lambda_env.insert(
            param.name.as_str(),
            LocalBinding {
                ty: param.ty.clone(),
                kind: ParamKind::Borrow,
            },
        );
    }

    let mut lambda_tracker = tracker.clone();
    let expected_ret = expected_fn.as_ref().map(|(_, ret)| ret.as_ref());
    let body_ty = type_of(
        body,
        &lambda_env,
        &mut lambda_tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        type_params,
        bounds,
        depth + 1,
        expected_ret,
    )?;
    if let Some((_, expected_ret)) = expected_fn {
        if !binding_compatible(&expected_ret, &body_ty, aliases)? {
            let body_span = expr_span(body);
            if !literal_can_coerce_unsigned(&expected_ret, &body_ty, body) {
                if let Some(err) = unsigned_literal_range_error(&expected_ret, &body_ty, body) {
                    return Err(err.into());
                }
                if base_types_match(&expected_ret, &body_ty, aliases)?
                    && refinement_loss(&expected_ret, &body_ty, aliases)
                {
                    return Err(TyperError::refinement_loss(
                        expected_ret.as_ref().clone(),
                        body_ty.clone(),
                        body_span,
                    )
                    .into());
                }
                return Err(TyperError::return_type_mismatch(
                    expected_ret.as_ref().clone(),
                    body_ty.clone(),
                    body_span,
                )
                .into());
            }
        }
    }

    Ok(Type::Fn {
        params: params
            .iter()
            .map(|param| param.ty.clone())
            .collect::<Vec<_>>(),
        ret: Box::new(body_ty),
    })
}
