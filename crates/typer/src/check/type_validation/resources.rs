use anyhow::Result;
use clg_ast::{Program, Span, Type};
use std::collections::HashSet;

use super::super::type_params::validate_type_params;
use super::super::{AliasMap, TraitEnv, TypeDefs};
use crate::errors::TyperError;

pub(crate) fn ensure_no_resource_collections(
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

pub(crate) fn ensure_no_unsupported_resource_collections(
    ty: &Type,
    span: Option<Span>,
    resource_names: &HashSet<&str>,
    type_params: &HashSet<String>,
) -> Result<()> {
    if let Some(offending) = find_unsupported_resource_collection(ty, resource_names, type_params)
    {
        return Err(TyperError::resource_in_collection(offending, span).into());
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
        Type::Result(ok, err) => find_resource_collection(ok, resource_names, type_params)
            .or_else(|| find_resource_collection(err, resource_names, type_params)),
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

fn find_unsupported_resource_collection(
    ty: &Type,
    resource_names: &HashSet<&str>,
    type_params: &HashSet<String>,
) -> Option<Type> {
    match ty {
        // Phase 17.6: List<Resource> is allowed. Keep recursing to detect unsupported
        // nested forms (e.g. List<Set<Resource>>).
        Type::List(inner) => {
            if !contains_resource(inner, resource_names, type_params) {
                None
            } else {
                find_unsupported_resource_collection(inner, resource_names, type_params)
            }
        }
        // Phase 17.6: Map<K, Resource> is allowed only when K is non-resource.
        // Recurse through value to detect unsupported nested forms.
        Type::Map(key, val) => {
            if contains_resource(key, resource_names, type_params) {
                Some(ty.clone())
            } else if !contains_resource(val, resource_names, type_params) {
                None
            } else {
                find_unsupported_resource_collection(val, resource_names, type_params)
            }
        }
        // Set<Resource> remains unsupported in 17.6.1/17.6.2.
        Type::Set(inner) => {
            if contains_resource(inner, resource_names, type_params) {
                Some(ty.clone())
            } else {
                None
            }
        }
        // Arrays/slices/tuples containing resources remain unsupported for now.
        Type::Array(inner, _) | Type::Slice(inner) => {
            if contains_resource(inner, resource_names, type_params) {
                Some(ty.clone())
            } else {
                None
            }
        }
        Type::Tuple(elements) => {
            if elements
                .iter()
                .any(|elem| contains_resource(elem, resource_names, type_params))
            {
                Some(ty.clone())
            } else {
                None
            }
        }
        Type::Option(inner) => {
            find_unsupported_resource_collection(inner, resource_names, type_params)
        }
        Type::Result(ok, err) => {
            find_unsupported_resource_collection(ok, resource_names, type_params)
                .or_else(|| find_unsupported_resource_collection(err, resource_names, type_params))
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

pub(crate) fn contains_named_resource(
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

pub(crate) fn validate_no_resource_collections(
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
        ensure_no_unsupported_resource_collections(&func.ret, None, resource_names, &type_params)?;
        for param in &func.params {
            ensure_no_unsupported_resource_collections(
                &param.ty,
                None,
                resource_names,
                &type_params,
            )?;
        }
    }
    Ok(())
}
