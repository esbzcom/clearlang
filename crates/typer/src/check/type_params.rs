use anyhow::Result;
use clg_ast::{TraitBound, Type, TypeParam};
use std::collections::{HashMap, HashSet};

use super::{base_type, AliasMap, BoundsMap, TraitEnv, TypeDefs, TypeSubst};
use crate::errors::TyperError;

fn builtin_type_names() -> HashSet<&'static str> {
    [
        "Int", "U8", "U64", "U128", "U256", "Bool", "String", "Bytes", "Option", "Result", "List",
        "Set", "Map", "Self",
    ]
    .into_iter()
    .collect()
}

pub(crate) fn type_param_names(params: &[TypeParam]) -> HashSet<String> {
    params.iter().map(|p| p.name.clone()).collect()
}

pub(super) fn validate_type_params(
    params: &[TypeParam],
    type_defs: &TypeDefs,
    aliases: &AliasMap,
    trait_env: &TraitEnv,
) -> Result<HashSet<String>> {
    let reserved = builtin_type_names();
    let mut names: HashSet<String> = HashSet::with_capacity(params.len());
    for param in params {
        if !names.insert(param.name.clone()) {
            return Err(TyperError::duplicate_type_param(&param.name, param.span).into());
        }
        if reserved.contains(param.name.as_str())
            || type_defs.resources.contains(param.name.as_str())
            || type_defs.structs.contains_key(param.name.as_str())
            || type_defs.enums.contains_key(param.name.as_str())
            || aliases.contains_key(param.name.as_str())
            || trait_env.traits.contains_key(param.name.as_str())
        {
            return Err(TyperError::type_param_conflict(&param.name, param.span).into());
        }
    }
    Ok(names)
}

pub(super) fn validate_bounds(
    bounds: &[TraitBound],
    type_params: &HashSet<String>,
    trait_env: &TraitEnv,
) -> Result<BoundsMap> {
    let mut map: BoundsMap = HashMap::new();
    for bound in bounds {
        if !type_params.contains(bound.param.as_str()) {
            return Err(TyperError::unknown_type_param(&bound.param, bound.span).into());
        }
        if !trait_env.traits.contains_key(bound.trait_name.as_str()) {
            return Err(TyperError::unknown_trait(&bound.trait_name, bound.span).into());
        }
        map.entry(bound.param.clone())
            .or_default()
            .insert(bound.trait_name.clone());
    }
    Ok(map)
}

pub(crate) fn substitute_type(ty: &Type, subst: &TypeSubst) -> Type {
    match ty {
        Type::Named { name, args } => {
            if args.is_empty() {
                if let Some(mapped) = subst.get(name) {
                    return mapped.clone();
                }
            }
            let mapped_args = args.iter().map(|arg| substitute_type(arg, subst)).collect();
            Type::Named {
                name: name.clone(),
                args: mapped_args,
            }
        }
        Type::Option(inner) => Type::Option(Box::new(substitute_type(inner, subst))),
        Type::Result(ok, err) => Type::Result(
            Box::new(substitute_type(ok, subst)),
            Box::new(substitute_type(err, subst)),
        ),
        Type::List(inner) => Type::List(Box::new(substitute_type(inner, subst))),
        Type::Set(inner) => Type::Set(Box::new(substitute_type(inner, subst))),
        Type::Map(k, v) => Type::Map(
            Box::new(substitute_type(k, subst)),
            Box::new(substitute_type(v, subst)),
        ),
        Type::Array(inner, len) => Type::Array(Box::new(substitute_type(inner, subst)), *len),
        Type::Slice(inner) => Type::Slice(Box::new(substitute_type(inner, subst))),
        Type::Tuple(elements) => {
            Type::Tuple(elements.iter().map(|elem| substitute_type(elem, subst)).collect())
        }
        _ => ty.clone(),
    }
}

