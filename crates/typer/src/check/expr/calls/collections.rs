use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn type_collection_call<'a>(
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
                            "std::list::remove_take",
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
                    if contains_named_resource(&inner, &type_defs.resources, type_params) {
                        return Err(TyperError::resource_collection_op_requires_ownership_api(
                            normalized_callee,
                            "std::list::remove_take",
                            span,
                        )
                        .into());
                    }
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
        "std::list::remove_take" => {
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
                    let list_out = Type::List(inner.clone());
                    let removed_out = Type::Option(inner);
                    Ok(Some(Type::Tuple(vec![list_out, removed_out])))
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
                    if contains_named_resource(&inner, &type_defs.resources, type_params) {
                        return Err(TyperError::resource_collection_op_requires_ownership_api(
                            normalized_callee,
                            "std::list::remove_take",
                            span,
                        )
                        .into());
                    }
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
                            "std::map::remove_take",
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
                    if contains_named_resource(&value_ty, &type_defs.resources, type_params) {
                        return Err(TyperError::resource_collection_op_requires_ownership_api(
                            normalized_callee,
                            "std::map::insert_take",
                            span,
                        )
                        .into());
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
        "std::map::insert_take" => {
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
                    let map_out = Type::Map(k.clone(), v.clone());
                    let replaced_out = Type::Option(v);
                    Ok(Some(Type::Tuple(vec![map_out, replaced_out])))
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
                    if contains_named_resource(&v, &type_defs.resources, type_params) {
                        return Err(TyperError::resource_collection_op_requires_ownership_api(
                            normalized_callee,
                            "std::map::remove_take",
                            span,
                        )
                        .into());
                    }
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
        "std::map::remove_take" => {
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
                    let map_out = Type::Map(k.clone(), v.clone());
                    let removed_out = Type::Option(v);
                    Ok(Some(Type::Tuple(vec![map_out, removed_out])))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::new" => Err(TyperError::cannot_infer_collection(span, "std::map").into()),
        _ => Ok(None),
    }
}
