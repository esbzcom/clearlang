use anyhow::Result;
use clg_ast::Type;
use std::collections::HashSet;

use super::{substitute_type, AliasMap, TypeDefs};
use crate::errors::TyperError;

pub(crate) fn resolve_aliases(
    ty: &Type,
    aliases: &AliasMap,
    visiting: &mut Vec<String>,
    type_params: &HashSet<String>,
) -> Result<Type> {
    match ty {
        Type::Named { name, args } => {
            if args.is_empty() && type_params.contains(name.as_str()) {
                return Ok(Type::Named {
                    name: name.clone(),
                    args: Vec::new(),
                });
            }
            let resolved_args = args
                .iter()
                .map(|arg| resolve_aliases(arg, aliases, visiting, type_params))
                .collect::<Result<Vec<_>>>()?;
            if let Some(def) = aliases.get(name) {
                if def.type_params.len() != resolved_args.len() {
                    return Ok(Type::Named {
                        name: name.clone(),
                        args: resolved_args,
                    });
                }
                if visiting.iter().any(|n| n == name) {
                    return Err(TyperError::cyclic_alias(name, def.span).into());
                }
                visiting.push(name.clone());
                let mut subst = std::collections::HashMap::with_capacity(def.type_params.len());
                for (param, arg) in def.type_params.iter().zip(resolved_args.iter()) {
                    subst.insert(param.clone(), arg.clone());
                }
                let instantiated = substitute_type(&def.base, &subst);
                let resolved = resolve_aliases(&instantiated, aliases, visiting, type_params)?;
                visiting.pop();
                return Ok(resolved);
            }
            Ok(Type::Named {
                name: name.clone(),
                args: resolved_args,
            })
        }
        Type::Option(inner) => Ok(Type::Option(Box::new(resolve_aliases(
            inner,
            aliases,
            visiting,
            type_params,
        )?))),
        Type::Result(ok, err) => Ok(Type::Result(
            Box::new(resolve_aliases(ok, aliases, visiting, type_params)?),
            Box::new(resolve_aliases(err, aliases, visiting, type_params)?),
        )),
        Type::List(inner) => Ok(Type::List(Box::new(resolve_aliases(
            inner,
            aliases,
            visiting,
            type_params,
        )?))),
        Type::Set(inner) => Ok(Type::Set(Box::new(resolve_aliases(
            inner,
            aliases,
            visiting,
            type_params,
        )?))),
        Type::Slice(inner) => Ok(Type::Slice(Box::new(resolve_aliases(
            inner,
            aliases,
            visiting,
            type_params,
        )?))),
        Type::Map(k, v) => Ok(Type::Map(
            Box::new(resolve_aliases(k, aliases, visiting, type_params)?),
            Box::new(resolve_aliases(v, aliases, visiting, type_params)?),
        )),
        Type::Array(inner, len) => Ok(Type::Array(
            Box::new(resolve_aliases(inner, aliases, visiting, type_params)?),
            *len,
        )),
        Type::Tuple(elements) => {
            let resolved_elems = elements
                .iter()
                .map(|elem| resolve_aliases(elem, aliases, visiting, type_params))
                .collect::<Result<Vec<_>>>()?;
            Ok(Type::Tuple(resolved_elems))
        }
        _ => Ok(ty.clone()),
    }
}

fn alias_name<'a>(ty: &'a Type, aliases: &'a AliasMap) -> Option<&'a str> {
    match ty {
        Type::Named { name, args } => {
            let def = aliases.get(name.as_str())?;
            if def.type_params.len() == args.len() {
                Some(name.as_str())
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn base_type(ty: &Type, aliases: &AliasMap) -> Result<Type> {
    let mut visiting = Vec::new();
    resolve_aliases(ty, aliases, &mut visiting, &HashSet::new())
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

pub(crate) fn is_resource_type(
    ty: &Type,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
) -> Result<bool> {
    fn contains_resource_in_type(ty: &Type, type_defs: &TypeDefs) -> bool {
        match ty {
            Type::Named { name, args } => {
                type_defs.resources.contains(name.as_str())
                    || args
                        .iter()
                        .any(|arg| contains_resource_in_type(arg, type_defs))
            }
            Type::Option(inner) | Type::List(inner) | Type::Set(inner) | Type::Slice(inner) => {
                contains_resource_in_type(inner, type_defs)
            }
            Type::Result(ok, err) | Type::Map(ok, err) => {
                contains_resource_in_type(ok, type_defs)
                    || contains_resource_in_type(err, type_defs)
            }
            Type::Array(inner, _) => contains_resource_in_type(inner, type_defs),
            Type::Tuple(elements) => elements
                .iter()
                .any(|elem| contains_resource_in_type(elem, type_defs)),
            _ => false,
        }
    }

    let resolved = base_type(ty, aliases)?;
    Ok(match &resolved {
        Type::Named { name, args } if args.is_empty() => {
            type_defs.resources.contains(name.as_str())
        }
        Type::Option(_)
        | Type::Result(_, _)
        | Type::List(_)
        | Type::Set(_)
        | Type::Map(_, _)
        | Type::Array(_, _)
        | Type::Slice(_)
        | Type::Tuple(_)
        | Type::Named { .. } => contains_resource_in_type(&resolved, type_defs),
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

pub(crate) fn binding_compatible(
    expected: &Type,
    actual: &Type,
    aliases: &AliasMap,
) -> Result<bool> {
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
