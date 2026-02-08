use anyhow::Result;
use clg_ast::Type;

use super::{AliasMap, TypeDefs};
use crate::errors::TyperError;

pub(crate) fn resolve_aliases(ty: &Type, aliases: &AliasMap, visiting: &mut Vec<String>) -> Result<Type> {
    match ty {
        Type::Named { name, args } => {
            if args.is_empty() {
                if let Some(def) = aliases.get(name) {
                    if visiting.iter().any(|n| n == name) {
                        return Err(TyperError::cyclic_alias(name, def.span).into());
                    }
                    visiting.push(name.clone());
                    let resolved = resolve_aliases(&def.base, aliases, visiting)?;
                    visiting.pop();
                    return Ok(resolved);
                }
            }
            let resolved_args = args
                .iter()
                .map(|arg| resolve_aliases(arg, aliases, visiting))
                .collect::<Result<Vec<_>>>()?;
            Ok(Type::Named {
                name: name.clone(),
                args: resolved_args,
            })
        }
        Type::Option(inner) => Ok(Type::Option(Box::new(resolve_aliases(
            inner, aliases, visiting,
        )?))),
        Type::Result(ok, err) => Ok(Type::Result(
            Box::new(resolve_aliases(ok, aliases, visiting)?),
            Box::new(resolve_aliases(err, aliases, visiting)?),
        )),
        Type::List(inner) => Ok(Type::List(Box::new(resolve_aliases(
            inner, aliases, visiting,
        )?))),
        Type::Set(inner) => Ok(Type::Set(Box::new(resolve_aliases(
            inner, aliases, visiting,
        )?))),
        Type::Slice(inner) => Ok(Type::Slice(Box::new(resolve_aliases(
            inner, aliases, visiting,
        )?))),
        Type::Map(k, v) => Ok(Type::Map(
            Box::new(resolve_aliases(k, aliases, visiting)?),
            Box::new(resolve_aliases(v, aliases, visiting)?),
        )),
        Type::Array(inner, len) => Ok(Type::Array(
            Box::new(resolve_aliases(inner, aliases, visiting)?),
            *len,
        )),
        Type::Tuple(elements) => {
            let resolved_elems = elements
                .iter()
                .map(|elem| resolve_aliases(elem, aliases, visiting))
                .collect::<Result<Vec<_>>>()?;
            Ok(Type::Tuple(resolved_elems))
        }
        _ => Ok(ty.clone()),
    }
}

fn alias_name<'a>(ty: &'a Type, aliases: &'a AliasMap) -> Option<&'a str> {
    match ty {
        Type::Named { name, args } if args.is_empty() && aliases.contains_key(name.as_str()) => {
            Some(name.as_str())
        }
        _ => None,
    }
}

pub(crate) fn base_type(ty: &Type, aliases: &AliasMap) -> Result<Type> {
    let mut visiting = Vec::new();
    resolve_aliases(ty, aliases, &mut visiting)
}

pub(crate) fn base_types_match(expected: &Type, actual: &Type, aliases: &AliasMap) -> Result<bool> {
    let expected = base_type(expected, aliases)?;
    let actual = base_type(actual, aliases)?;
    Ok(match (&expected, &actual) {
        (Type::Array(exp_inner, _), Type::Array(act_inner, _)) => {
            base_types_match(exp_inner, act_inner, aliases)?
        }
        (Type::Slice(exp_inner), Type::Slice(act_inner)) => {
            base_types_match(exp_inner, act_inner, aliases)?
        }
        _ => expected == actual,
    })
}

pub(crate) fn is_resource_type(ty: &Type, aliases: &AliasMap, type_defs: &TypeDefs) -> Result<bool> {
    Ok(match base_type(ty, aliases)? {
        Type::Named { name, args } if args.is_empty() => {
            type_defs.resources.contains(name.as_str())
        }
        _ => false,
    })
}

pub(crate) fn refinement_loss(expected: &Type, actual: &Type, aliases: &AliasMap) -> bool {
    match (expected, actual) {
        (Type::Option(exp), Type::Option(act)) => refinement_loss(exp, act, aliases),
        (Type::Result(exp_ok, exp_err), Type::Result(act_ok, act_err)) => {
            refinement_loss(exp_ok, act_ok, aliases) || refinement_loss(exp_err, act_err, aliases)
        }
        (Type::List(exp), Type::List(act)) => refinement_loss(exp, act, aliases),
        (Type::Set(exp), Type::Set(act)) => refinement_loss(exp, act, aliases),
        (Type::Map(exp_k, exp_v), Type::Map(act_k, act_v)) => {
            refinement_loss(exp_k, act_k, aliases) || refinement_loss(exp_v, act_v, aliases)
        }
        _ => alias_name(actual, aliases).is_some() && alias_name(expected, aliases).is_none(),
    }
}

pub(crate) fn binding_compatible(expected: &Type, actual: &Type, aliases: &AliasMap) -> Result<bool> {
    if expected == actual {
        return Ok(true);
    }
    if matches!(expected, Type::Array(_, _)) && matches!(actual, Type::Array(_, _)) {
        return base_types_match(expected, actual, aliases);
    }
    if matches!(expected, Type::Slice(_)) && matches!(actual, Type::Slice(_)) {
        return base_types_match(expected, actual, aliases);
    }
    if alias_name(expected, aliases).is_some() && alias_name(actual, aliases).is_none() {
        return base_types_match(expected, actual, aliases);
    }
    Ok(false)
}
