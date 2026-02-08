use anyhow::Result;
use clg_ast::{ParamKind, Program, Type};
use std::collections::{HashMap, HashSet};

use super::expr::{max_effect, type_of, ResourceTracker};
use super::refinement_predicate::predicate_is_contradiction;
use super::type_resolve::resolve_aliases;
use super::type_validation::{
    contains_named_resource, ensure_equatable_collection_keys, ensure_no_resource_collections,
};
use super::{
    base_type, AliasDef, AliasMap, BoundsMap, EffectLevel, FnSig, LocalBinding, TraitEnv, TypeDefs,
};
use crate::errors::TyperError;

pub(super) fn build_alias_map(program: &Program, type_defs: &TypeDefs) -> Result<AliasMap> {
    let mut aliases: AliasMap = HashMap::with_capacity(program.refined_aliases.len());

    for alias in &program.refined_aliases {
        if !alias.type_params.is_empty() {
            return Err(
                TyperError::generic_alias_not_supported(alias.name.as_str(), alias.span).into(),
            );
        }
        if aliases.contains_key(alias.name.as_str()) {
            return Err(TyperError::duplicate_type(&alias.name, alias.name_span).into());
        }
        if type_defs.resources.contains(alias.name.as_str()) {
            return Err(
                TyperError::type_conflicts_with_resource(&alias.name, alias.name_span).into(),
            );
        }
        if type_defs.structs.contains_key(alias.name.as_str())
            || type_defs.enums.contains_key(alias.name.as_str())
        {
            return Err(TyperError::duplicate_type(&alias.name, alias.name_span).into());
        }
        let mut visited = Vec::with_capacity(program.refined_aliases.len());
        let resolved_base = resolve_aliases(&alias.base, &aliases, &mut visited)?;
        if contains_named_resource(&resolved_base, &type_defs.resources, &HashSet::new()) {
            return Err(TyperError::refined_resource_not_supported(
                alias.name.as_str(),
                alias.span,
            )
            .into());
        }
        ensure_no_resource_collections(
            &resolved_base,
            Some(alias.span),
            &type_defs.resources,
            &HashSet::new(),
        )?;
        ensure_equatable_collection_keys(
            &resolved_base,
            Some(alias.span),
            type_defs,
            &aliases,
            &HashSet::new(),
        )?;

        aliases.insert(
            alias.name.clone(),
            AliasDef {
                base: resolved_base,
                predicate: alias.predicate.clone(),
                binder: alias.binder.clone(),
                span: alias.span,
            },
        );
    }
    Ok(aliases)
}

pub(super) fn validate_alias_predicates(
    aliases: &AliasMap,
    fns: &HashMap<&str, FnSig>,
    type_defs: &TypeDefs,
    trait_env: &TraitEnv,
) -> Result<()> {
    for (name, def) in aliases {
        let mut env: HashMap<&str, LocalBinding> =
            HashMap::with_capacity(def.binder.as_ref().map(|_| 1).unwrap_or(0));
        if let Some(binder) = def.binder.as_ref() {
            env.insert(
                binder.as_str(),
                LocalBinding {
                    ty: def.base.clone(),
                    kind: ParamKind::Borrow,
                },
            );
        }
        let mut tracker = ResourceTracker::new();
        let empty_bounds: BoundsMap = HashMap::new();
        let pred_ty = type_of(
            &def.predicate,
            &env,
            &mut tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            &HashSet::new(),
            &empty_bounds,
            0,
            None,
        )?;
        if base_type(&pred_ty, aliases)? != Type::Bool {
            return Err(TyperError::alias_predicate_not_bool(name.as_str(), def.span).into());
        }
        if let Err(err) = max_effect(&def.predicate, fns, trait_env, EffectLevel::Pure) {
            if let Some(typer) = err.downcast_ref::<TyperError>() {
                if typer.code == "T401" {
                    return Err(TyperError::alias_predicate_impure(name.as_str(), def.span).into());
                }
            }
            return Err(err);
        }
        if predicate_is_contradiction(&def.predicate, def.binder.as_deref(), &def.base) {
            return Err(TyperError::alias_predicate_unsat(name.as_str(), def.span).into());
        }
    }
    Ok(())
}
