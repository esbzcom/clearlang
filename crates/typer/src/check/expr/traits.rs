use anyhow::Result;
use clg_ast::{Span, Type};
use std::collections::{HashMap, HashSet};

use super::super::{
    base_type, type_contains_params, type_param_names, AliasMap, BoundsMap, TraitEnv, TypeSubst,
};
use super::show_ty;
use crate::errors::TyperError;

#[allow(clippy::too_many_arguments)]
pub(crate) fn ensure_trait_bound(
    ty: &Type,
    trait_name: &str,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    trait_env: &TraitEnv<'_>,
    aliases: &AliasMap,
    span: Span,
) -> Result<()> {
    if let Type::Named { name, args } = ty {
        if args.is_empty() && type_params.contains(name.as_str()) {
            if bounds
                .get(name)
                .map(|set| set.contains(trait_name))
                .unwrap_or(false)
            {
                return Ok(());
            }
            return Err(TyperError::missing_trait_bound(name, trait_name, span).into());
        }
    }
    if type_contains_params(ty, type_params) {
        let rendered = show_ty(ty.clone());
        return Err(TyperError::missing_trait_bound(&rendered, trait_name, span).into());
    }
    if trait_impl_exists(trait_name, ty, trait_env, aliases)? {
        Ok(())
    } else {
        let rendered = show_ty(ty.clone());
        Err(TyperError::missing_trait_bound(&rendered, trait_name, span).into())
    }
}

pub(crate) fn trait_impl_exists(
    trait_name: &str,
    ty: &Type,
    trait_env: &TraitEnv<'_>,
    aliases: &AliasMap,
) -> Result<bool> {
    fn inner(
        trait_name: &str,
        ty: &Type,
        trait_env: &TraitEnv<'_>,
        aliases: &AliasMap,
        visiting: &mut HashSet<String>,
    ) -> Result<bool> {
        let key = format!("{}::{}", trait_name, show_ty(ty.clone()));
        if !visiting.insert(key.clone()) {
            return Ok(true);
        }
        let mut matches = 0usize;
        for imp in &trait_env.impls {
            if imp.decl.trait_name != trait_name {
                continue;
            }
            let impl_params = type_param_names(&imp.decl.type_params);
            let mut subst: TypeSubst = HashMap::new();
            if !type_pattern_matches(&imp.decl.for_type, ty, &impl_params, &mut subst, aliases)? {
                continue;
            }
            if impl_params.iter().any(|p| !subst.contains_key(p)) {
                continue;
            }
            let mut ok = true;
            for bound in &imp.decl.where_bounds {
                let Some(bound_ty) = subst.get(&bound.param) else {
                    ok = false;
                    break;
                };
                if !inner(bound.trait_name.as_str(), bound_ty, trait_env, aliases, visiting)? {
                    ok = false;
                    break;
                }
            }
            if ok {
                matches += 1;
            }
        }
        visiting.remove(&key);
        Ok(matches > 0)
    }

    inner(
        trait_name,
        ty,
        trait_env,
        aliases,
        &mut HashSet::new(),
    )
}

pub(crate) fn type_pattern_matches(
    pattern: &Type,
    actual: &Type,
    params: &HashSet<String>,
    subst: &mut TypeSubst,
    aliases: &AliasMap,
) -> Result<bool> {
    match pattern {
        Type::Named { name, args } if args.is_empty() && params.contains(name.as_str()) => {
            if let Some(existing) = subst.get(name) {
                return Ok(base_type(existing, aliases)? == base_type(actual, aliases)?);
            }
            subst.insert(name.clone(), actual.clone());
            Ok(true)
        }
        Type::Option(inner) => match actual {
            Type::Option(act_inner) => type_pattern_matches(inner, act_inner, params, subst, aliases),
            _ => Ok(false),
        },
        Type::Result(ok, err) => match actual {
            Type::Result(act_ok, act_err) => {
                Ok(type_pattern_matches(ok, act_ok, params, subst, aliases)?
                    && type_pattern_matches(err, act_err, params, subst, aliases)?)
            }
            _ => Ok(false),
        },
        Type::List(inner) => match actual {
            Type::List(act_inner) => type_pattern_matches(inner, act_inner, params, subst, aliases),
            _ => Ok(false),
        },
        Type::Set(inner) => match actual {
            Type::Set(act_inner) => type_pattern_matches(inner, act_inner, params, subst, aliases),
            _ => Ok(false),
        },
        Type::Map(k, v) => match actual {
            Type::Map(act_k, act_v) => Ok(
                type_pattern_matches(k, act_k, params, subst, aliases)?
                    && type_pattern_matches(v, act_v, params, subst, aliases)?,
            ),
            _ => Ok(false),
        },
        Type::Array(inner, _) => match actual {
            Type::Array(act_inner, _) => type_pattern_matches(inner, act_inner, params, subst, aliases),
            _ => Ok(false),
        },
        Type::Slice(inner) => match actual {
            Type::Slice(act_inner) => type_pattern_matches(inner, act_inner, params, subst, aliases),
            _ => Ok(false),
        },
        Type::Tuple(elements) => match actual {
            Type::Tuple(act_elems) if elements.len() == act_elems.len() => {
                for (left, right) in elements.iter().zip(act_elems.iter()) {
                    if !type_pattern_matches(left, right, params, subst, aliases)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        },
        Type::Named { name, args } => match actual {
            Type::Named {
                name: act_name,
                args: act_args,
            } if name == act_name && args.len() == act_args.len() => {
                for (left, right) in args.iter().zip(act_args.iter()) {
                    if !type_pattern_matches(left, right, params, subst, aliases)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        },
        _ => Ok(base_type(pattern, aliases)? == base_type(actual, aliases)?),
    }
}
