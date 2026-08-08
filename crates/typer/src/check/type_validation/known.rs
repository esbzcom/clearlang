use anyhow::Result;
use clg_ast::{Program, Span, Type};
use std::collections::HashSet;

use super::super::type_params::validate_type_params;
use super::super::{AliasMap, StdTypeMap, TraitEnv, TypeDefs};
use super::contains_named_resource;
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
                let expected = aliases
                    .get(name)
                    .map(|alias| alias.type_params.len())
                    .unwrap_or(0);
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
    let mut contract_names = HashSet::new();
    for contract in &program.contracts {
        if !contract_names.insert(contract.name.as_str()) {
            return Err(TyperError::duplicate_contract_state_field(
                &contract.name,
                contract.name_span,
            )
            .into());
        }
        let mut field_names = HashSet::new();
        for field in &contract.fields {
            if !field_names.insert(field.name.as_str()) {
                return Err(
                    TyperError::duplicate_contract_state_field(&field.name, field.span).into(),
                );
            }
            ensure_known_type(
                &field.ty,
                aliases,
                type_defs,
                &HashSet::new(),
                std_types,
                Some(field.span),
            )?;
            if !is_persistable_state_type(&field.ty)
                || contains_named_resource(&field.ty, &type_defs.resources, &HashSet::new())
            {
                return Err(TyperError::invalid_contract_state_type(&field.ty, field.span).into());
            }
        }
        let mut event_names = HashSet::new();
        for event in &contract.events {
            if !event_names.insert(event.name.as_str()) {
                return Err(
                    TyperError::duplicate_contract_event(&event.name, event.name_span).into(),
                );
            }
            let mut event_field_names = HashSet::new();
            for field in &event.fields {
                if !event_field_names.insert(field.name.as_str()) {
                    return Err(TyperError::duplicate_contract_event_field(
                        &field.name,
                        field.span,
                    )
                    .into());
                }
                ensure_known_type(
                    &field.ty,
                    aliases,
                    type_defs,
                    &HashSet::new(),
                    std_types,
                    Some(field.span),
                )?;
                if !is_persistable_state_type(&field.ty)
                    || contains_named_resource(&field.ty, &type_defs.resources, &HashSet::new())
                {
                    return Err(
                        TyperError::invalid_contract_event_type(&field.ty, field.span).into(),
                    );
                }
            }
        }
    }
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
        let alias_type_params: HashSet<String> = alias.type_params.iter().cloned().collect();
        ensure_known_type(
            &alias.base,
            aliases,
            type_defs,
            &alias_type_params,
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

fn is_persistable_state_type(ty: &Type) -> bool {
    match ty {
        Type::Fn { .. } => false,
        Type::Option(inner)
        | Type::List(inner)
        | Type::Set(inner)
        | Type::Slice(inner)
        | Type::Array(inner, _) => is_persistable_state_type(inner),
        Type::Result(ok, err) | Type::Map(ok, err) => {
            is_persistable_state_type(ok) && is_persistable_state_type(err)
        }
        Type::Tuple(elements) => elements.iter().all(is_persistable_state_type),
        Type::Named { args, .. } => args.iter().all(is_persistable_state_type),
        _ => true,
    }
}
