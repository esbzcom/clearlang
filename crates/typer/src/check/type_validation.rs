use anyhow::Result;
use clg_ast::{Program, Span, Type, TypeParam};
use std::collections::HashSet;

use super::expr::expr_span;
use super::type_params::{substitute_type, validate_type_params};
use super::type_resolve::base_type;
use super::{AliasMap, StdTypeMap, TraitEnv, TypeDefs, TypeSubst};
use crate::errors::TyperError;

pub(super) fn ensure_no_resource_collections(
    ty: &Type,
    span: Option<Span>,
    resource_names: &HashSet<&str>,
    type_params: &HashSet<String>,
) -> Result<()> {
    if let Some(offending) = find_resource_collection(ty, resource_names, type_params) {
        return Err(TyperError::resource_in_collection(offending, span).into());
    }
    Ok(())
}

fn ensure_supported_type(
    ty: &Type,
    span: Option<Span>,
    type_params: &HashSet<String>,
) -> Result<()> {
    match ty {
        Type::U128 | Type::U256 => Ok(()),
        Type::Option(inner) | Type::List(inner) | Type::Set(inner) | Type::Slice(inner) => {
            ensure_supported_type(inner, span, type_params)
        }
        Type::Named { args, .. } => {
            if let Type::Named { name, .. } = ty {
                if type_params.contains(name.as_str()) {
                    if !args.is_empty() {
                        return Err(TyperError::type_param_has_args(name, span).into());
                    }
                    return Ok(());
                }
            }
            for arg in args {
                ensure_supported_type(arg, span, type_params)?;
            }
            Ok(())
        }
        Type::Array(inner, _) => ensure_supported_type(inner, span, type_params),
        Type::Tuple(elements) => {
            for elem in elements {
                ensure_supported_type(elem, span, type_params)?;
            }
            Ok(())
        }
        Type::Result(ok, err) | Type::Map(ok, err) => {
            ensure_supported_type(ok, span, type_params)?;
            ensure_supported_type(err, span, type_params)
        }
        _ => Ok(()),
    }
}

