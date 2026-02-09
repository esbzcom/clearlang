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

pub(super) fn type_collection_call<'a>(
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
    let mut local_tracker = tracker.clone();
    let normalized_callee = match callee {
        "std::list::push_mut" => "std::list::push",
        "std::list::insert_mut" => "std::list::insert",
        "std::list::remove_mut" => "std::list::remove",
        "std::list::pop_mut" => "std::list::pop",
        "std::set::insert_mut" => "std::set::insert",
        "std::set::remove_mut" => "std::set::remove",
        "std::map::insert_mut" => "std::map::insert",
        "std::map::remove_mut" => "std::map::remove",
        other => other,
    };
    let var_binding = |i: usize| -> Option<(&str, Span, Type)> {
        match &args[i] {
            Expr::Var(name, var_span) => env
                .get(name.as_str())
                .map(|binding| (name.as_str(), *var_span, binding.ty.clone())),
            _ => None,
        }
    };
    let mut arg_ty = |i: usize, consume_position: bool| -> Result<Type> {
        if let Some((name, var_span, ty)) = var_binding(i) {
            if consume_position {
                return Ok(ty);
            }
            local_tracker.use_var_in_call(name, var_span, normalized_callee)?;
        }
        type_of(
            &args[i],
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
        )
    };
    match normalized_callee {
        // Array/Slice
        "std::array::len" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let aty = arg_ty(0, false)?;
            match base_type(&aty, aliases)? {
                Type::Array(_, _) => Ok(Some(Type::Int)),
                other => Err(TyperError::expected_collection("Array", other, span).into()),
            }
        }
        "std::slice::len" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let sty = arg_ty(0, false)?;
            match base_type(&sty, aliases)? {
                Type::Slice(_) => Ok(Some(Type::Int)),
                other => Err(TyperError::expected_collection("Slice", other, span).into()),
            }
        }
        "std::slice::from_array" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let aty = arg_ty(0, false)?;
            match base_type(&aty, aliases)? {
                Type::Array(inner, _) => Ok(Some(Type::Slice(inner))),
                other => Err(TyperError::expected_collection("Array", other, span).into()),
            }
        }
        "std::slice::sub" => {
            if args.len() != 3 {
                return Err(TyperError::arity_mismatch(callee, 3, args.len(), span).into());
            }
            let sty = arg_ty(0, false)?;
            let start_ty = arg_ty(1, false)?;
            let len_ty = arg_ty(2, false)?;
            ensure_int(start_ty, aliases, "start", Some(expr_span(&args[1])))?;
            ensure_int(len_ty, aliases, "len", Some(expr_span(&args[2])))?;
            match base_type(&sty, aliases)? {
                Type::Slice(inner) => Ok(Some(Type::Slice(inner))),
                other => Err(TyperError::expected_collection("Slice", other, span).into()),
            }
        }
        // List
        "std::list::len" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let lty = arg_ty(0, false)?;
            if let Type::List(_) = lty {
                *tracker = local_tracker;
                Ok(Some(Type::Int))
            } else {
                Err(TyperError::expected_collection("List", lty, span).into())
            }
        }
        "std::list::can_mut" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let lty = arg_ty(0, false)?;
            if let Type::List(_) = lty {
                *tracker = local_tracker;
                Ok(Some(Type::Bool))
            } else {
                Err(TyperError::expected_collection("List", lty, span).into())
            }
        }
        "std::list::get" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            let lty = arg_ty(0, false)?;
            let ity = arg_ty(1, false)?;
            let sp = expr_span(&args[1]);
            ensure_int(ity, aliases, "index", Some(sp))?;
            match lty {
                Type::List(inner) => {
                    if contains_named_resource(&inner, &type_defs.resources, type_params) {
                        return Err(TyperError::resource_get_requires_move_out(
                            normalized_callee,
                            (*inner).clone(),
                            span,
                        )
                        .into());
                    }
                    *tracker = local_tracker;
                    Ok(Some(Type::Option(inner)))
                }
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::push" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            let lty = arg_ty(0, true)?;
            match lty {
                Type::List(inner) => {
                    let list_ty = Type::List(inner.clone());
                    let letxty: Type = (*inner).clone();
                    let aty = arg_ty(1, true)?;
                    if aty != letxty {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(letxty, aty, sp).into());
                    }
                    if is_resource_type(&list_ty, aliases, type_defs)? {
                        if let Expr::Var(arg_name, arg_span) = &args[0] {
                            local_tracker.consume_var_in_call(
                                arg_name,
                                *arg_span,
                                normalized_callee,
                            )?;
                        }
                    }
                    if is_resource_type(&letxty, aliases, type_defs)? {
                        if let Expr::Var(arg_name, arg_span) = &args[1] {
                            local_tracker.consume_var_in_call(
                                arg_name,
                                *arg_span,
                                normalized_callee,
                            )?;
                        }
                    }
                    *tracker = local_tracker;
                    Ok(Some(Type::List(Box::new(*inner))))
                }
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::insert" => {
            if args.len() != 3 {
                return Err(TyperError::arity_mismatch(callee, 3, args.len(), span).into());
            }
            let lty = arg_ty(0, true)?;
            match lty {
                Type::List(inner) => {
                    let list_ty = Type::List(inner.clone());
                    let elem_expected: Type = (*inner).clone();
                    let aty_elem = arg_ty(1, true)?;
                    if aty_elem != elem_expected {
                        let sp = expr_span(&args[1]);
                        return Err(
                            TyperError::element_type_mismatch(elem_expected, aty_elem, sp).into(),
                        );
                    }
                    let ity = arg_ty(2, false)?;
                    let sp = expr_span(&args[2]);
                    ensure_int(ity, aliases, "index", Some(sp))?;
                    if is_resource_type(&list_ty, aliases, type_defs)? {
                        if let Expr::Var(arg_name, arg_span) = &args[0] {
                            local_tracker.consume_var_in_call(
                                arg_name,
                                *arg_span,
                                normalized_callee,
                            )?;
                        }
                    }
                    if is_resource_type(&elem_expected, aliases, type_defs)? {
                        if let Expr::Var(arg_name, arg_span) = &args[1] {
                            local_tracker.consume_var_in_call(
                                arg_name,
                                *arg_span,
                                normalized_callee,
                            )?;
                        }
                    }
                    *tracker = local_tracker;
                    Ok(Some(Type::List(Box::new(*inner))))
                }
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::remove" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            let lty = arg_ty(0, true)?;
            let ity = arg_ty(1, false)?;
            let sp = expr_span(&args[1]);
            ensure_int(ity, aliases, "index", Some(sp))?;
            match lty {
                Type::List(inner) => {
                    let list_ty = Type::List(inner.clone());
                    if is_resource_type(&list_ty, aliases, type_defs)? {
                        if let Expr::Var(arg_name, arg_span) = &args[0] {
                            local_tracker.consume_var_in_call(
                                arg_name,
                                *arg_span,
                                normalized_callee,
                            )?;
                        }
                    }
                    *tracker = local_tracker;
                    Ok(Some(Type::List(inner)))
                }
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::pop" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let lty = arg_ty(0, true)?;
            match lty {
                Type::List(inner) => {
                    let list_ty = Type::List(inner.clone());
                    if is_resource_type(&list_ty, aliases, type_defs)? {
                        if let Expr::Var(arg_name, arg_span) = &args[0] {
                            local_tracker.consume_var_in_call(
                                arg_name,
                                *arg_span,
                                normalized_callee,
                            )?;
                        }
                    }
                    *tracker = local_tracker;
                    Ok(Some(Type::Option(inner)))
                }
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::new" => Err(TyperError::cannot_infer_collection(span, "std::list").into()),
        // Set
        "std::set::len" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let sty = arg_ty(0, false)?;
            if let Type::Set(_) = sty {
                *tracker = local_tracker;
                Ok(Some(Type::Int))
            } else {
                Err(TyperError::expected_collection("Set", sty, span).into())
            }
        }
        "std::set::can_mut" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let sty = arg_ty(0, false)?;
            if let Type::Set(_) = sty {
                *tracker = local_tracker;
                Ok(Some(Type::Bool))
            } else {
                Err(TyperError::expected_collection("Set", sty, span).into())
            }
        }
        "std::set::contains" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0, false)? {
                Type::Set(inner) => {
                    let aty = arg_ty(1, false)?;
                    if aty != *inner {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*inner, aty, sp).into());
                    }
                    *tracker = local_tracker;
                    Ok(Some(Type::Bool))
                }
                other => Err(TyperError::expected_collection("Set", other, span).into()),
            }
        }
        "std::set::insert" | "std::set::remove" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0, false)? {
                Type::Set(inner) => {
                    let aty = arg_ty(1, false)?;
                    if aty != *inner {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*inner, aty, sp).into());
                    }
                    *tracker = local_tracker;
                    Ok(Some(Type::Set(inner)))
                }
                other => Err(TyperError::expected_collection("Set", other, span).into()),
            }
        }
        "std::set::new" => Err(TyperError::cannot_infer_collection(span, "std::set").into()),
        // Map
        "std::map::len" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let mty = arg_ty(0, false)?;
            if let Type::Map(_, _) = mty {
                *tracker = local_tracker;
                Ok(Some(Type::Int))
            } else {
                Err(TyperError::expected_collection("Map", mty, span).into())
            }
        }
        "std::map::can_mut" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let mty = arg_ty(0, false)?;
            if let Type::Map(_, _) = mty {
                *tracker = local_tracker;
                Ok(Some(Type::Bool))
            } else {
                Err(TyperError::expected_collection("Map", mty, span).into())
            }
        }
        "std::map::contains" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0, false)? {
                Type::Map(k, _v) => {
                    let aty = arg_ty(1, false)?;
                    if aty != *k {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*k, aty, sp).into());
                    }
                    *tracker = local_tracker;
                    Ok(Some(Type::Bool))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::get" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0, false)? {
                Type::Map(k, v) => {
                    let aty = arg_ty(1, false)?;
                    if aty != *k {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*k, aty, sp).into());
                    }
                    if contains_named_resource(&v, &type_defs.resources, type_params) {
                        return Err(TyperError::resource_get_requires_move_out(
                            normalized_callee,
                            (*v).clone(),
                            span,
                        )
                        .into());
                    }
                    *tracker = local_tracker;
                    Ok(Some(Type::Option(v)))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::insert" => {
            if args.len() != 3 {
                return Err(TyperError::arity_mismatch(callee, 3, args.len(), span).into());
            }
            match arg_ty(0, true)? {
                Type::Map(k, v) => {
                    let map_ty = Type::Map(k.clone(), v.clone());
                    let aty_k = arg_ty(1, false)?;
                    if aty_k != *k {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*k, aty_k, sp).into());
                    }
                    let value_ty = (*v).clone();
                    let aty_v = arg_ty(2, true)?;
                    if aty_v != value_ty {
                        let sp = expr_span(&args[2]);
                        return Err(TyperError::element_type_mismatch(value_ty, aty_v, sp).into());
                    }
                    if is_resource_type(&map_ty, aliases, type_defs)? {
                        if let Expr::Var(arg_name, arg_span) = &args[0] {
                            local_tracker.consume_var_in_call(
                                arg_name,
                                *arg_span,
                                normalized_callee,
                            )?;
                        }
                    }
                    if is_resource_type(&value_ty, aliases, type_defs)? {
                        if let Expr::Var(arg_name, arg_span) = &args[2] {
                            local_tracker.consume_var_in_call(
                                arg_name,
                                *arg_span,
                                normalized_callee,
                            )?;
                        }
                    }
                    *tracker = local_tracker;
                    Ok(Some(Type::Map(k, v)))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::remove" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0, true)? {
                Type::Map(k, v) => {
                    let map_ty = Type::Map(k.clone(), v.clone());
                    let aty = arg_ty(1, false)?;
                    if aty != *k {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*k, aty, sp).into());
                    }
                    if is_resource_type(&map_ty, aliases, type_defs)? {
                        if let Expr::Var(arg_name, arg_span) = &args[0] {
                            local_tracker.consume_var_in_call(
                                arg_name,
                                *arg_span,
                                normalized_callee,
                            )?;
                        }
                    }
                    *tracker = local_tracker;
                    Ok(Some(Type::Map(k, v)))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::new" => Err(TyperError::cannot_infer_collection(span, "std::map").into()),
        _ => Ok(None),
    }
}

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
