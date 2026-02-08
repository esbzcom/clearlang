use anyhow::Result;
use clg_ast::{Program, Span, Type};
use std::collections::HashSet;

use super::super::expr::expr_span;
use super::super::type_params::validate_type_params;
use super::super::{AliasMap, TraitEnv, TypeDefs};
use crate::errors::TyperError;

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

pub(crate) fn validate_supported_types(
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
        let mut impl_params =
            validate_type_params(&imp.type_params, type_defs, aliases, trait_env)?;
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
