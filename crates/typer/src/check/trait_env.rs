use anyhow::Result;
use clg_ast::{Func, Program, TraitMethod, Type};
use std::collections::{HashMap, HashSet};

use super::expr::expr_span;
use super::type_params::{
    substitute_type, type_param_names, validate_bounds, validate_type_params,
};
use super::type_validation::ensure_known_type;
use super::{AliasMap, ImplInfo, StdTypeMap, TraitEnv, TraitInfo, TypeDefs, TypeSubst};
use crate::errors::TyperError;

pub(super) fn build_trait_env<'a>(
    program: &'a Program,
    aliases: &AliasMap,
    type_defs: &TypeDefs<'a>,
    std_types: &StdTypeMap,
) -> Result<TraitEnv<'a>> {
    let mut traits: HashMap<&'a str, TraitInfo<'a>> = HashMap::with_capacity(program.traits.len());

    for decl in &program.traits {
        if traits.contains_key(decl.name.as_str()) {
            return Err(TyperError::duplicate_trait(&decl.name, decl.name_span).into());
        }
        if !decl.type_params.is_empty() {
            return Err(
                TyperError::trait_type_params_not_supported(decl.name.as_str(), decl.span).into(),
            );
        }
        let mut methods: HashMap<&str, &TraitMethod> = HashMap::with_capacity(decl.methods.len());
        let mut self_params: HashSet<String> = HashSet::with_capacity(1);
        self_params.insert("Self".to_string());
        for method in &decl.methods {
            if methods.insert(method.name.as_str(), method).is_some() {
                return Err(TyperError::duplicate_trait_method(
                    decl.name.as_str(),
                    method.name.as_str(),
                    method.span,
                )
                .into());
            }
            for param in &method.params {
                ensure_known_type(
                    &param.ty,
                    aliases,
                    type_defs,
                    &self_params,
                    std_types,
                    Some(method.span),
                )?;
            }
            ensure_known_type(
                &method.ret,
                aliases,
                type_defs,
                &self_params,
                std_types,
                Some(method.span),
            )?;
        }
        traits.insert(decl.name.as_str(), TraitInfo { decl, methods });
    }

    let trait_env = TraitEnv {
        traits,
        impls: Vec::new(),
    };

    let mut impls: Vec<ImplInfo<'a>> = Vec::with_capacity(program.impls.len());
    for decl in &program.impls {
        let Some(trait_info) = trait_env.traits.get(decl.trait_name.as_str()) else {
            return Err(TyperError::unknown_trait(&decl.trait_name, decl.trait_name_span).into());
        };
        let type_params = validate_type_params(&decl.type_params, type_defs, aliases, &trait_env)?;
        let _ = validate_bounds(&decl.where_bounds, &type_params, &trait_env)?;
        ensure_known_type(
            &decl.for_type,
            aliases,
            type_defs,
            &type_params,
            std_types,
            Some(decl.span),
        )?;

        let mut methods: HashMap<&str, &Func> = HashMap::with_capacity(decl.methods.len());
        for method in &decl.methods {
            if !method.type_params.is_empty() || !method.where_bounds.is_empty() {
                return Err(TyperError::impl_method_generics_not_supported(
                    method.name.as_str(),
                    method.effect_span,
                )
                .into());
            }
            if methods.insert(method.name.as_str(), method).is_some() {
                return Err(TyperError::duplicate_trait_method(
                    decl.trait_name.as_str(),
                    method.name.as_str(),
                    expr_span(&method.body),
                )
                .into());
            }
        }

        for (name, trait_method) in &trait_info.methods {
            let Some(impl_method) = methods.get(name) else {
                return Err(TyperError::trait_method_missing(
                    decl.trait_name.as_str(),
                    name,
                    decl.span,
                )
                .into());
            };
            validate_impl_method_signature(
                decl.trait_name.as_str(),
                trait_method,
                impl_method,
                &decl.for_type,
            )?;
        }
        for name in methods.keys() {
            if !trait_info.methods.contains_key(name) {
                return Err(TyperError::trait_method_extra(
                    decl.trait_name.as_str(),
                    name,
                    decl.span,
                )
                .into());
            }
        }

        impls.push(ImplInfo { decl, methods });
    }

    let env = TraitEnv {
        traits: trait_env.traits,
        impls,
    };
    validate_impl_coherence(&env)?;
    Ok(env)
}