pub(crate) fn unify_type_params(
    pattern: &Type,
    actual: &Type,
    params: &HashSet<String>,
    subst: &mut TypeSubst,
    aliases: &AliasMap,
) -> Result<()> {
    match pattern {
        Type::Named { name, args } if args.is_empty() && params.contains(name.as_str()) => {
            if let Some(existing) = subst.get(name) {
                if base_type(existing, aliases)? != base_type(actual, aliases)? {
                    return Err(TyperError::type_param_mismatch(
                        name,
                        existing.clone(),
                        actual.clone(),
                    )
                    .into());
                }
            } else {
                subst.insert(name.clone(), actual.clone());
            }
            Ok(())
        }
        Type::Option(inner) => match actual {
            Type::Option(act_inner) => unify_type_params(inner, act_inner, params, subst, aliases),
            _ => Ok(()),
        },
        Type::Result(ok, err) => match actual {
            Type::Result(act_ok, act_err) => {
                unify_type_params(ok, act_ok, params, subst, aliases)?;
                unify_type_params(err, act_err, params, subst, aliases)
            }
            _ => Ok(()),
        },
        Type::List(inner) => match actual {
            Type::List(act_inner) => unify_type_params(inner, act_inner, params, subst, aliases),
            _ => Ok(()),
        },
        Type::Set(inner) => match actual {
            Type::Set(act_inner) => unify_type_params(inner, act_inner, params, subst, aliases),
            _ => Ok(()),
        },
        Type::Map(k, v) => match actual {
            Type::Map(act_k, act_v) => {
                unify_type_params(k, act_k, params, subst, aliases)?;
                unify_type_params(v, act_v, params, subst, aliases)
            }
            _ => Ok(()),
        },
        Type::Array(inner, _) => match actual {
            Type::Array(act_inner, _) => unify_type_params(inner, act_inner, params, subst, aliases),
            _ => Ok(()),
        },
        Type::Slice(inner) => match actual {
            Type::Slice(act_inner) => unify_type_params(inner, act_inner, params, subst, aliases),
            _ => Ok(()),
        },
        Type::Tuple(elements) => match actual {
            Type::Tuple(act_elems) if elements.len() == act_elems.len() => {
                for (left, right) in elements.iter().zip(act_elems.iter()) {
                    unify_type_params(left, right, params, subst, aliases)?;
                }
                Ok(())
            }
            _ => Ok(()),
        },
        Type::Named { name, args } => match actual {
            Type::Named {
                name: act_name,
                args: act_args,
            } if name == act_name && args.len() == act_args.len() => {
                for (left, right) in args.iter().zip(act_args.iter()) {
                    unify_type_params(left, right, params, subst, aliases)?;
                }
                Ok(())
            }
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

pub(crate) fn type_contains_params(ty: &Type, params: &HashSet<String>) -> bool {
    match ty {
        Type::Named { name, args } => {
            params.contains(name.as_str())
                || args.iter().any(|arg| type_contains_params(arg, params))
        }
        Type::Option(inner) | Type::List(inner) | Type::Set(inner) | Type::Slice(inner) => {
            type_contains_params(inner, params)
        }
        Type::Result(ok, err) | Type::Map(ok, err) => {
            type_contains_params(ok, params) || type_contains_params(err, params)
        }
        Type::Array(inner, _) => type_contains_params(inner, params),
        Type::Tuple(elements) => elements.iter().any(|elem| type_contains_params(elem, params)),
        _ => false,
    }
}

#[allow(dead_code)]
fn collect_type_params_in_type(ty: &Type, params: &HashSet<String>, used: &mut HashSet<String>) {
    match ty {
        Type::Named { name, args } => {
            if args.is_empty() && params.contains(name.as_str()) {
                used.insert(name.clone());
            }
            for arg in args {
                collect_type_params_in_type(arg, params, used);
            }
        }
        Type::Option(inner) | Type::List(inner) | Type::Set(inner) | Type::Slice(inner) => {
            collect_type_params_in_type(inner, params, used);
        }
        Type::Result(ok, err) | Type::Map(ok, err) => {
            collect_type_params_in_type(ok, params, used);
            collect_type_params_in_type(err, params, used);
        }
        Type::Array(inner, _) => collect_type_params_in_type(inner, params, used),
        Type::Tuple(elements) => {
            for elem in elements {
                collect_type_params_in_type(elem, params, used);
            }
        }
        _ => {}
    }
}