pub(super) fn validate_supported_types(
    program: &Program,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    trait_env: &TraitEnv,
) -> Result<()> {
    for res in &program.resources {
        for field in &res.fields {
            ensure_supported_type(&field.ty, Some(field.span), &HashSet::new())?;
        }
    }
    for s in &program.structs {
        let type_params = validate_type_params(&s.type_params, type_defs, aliases, trait_env)?;
        for field in &s.fields {
            ensure_supported_type(&field.ty, Some(field.span), &type_params)?;
        }
    }
    for e in &program.enums {
        let type_params = validate_type_params(&e.type_params, type_defs, aliases, trait_env)?;
        for variant in &e.variants {
            for ty in &variant.fields {
                ensure_supported_type(ty, Some(variant.span), &type_params)?;
            }
        }
    }
    for alias in &program.refined_aliases {
        ensure_supported_type(&alias.base, Some(alias.span), &HashSet::new())?;
    }
    for func in &program.funcs {
        let type_params = validate_type_params(&func.type_params, type_defs, aliases, trait_env)?;
        ensure_supported_type(&func.ret, None, &type_params)?;
        for param in &func.params {
            ensure_supported_type(&param.ty, None, &type_params)?;
        }
    }
    for tr in &program.traits {
        let mut self_params: HashSet<String> = HashSet::new();
        self_params.insert("Self".to_string());
        for method in &tr.methods {
            ensure_supported_type(&method.ret, Some(method.span), &self_params)?;
            for param in &method.params {
                ensure_supported_type(&param.ty, Some(method.span), &self_params)?;
            }
        }
    }
    for imp in &program.impls {
        let mut impl_params = validate_type_params(&imp.type_params, type_defs, aliases, trait_env)?;
        impl_params.insert("Self".to_string());
        ensure_supported_type(&imp.for_type, Some(imp.span), &impl_params)?;
        for method in &imp.methods {
            ensure_supported_type(&method.ret, Some(expr_span(&method.body)), &impl_params)?;
            for param in &method.params {
                ensure_supported_type(&param.ty, Some(expr_span(&method.body)), &impl_params)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn find_resource_collection(
    ty: &Type,
    resource_names: &HashSet<&str>,
    type_params: &HashSet<String>,
) -> Option<Type> {
    match ty {
        Type::List(inner) | Type::Set(inner) => {
            if contains_resource(inner, resource_names, type_params) {
                Some(ty.clone())
            } else {
                None
            }
        }
        Type::Map(key, val) => {
            if contains_resource(key, resource_names, type_params)
                || contains_resource(val, resource_names, type_params)
            {
                Some(ty.clone())
            } else {
                None
            }
        }
        Type::Option(inner) | Type::Slice(inner) => {
            find_resource_collection(inner, resource_names, type_params)
        }
        Type::Result(ok, err) => {
            find_resource_collection(ok, resource_names, type_params)
                .or_else(|| find_resource_collection(err, resource_names, type_params))
        }
        Type::Array(inner, _) => find_resource_collection(inner, resource_names, type_params),
        Type::Tuple(elements) => {
            for elem in elements {
                if let Some(found) = find_resource_collection(elem, resource_names, type_params) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn contains_resource(
    ty: &Type,
    resource_names: &HashSet<&str>,
    type_params: &HashSet<String>,
) -> bool {
    match ty {
        Type::Named { name, args } => {
            if type_params.contains(name.as_str()) {
                return false;
            }
            resource_names.contains(name.as_str())
                || args
                    .iter()
                    .any(|arg| contains_resource(arg, resource_names, type_params))
        }
        Type::Option(inner) | Type::List(inner) | Type::Set(inner) | Type::Slice(inner) => {
            contains_resource(inner, resource_names, type_params)
        }
        Type::Result(ok, err) | Type::Map(ok, err) => {
            contains_resource(ok, resource_names, type_params)
                || contains_resource(err, resource_names, type_params)
        }
        Type::Array(inner, _) => contains_resource(inner, resource_names, type_params),
        Type::Tuple(elements) => elements
            .iter()
            .any(|elem| contains_resource(elem, resource_names, type_params)),
        _ => false,
    }
}

pub(super) fn ensure_equatable_collection_keys(
    ty: &Type,
    span: Option<Span>,
    type_defs: &TypeDefs,
    aliases: &AliasMap,
    type_params: &HashSet<String>,
) -> Result<()> {
    let mut seen = HashSet::new();
    if let Some(offending) =
        find_non_equatable_collection_key(ty, type_defs, aliases, type_params, &mut seen)?
    {
        return Err(TyperError::non_equatable_key(offending, span).into());
    }
    Ok(())
}

fn find_non_equatable_collection_key(
    ty: &Type,
    type_defs: &TypeDefs,
    aliases: &AliasMap,
    type_params: &HashSet<String>,
    seen: &mut HashSet<String>,
) -> Result<Option<Type>> {
    let ty = base_type(ty, aliases)?;
    match ty {
        Type::Set(inner) => {
            if !is_equatable_type(&inner, type_defs, aliases, type_params, seen)? {
                Ok(Some(*inner))
            } else {
                Ok(None)
            }
        }
        Type::Map(key, val) => {
            if !is_equatable_type(&key, type_defs, aliases, type_params, seen)? {
                Ok(Some(*key))
            } else {
                find_non_equatable_collection_key(&val, type_defs, aliases, type_params, seen)
            }
        }
        Type::Option(inner) | Type::List(inner) | Type::Array(inner, _) | Type::Slice(inner) => {
            find_non_equatable_collection_key(&inner, type_defs, aliases, type_params, seen)
        }
        Type::Result(ok, err) => {
            if let Some(offending) =
                find_non_equatable_collection_key(&ok, type_defs, aliases, type_params, seen)?
            {
                Ok(Some(offending))
            } else {
                find_non_equatable_collection_key(&err, type_defs, aliases, type_params, seen)
            }
        }
        Type::Tuple(elements) => {
            for elem in elements {
                if let Some(offending) =
                    find_non_equatable_collection_key(&elem, type_defs, aliases, type_params, seen)?
                {
                    return Ok(Some(offending));
                }
            }
            Ok(None)
        }
        Type::Named { name, args } => {
            if type_params.contains(&name) || name == "Self" {
                return Ok(None);
            }
            if let Some(info) = type_defs.structs.get(name.as_str()) {
                if !seen.insert(format!("struct:{name}")) {
                    return Ok(None);
                }
                let subst = build_type_subst(&info.decl.type_params, &args);
                for field in &info.decl.fields {
                    let field_ty = substitute_type(&field.ty, &subst);
                    if let Some(offending) = find_non_equatable_collection_key(
                        &field_ty, type_defs, aliases, type_params, seen,
                    )? {
                        seen.remove(&format!("struct:{name}"));
                        return Ok(Some(offending));
                    }
                }
                seen.remove(&format!("struct:{name}"));
                Ok(None)
            } else if let Some(info) = type_defs.enums.get(name.as_str()) {
                if !seen.insert(format!("enum:{name}")) {
                    return Ok(None);
                }
                let subst = build_type_subst(&info.decl.type_params, &args);
                for variant in &info.decl.variants {
                    for field_ty in &variant.fields {
                        let field_ty = substitute_type(field_ty, &subst);
                        if let Some(offending) = find_non_equatable_collection_key(
                            &field_ty, type_defs, aliases, type_params, seen,
                        )? {
                            seen.remove(&format!("enum:{name}"));
                            return Ok(Some(offending));
                        }
                    }
                }
                seen.remove(&format!("enum:{name}"));
                Ok(None)
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn is_equatable_type(
    ty: &Type,
    type_defs: &TypeDefs,
    aliases: &AliasMap,
    type_params: &HashSet<String>,
    seen: &mut HashSet<String>,
) -> Result<bool> {
    let ty = base_type(ty, aliases)?;
    Ok(match ty {
        Type::Int
        | Type::Bool
        | Type::U8
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::String
        | Type::Bytes => true,
        Type::Option(inner) => is_equatable_type(&inner, type_defs, aliases, type_params, seen)?,
        Type::Result(ok, err) => {
            is_equatable_type(&ok, type_defs, aliases, type_params, seen)?
                && is_equatable_type(&err, type_defs, aliases, type_params, seen)?
        }
        Type::Array(_, _) | Type::Slice(_) => false,
        Type::Tuple(elements) => {
            let mut ok = true;
            for elem in elements {
                if !is_equatable_type(&elem, type_defs, aliases, type_params, seen)? {
                    ok = false;
                    break;
                }
            }
            ok
        }
        Type::List(_) | Type::Set(_) | Type::Map(_, _) => false,
        Type::Named { name, args } => {
            if type_params.contains(&name) || name == "Self" {
                return Ok(true);
            }
            if type_defs.resources.contains(name.as_str()) {
                return Ok(false);
            }
            if let Some(info) = type_defs.structs.get(name.as_str()) {
                if !seen.insert(format!("struct:{name}")) {
                    return Ok(true);
                }
                let subst = build_type_subst(&info.decl.type_params, &args);
                for field in &info.decl.fields {
                    let field_ty = substitute_type(&field.ty, &subst);
                    if !is_equatable_type(&field_ty, type_defs, aliases, type_params, seen)? {
                        seen.remove(&format!("struct:{name}"));
                        return Ok(false);
                    }
                }
                seen.remove(&format!("struct:{name}"));
                true
            } else if let Some(info) = type_defs.enums.get(name.as_str()) {
                if !seen.insert(format!("enum:{name}")) {
                    return Ok(true);
                }
                let subst = build_type_subst(&info.decl.type_params, &args);
                for variant in &info.decl.variants {
                    for field_ty in &variant.fields {
                        let field_ty = substitute_type(field_ty, &subst);
                        if !is_equatable_type(&field_ty, type_defs, aliases, type_params, seen)? {
                            seen.remove(&format!("enum:{name}"));
                            return Ok(false);
                        }
                    }
                }
                seen.remove(&format!("enum:{name}"));
                true
            } else {
                true
            }
        }
    })
}

fn build_type_subst(params: &[TypeParam], args: &[Type]) -> TypeSubst {
    let mut subst = TypeSubst::with_capacity(params.len());
    for (param, arg) in params.iter().zip(args.iter()) {
        subst.insert(param.name.clone(), arg.clone());
    }
    subst
}

pub(super) fn contains_named_resource(
    ty: &Type,
    resource_names: &HashSet<&str>,
    type_params: &HashSet<String>,
) -> bool {
    match ty {
        Type::Named { name, args } => {
            if type_params.contains(name.as_str()) {
                return false;
            }
            resource_names.contains(name.as_str())
                || args
                    .iter()
                    .any(|arg| contains_named_resource(arg, resource_names, type_params))
        }
        Type::Option(inner) | Type::List(inner) | Type::Set(inner) | Type::Slice(inner) => {
            contains_named_resource(inner, resource_names, type_params)
        }
        Type::Result(ok, err) | Type::Map(ok, err) => {
            contains_named_resource(ok, resource_names, type_params)
                || contains_named_resource(err, resource_names, type_params)
        }
        Type::Array(inner, _) => contains_named_resource(inner, resource_names, type_params),
        Type::Tuple(elements) => elements
            .iter()
            .any(|elem| contains_named_resource(elem, resource_names, type_params)),
        _ => false,
    }
}

pub(super) fn validate_no_resource_collections(
    program: &Program,
    resource_names: &HashSet<&str>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    trait_env: &TraitEnv,
) -> Result<()> {
    for res in &program.resources {
        for field in &res.fields {
            ensure_no_resource_collections(
                &field.ty,
                Some(field.span),
                resource_names,
                &HashSet::new(),
            )?;
        }
    }
    for s in &program.structs {
        let type_params = validate_type_params(&s.type_params, type_defs, aliases, trait_env)?;
        for field in &s.fields {
            ensure_no_resource_collections(
                &field.ty,
                Some(field.span),
                resource_names,
                &type_params,
            )?;
        }
    }
    for e in &program.enums {
        let type_params = validate_type_params(&e.type_params, type_defs, aliases, trait_env)?;
        for variant in &e.variants {
            for ty in &variant.fields {
                ensure_no_resource_collections(
                    ty,
                    Some(variant.span),
                    resource_names,
                    &type_params,
                )?;
            }
        }
    }
    for func in &program.funcs {
        let type_params = validate_type_params(&func.type_params, type_defs, aliases, trait_env)?;
        ensure_no_resource_collections(&func.ret, None, resource_names, &type_params)?;
        for param in &func.params {
            ensure_no_resource_collections(&param.ty, None, resource_names, &type_params)?;
        }
    }
    Ok(())
}

pub(super) fn validate_equatable_collections(
    program: &Program,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    trait_env: &TraitEnv,
) -> Result<()> {
    for res in &program.resources {
        for field in &res.fields {
            ensure_equatable_collection_keys(
                &field.ty,
                Some(field.span),
                type_defs,
                aliases,
                &HashSet::new(),
            )?;
        }
    }
    for alias in &program.refined_aliases {
        ensure_equatable_collection_keys(
            &alias.base,
            Some(alias.span),
            type_defs,
            aliases,
            &HashSet::new(),
        )?;
    }
    for s in &program.structs {
        let type_params = validate_type_params(&s.type_params, type_defs, aliases, trait_env)?;
        for field in &s.fields {
            ensure_equatable_collection_keys(
                &field.ty,
                Some(field.span),
                type_defs,
                aliases,
                &type_params,
            )?;
        }
    }
    for e in &program.enums {
        let type_params = validate_type_params(&e.type_params, type_defs, aliases, trait_env)?;
        for variant in &e.variants {
            for ty in &variant.fields {
                ensure_equatable_collection_keys(
                    ty,
                    Some(variant.span),
                    type_defs,
                    aliases,
                    &type_params,
                )?;
            }
        }
    }
    for func in &program.funcs {
        let type_params = validate_type_params(&func.type_params, type_defs, aliases, trait_env)?;
        ensure_equatable_collection_keys(&func.ret, None, type_defs, aliases, &type_params)?;
        for param in &func.params {
            ensure_equatable_collection_keys(&param.ty, None, type_defs, aliases, &type_params)?;
        }
    }
    Ok(())
}

pub(super) fn ensure_known_type(
    ty: &Type,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    std_types: &StdTypeMap,
    span: Option<Span>,
) -> Result<()> {
    match ty {
        Type::Named { name, args } => {
            let name = name.as_str();
            if type_params.contains(name) {
                if !args.is_empty() {
                    return Err(TyperError::type_param_has_args(name, span).into());
                }
                return Ok(());
            }
            if aliases.contains_key(name) {
                if !args.is_empty() {
                    return Err(TyperError::type_arg_count_mismatch(name, 0, args.len(), span).into());
                }
                return Ok(());
            }
            if type_defs.resources.contains(name) {
                if !args.is_empty() {
                    return Err(TyperError::type_arg_count_mismatch(name, 0, args.len(), span).into());
                }
                return Ok(());
            }
            if let Some(struct_info) = type_defs.structs.get(name) {
                let expected = struct_info.decl.type_params.len();
                if expected != args.len() {
                    return Err(TyperError::type_arg_count_mismatch(
                        name,
                        expected,
                        args.len(),
                        span,
                    )
                    .into());
                }
                for arg in args {
                    ensure_known_type(arg, aliases, type_defs, type_params, std_types, span)?;
                }
                return Ok(());
            }
            if let Some(enum_info) = type_defs.enums.get(name) {
                let expected = enum_info.decl.type_params.len();
                if expected != args.len() {
                    return Err(TyperError::type_arg_count_mismatch(
                        name,
                        expected,
                        args.len(),
                        span,
                    )
                    .into());
                }
                for arg in args {
                    ensure_known_type(arg, aliases, type_defs, type_params, std_types, span)?;
                }
                return Ok(());
            }
            if std_types.contains_key(name) {
                if !args.is_empty() {
                    return Err(TyperError::type_arg_count_mismatch(
                        name,
                        0,
                        args.len(),
                        span,
                    )
                    .into());
                }
                return Ok(());
            }
            Err(TyperError::unknown_type(name, span).into())
        }
        Type::Option(inner) | Type::List(inner) | Type::Set(inner) | Type::Slice(inner) => {
            ensure_known_type(inner, aliases, type_defs, type_params, std_types, span)
        }
        Type::Array(inner, _) => {
            ensure_known_type(inner, aliases, type_defs, type_params, std_types, span)
        }
        Type::Tuple(elements) => {
            for elem in elements {
                ensure_known_type(elem, aliases, type_defs, type_params, std_types, span)?;
            }
            Ok(())
        }
        Type::Result(ok, err) | Type::Map(ok, err) => {
            ensure_known_type(ok, aliases, type_defs, type_params, std_types, span)?;
            ensure_known_type(err, aliases, type_defs, type_params, std_types, span)
        }
        _ => Ok(()),
    }
}

pub(super) fn validate_known_types(
    program: &Program,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    trait_env: &TraitEnv,
    std_types: &StdTypeMap,
) -> Result<()> {
    for res in &program.resources {
        for field in &res.fields {
            ensure_known_type(
                &field.ty,
                aliases,
                type_defs,
                &HashSet::new(),
                std_types,
                Some(field.span),
            )?;
        }
    }
    for s in &program.structs {
        let type_params = validate_type_params(&s.type_params, type_defs, aliases, trait_env)?;
        for field in &s.fields {
            ensure_known_type(
                &field.ty,
                aliases,
                type_defs,
                &type_params,
                std_types,
                Some(field.span),
            )?;
        }
    }
    for e in &program.enums {
        let type_params = validate_type_params(&e.type_params, type_defs, aliases, trait_env)?;
        for variant in &e.variants {
            for ty in &variant.fields {
                ensure_known_type(
                    ty,
                    aliases,
                    type_defs,
                    &type_params,
                    std_types,
                    Some(variant.span),
                )?;
            }
        }
    }
    for alias in &program.refined_aliases {
        ensure_known_type(
            &alias.base,
            aliases,
            type_defs,
            &HashSet::new(),
            std_types,
            Some(alias.span),
        )?;
    }
    for func in &program.funcs {
        let type_params = validate_type_params(&func.type_params, type_defs, aliases, trait_env)?;
        ensure_known_type(&func.ret, aliases, type_defs, &type_params, std_types, None)?;
        for param in &func.params {
            ensure_known_type(&param.ty, aliases, type_defs, &type_params, std_types, None)?;
        }
    }
    Ok(())
}