fn validate_impl_method_signature(
    trait_name: &str,
    trait_method: &TraitMethod,
    impl_method: &Func,
    for_type: &Type,
) -> Result<()> {
    if trait_method.effect != impl_method.effect {
        return Err(TyperError::trait_method_signature_mismatch(
            trait_name,
            trait_method.name.as_str(),
            trait_method.span,
        )
        .into());
    }
    if trait_method.params.len() != impl_method.params.len() {
        return Err(TyperError::trait_method_signature_mismatch(
            trait_name,
            trait_method.name.as_str(),
            trait_method.span,
        )
        .into());
    }
    let mut subst: TypeSubst = HashMap::new();
    subst.insert("Self".to_string(), for_type.clone());
    for (tparam, iparam) in trait_method.params.iter().zip(impl_method.params.iter()) {
        let trait_ty = substitute_type(&tparam.ty, &subst);
        let impl_ty = substitute_type(&iparam.ty, &subst);
        if trait_ty != impl_ty || tparam.kind != iparam.kind {
            return Err(TyperError::trait_method_signature_mismatch(
                trait_name,
                trait_method.name.as_str(),
                trait_method.span,
            )
            .into());
        }
    }
    let trait_ret = substitute_type(&trait_method.ret, &subst);
    let impl_ret = substitute_type(&impl_method.ret, &subst);
    if trait_ret != impl_ret {
        return Err(TyperError::trait_method_signature_mismatch(
            trait_name,
            trait_method.name.as_str(),
            trait_method.span,
        )
        .into());
    }
    Ok(())
}

fn validate_impl_coherence(trait_env: &TraitEnv<'_>) -> Result<()> {
    for (trait_name, _) in &trait_env.traits {
        let impls: Vec<&ImplInfo<'_>> = trait_env
            .impls
            .iter()
            .filter(|info| info.decl.trait_name == *trait_name)
            .collect();
        for i in 0..impls.len() {
            for j in (i + 1)..impls.len() {
                let left = impls[i].decl;
                let right = impls[j].decl;
                let left_params = type_param_names(&left.type_params);
                let right_params = type_param_names(&right.type_params);
                if types_overlap(&left.for_type, &left_params, &right.for_type, &right_params) {
                    return Err(
                        TyperError::overlapping_impl(trait_name, left.span, right.span).into(),
                    );
                }
            }
        }
    }
    Ok(())
}

fn types_overlap(
    left: &Type,
    left_params: &HashSet<String>,
    right: &Type,
    right_params: &HashSet<String>,
) -> bool {
    fn overlaps(
        left: &Type,
        left_params: &HashSet<String>,
        right: &Type,
        right_params: &HashSet<String>,
        seen: &mut HashMap<String, Type>,
    ) -> bool {
        match (left, right) {
            (Type::Named { name, args }, _) if args.is_empty() && left_params.contains(name) => {
                if let Some(existing) = seen.get(name) {
                    return existing == right;
                }
                seen.insert(name.clone(), right.clone());
                true
            }
            (_, Type::Named { name, args }) if args.is_empty() && right_params.contains(name) => {
                if let Some(existing) = seen.get(name) {
                    return existing == left;
                }
                seen.insert(name.clone(), left.clone());
                true
            }
            (Type::Named { name: ln, args: la }, Type::Named { name: rn, args: ra })
                if ln == rn && la.len() == ra.len() =>
            {
                la.iter()
                    .zip(ra.iter())
                    .all(|(l, r)| overlaps(l, left_params, r, right_params, seen))
            }
            (Type::Option(l), Type::Option(r))
            | (Type::List(l), Type::List(r))
            | (Type::Set(l), Type::Set(r)) => overlaps(l, left_params, r, right_params, seen),
            (Type::Result(lk, lv), Type::Result(rk, rv))
            | (Type::Map(lk, lv), Type::Map(rk, rv)) => {
                overlaps(lk, left_params, rk, right_params, seen)
                    && overlaps(lv, left_params, rv, right_params, seen)
            }
            (Type::Array(l, _), Type::Array(r, _)) => {
                overlaps(l, left_params, r, right_params, seen)
            }
            (Type::Slice(l), Type::Slice(r)) => overlaps(l, left_params, r, right_params, seen),
            (Type::Tuple(l), Type::Tuple(r)) if l.len() == r.len() => l
                .iter()
                .zip(r.iter())
                .all(|(lty, rty)| overlaps(lty, left_params, rty, right_params, seen)),
            _ => left == right,
        }
    }

    overlaps(left, left_params, right, right_params, &mut HashMap::new())
}
