use anyhow::Result;
use clg_ast::{Expr, Type};
use std::collections::{HashMap, HashSet};

use super::super::{
    base_type, base_types_match, binding_compatible, find_resource_collection, is_resource_type,
    refinement_loss, substitute_type, type_param_names, unify_type_params, AliasMap, BoundsMap,
    FnSig, LocalBinding, TraitEnv, TypeDefs, TypeSubst, RETURN_KEY,
};
use super::block::type_block;
use super::call_expr::type_call_expr;
use super::literals::{
    ensure_int, int_literal_value, literal_can_coerce_unsigned, unsigned_literal_range_error,
};
use super::match_expr::type_match_expr;
use super::ops::{type_bin_expr, type_unary_expr};
use super::{expr_span, show_ty, ResourceTracker};
use crate::errors::TyperError;
pub(crate) fn consume_var_expr(tracker: &mut ResourceTracker, expr: &Expr) -> Result<()> {
    if let Expr::Var(name, span) = expr {
        tracker.consume_var(name, *span)?;
    }
    Ok(())
}
pub(crate) fn type_of<'a>(
    e: &'a Expr,
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
) -> Result<Type> {
    if depth > 1024 {
        return Err(TyperError::new(
            "T011",
            "type-check recursion limit exceeded".to_string(),
            0,
            0,
        )
        .into());
    }
    match e {
        Expr::Int(_, _) => Ok(Type::Int),
        Expr::Bool(_, _) => Ok(Type::Bool),
        Expr::String(_, _) => Ok(Type::String),
        Expr::ArrayLit { elems, span } => {
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
                        return Err(TyperError::array_length_mismatch(
                            len,
                            elems.len() as u32,
                            *span,
                        )
                        .into());
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
            if let Some(offending) =
                find_resource_collection(&arr_ty, &type_defs.resources, type_params)
            {
                return Err(TyperError::resource_in_collection(offending, Some(*span)).into());
            }
            *tracker = local_tracker;
            Ok(arr_ty)
        }
        Expr::TupleLit { elems, span } => {
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
                elem_tys.push(type_of(
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
                )?);
            }
            let tuple_ty = Type::Tuple(elem_tys);
            if let Some(offending) =
                find_resource_collection(&tuple_ty, &type_defs.resources, type_params)
            {
                return Err(TyperError::resource_in_collection(offending, Some(*span)).into());
            }
            *tracker = local_tracker;
            Ok(tuple_ty)
        }
        Expr::StructLit { name, fields, span } => {
            let Some(struct_info) = type_defs.structs.get(name.as_str()) else {
                if type_defs.enums.contains_key(name.as_str())
                    || type_defs.resources.contains(name.as_str())
                    || aliases.contains_key(name.as_str())
                {
                    return Err(TyperError::expected_struct(name.as_str(), *span).into());
                }
                return Err(TyperError::unknown_type(name.as_str(), Some(*span)).into());
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
                    return Err(TyperError::unknown_struct_field(
                        name.as_str(),
                        field_name,
                        field.span,
                    )
                    .into());
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
                    return Err(TyperError::missing_struct_field(
                        name.as_str(),
                        field.name.as_str(),
                        *span,
                    )
                    .into());
                }
            }
            let mut args: Vec<Type> = Vec::with_capacity(struct_info.decl.type_params.len());
            for param in &struct_info.decl.type_params {
                let Some(arg_ty) = subst.get(&param.name) else {
                    return Err(TyperError::cannot_infer_type_params(name.as_str(), *span).into());
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
                    if let Some(err) = unsigned_literal_range_error(&expected, &found, &field.expr)
                    {
                        return Err(err.into());
                    }
                    let sp = expr_span(&field.expr);
                    return Err(TyperError::struct_field_type_mismatch(
                        name.as_str(),
                        field_name,
                        expected,
                        found,
                        sp,
                    )
                    .into());
                }
                if !binding_compatible(&expected, &found, aliases)? {
                    if literal_can_coerce_unsigned(&expected, &found, &field.expr) {
                        continue;
                    }
                    if let Some(err) = unsigned_literal_range_error(&expected, &found, &field.expr)
                    {
                        return Err(err.into());
                    }
                    let sp = expr_span(&field.expr);
                    if refinement_loss(&expected, &found, aliases) {
                        return Err(TyperError::refinement_loss(expected, found, sp).into());
                    }
                    return Err(TyperError::struct_field_type_mismatch(
                        name.as_str(),
                        field_name,
                        expected,
                        found,
                        sp,
                    )
                    .into());
                }
            }
            *tracker = local_tracker;
            Ok(Type::Named {
                name: name.clone(),
                args,
            })
        }
        Expr::FieldAccess { base, field, span } => {
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
                    let Some(struct_info) = type_defs.structs.get(name.as_str()) else {
                        return Err(TyperError::expected_struct(name.as_str(), *span).into());
                    };
                    if struct_info.decl.type_params.len() != args.len() {
                        return Err(TyperError::type_arg_count_mismatch(
                            name.as_str(),
                            struct_info.decl.type_params.len(),
                            args.len(),
                            Some(*span),
                        )
                        .into());
                    }
                    let mut subst: TypeSubst = HashMap::new();
                    for (param, arg) in struct_info.decl.type_params.iter().zip(args.iter()) {
                        subst.insert(param.name.clone(), arg.clone());
                    }
                    let Some(field_def) = struct_info.fields.get(field.as_str()) else {
                        return Err(TyperError::unknown_struct_field(
                            name.as_str(),
                            field.as_str(),
                            *span,
                        )
                        .into());
                    };
                    Ok(substitute_type(&field_def.ty, &subst))
                }
                other => {
                    let rendered = show_ty(other);
                    Err(TyperError::expected_struct(&rendered, *span).into())
                }
            }
        }
        Expr::Index { base, index, span } => {
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
                            return Err(
                                TyperError::array_index_out_of_bounds(expr_span(index)).into()
                            );
                        }
                        if let Some(len) = len {
                            if idx as u64 >= len as u64 {
                                return Err(TyperError::array_index_out_of_bounds(expr_span(
                                    index,
                                ))
                                .into());
                            }
                        }
                    }
                    *tracker = local_tracker;
                    Ok(*inner)
                }
                Type::Slice(inner) => {
                    if let Some(idx) = int_literal_value(index) {
                        if idx < 0 {
                            return Err(
                                TyperError::array_index_out_of_bounds(expr_span(index)).into()
                            );
                        }
                    }
                    *tracker = local_tracker;
                    Ok(*inner)
                }
                Type::Tuple(elems) => {
                    let Some(idx) = int_literal_value(index) else {
                        return Err(
                            TyperError::tuple_index_requires_constant(expr_span(index)).into()
                        );
                    };
                    if idx < 0 || idx as usize >= elems.len() {
                        return Err(TyperError::array_index_out_of_bounds(expr_span(index)).into());
                    }
                    *tracker = local_tracker;
                    Ok(elems[idx as usize].clone())
                }
                other => {
                    Err(
                        TyperError::expected_collection("array, slice, or tuple", other, *span)
                            .into(),
                    )
                }
            }
        }
        Expr::Block { block } => type_block(
            block,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth + 1,
            expected,
        ),
        Expr::If {
            cond,
            then_br,
            else_br,
            span,
        } => {
            let cty = type_of(
                cond,
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
            if base_type(&cty, aliases)? != Type::Bool {
                // Reuse arg_type_mismatch with pseudo-callee `if` for stable code T003
                return Err(TyperError::arg_type_mismatch(1, "if", Type::Bool, cty, *span).into());
            }
            let baseline = tracker.clone();
            let mut then_tracker = baseline.clone();
            let tty = type_of(
                then_br,
                env,
                &mut then_tracker,
                fns,
                trait_env,
                aliases,
                type_defs,
                type_params,
                bounds,
                depth + 1,
                expected,
            )?;
            let mut else_tracker = baseline.clone();
            let ety = type_of(
                else_br,
                env,
                &mut else_tracker,
                fns,
                trait_env,
                aliases,
                type_defs,
                type_params,
                bounds,
                depth + 1,
                expected,
            )?;
            if tty != ety {
                return Err(TyperError::branch_type_mismatch(tty, ety, *span).into());
            }
            then_tracker.retain_keys_from(&baseline);
            else_tracker.retain_keys_from(&baseline);
            then_tracker.merge_branch(&else_tracker, *span)?;
            *tracker = then_tracker;
            Ok(tty)
        }

        Expr::Match {
            scrutinee,
            arms,
            span,
        } => type_match_expr(
            scrutinee,
            arms,
            *span,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
            expected,
        ),
        Expr::Try { expr, span } => {
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
                .ok_or_else(|| TyperError::try_missing_return(*span))?;
            let ret_ty = ret_binding.ty;
            match (inner, ret_ty) {
                (Type::Option(inner_ty), Type::Option(ret_inner)) => {
                    let found = (*inner_ty).clone();
                    let declared = (*ret_inner).clone();
                    if found != declared {
                        return Err(
                            TyperError::try_option_inner_mismatch(declared, found, *span).into(),
                        );
                    }
                    Ok(found)
                }
                (Type::Option(_), ret_other) => {
                    Err(TyperError::try_option_return_required(ret_other, *span).into())
                }
                (Type::Result(ok_ty, err_ty), Type::Result(ret_ok, ret_err)) => {
                    let ok_found = (*ok_ty).clone();
                    let err_found = (*err_ty).clone();
                    let ok_decl = (*ret_ok).clone();
                    let err_decl = (*ret_err).clone();
                    if ok_found != ok_decl || err_found != err_decl {
                        return Err(TyperError::try_result_mismatch(
                            ok_decl, err_decl, ok_found, err_found, *span,
                        )
                        .into());
                    }
                    Ok(ok_found)
                }
                (Type::Result(_, _), ret_other) => {
                    Err(TyperError::try_result_return_required(ret_other, *span).into())
                }
                (other, _) => Err(TyperError::try_input_not_option_result(other, *span).into()),
            }
        }
        Expr::Var(name, sp) => {
            tracker.use_var(name, *sp)?;
            match env.get(name.as_str()) {
                Some(binding) => Ok(binding.ty.clone()),
                None => {
                    if name == "result" {
                        if let Some(binding) = env.get(RETURN_KEY) {
                            return Ok(binding.ty.clone());
                        }
                    }
                    Err(TyperError::unknown_variable(name, *sp).into())
                }
            }
        }
        Expr::Return { expr, .. } => {
            let ret_expected = expected.or_else(|| env.get(RETURN_KEY).map(|binding| &binding.ty));
            let ty = type_of(
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
                ret_expected,
            )?;
            if is_resource_type(&ty, aliases, type_defs)? {
                consume_var_expr(tracker, expr.as_ref())?;
            }
            Ok(ty)
        }
        Expr::Unary { op, expr, span } => type_unary_expr(
            *op,
            expr,
            *span,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
        ),
        Expr::Bin { op, lhs, rhs, span } => type_bin_expr(
            *op,
            lhs,
            rhs,
            *span,
            env,
            tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            type_params,
            bounds,
            depth,
        ),
        Expr::Call { callee, args, span } => type_call_expr(
            callee.as_str(),
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
            expected,
            *span,
        ),
    }
}

pub(crate) fn infer_expr_type<'a>(
    e: &'a Expr,
    env: &HashMap<&'a str, LocalBinding>,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
) -> Result<Type> {
    let mut tracker = ResourceTracker::new();
    type_of(
        e,
        env,
        &mut tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        type_params,
        bounds,
        0,
        None,
    )
}
