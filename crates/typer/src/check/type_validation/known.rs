use anyhow::Result;
use clg_ast::{Program, Span, Type};
use std::collections::HashSet;

use super::super::type_params::validate_type_params;
use super::super::{AliasMap, StdTypeMap, TraitEnv, TypeDefs};
use crate::errors::TyperError;

pub(crate) fn ensure_known_type(
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
                    return Err(
                        TyperError::type_arg_count_mismatch(name, 0, args.len(), span).into(),
                    );
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
            if type_defs.resources.contains(name) {
                if !args.is_empty() {
                    return Err(
                        TyperError::type_arg_count_mismatch(name, 0, args.len(), span).into(),
                    );
                }
                return Ok(());
            }
            if std_types.contains_key(name) {
                if !args.is_empty() {
                    return Err(
                        TyperError::type_arg_count_mismatch(name, 0, args.len(), span).into(),
                    );
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
        Type::Fn { params, ret } => {
            for param in params {
                ensure_known_type(param, aliases, type_defs, type_params, std_types, span)?;
            }
            ensure_known_type(ret, aliases, type_defs, type_params, std_types, span)
        }
        _ => Ok(()),
    }
}

pub(crate) fn validate_known_types(
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
