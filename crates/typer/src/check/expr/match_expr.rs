use anyhow::Result;
use clg_ast::{Expr, MatchArm, MatchPat, ParamKind, Span, Type};
use std::collections::{HashMap, HashSet};

use super::super::{
    substitute_type, AliasMap, BoundsMap, FnSig, LocalBinding, TraitEnv, TypeDefs, TypeSubst,
};
use super::{expr_span, type_of, ResourceTracker};
use crate::errors::TyperError;

fn merge_match_trackers(
    baseline: &ResourceTracker,
    branch_trackers: &[(ResourceTracker, Span)],
    tracker: &mut ResourceTracker,
) -> Result<()> {
    if let Some((first_tracker, _)) = branch_trackers.first() {
        let mut merged = first_tracker.clone();
        merged.retain_keys_from(baseline);
        for (branch_tracker, branch_span) in branch_trackers.iter().skip(1) {
            let mut filtered = branch_tracker.clone();
            filtered.retain_keys_from(baseline);
            merged.merge_branch(&filtered, *branch_span)?;
        }
        *tracker = merged;
    } else {
        *tracker = baseline.clone();
    }
    Ok(())
}

fn resolve_alias_shallow(ty: &Type, aliases: &AliasMap) -> Result<Type> {
    let mut visiting = Vec::new();
    resolve_alias_shallow_inner(ty, aliases, &mut visiting)
}

fn resolve_alias_shallow_inner(
    ty: &Type,
    aliases: &AliasMap,
    visiting: &mut Vec<String>,
) -> Result<Type> {
    match ty {
        Type::Named { name, args } if args.is_empty() => {
            if let Some(def) = aliases.get(name) {
                if visiting.iter().any(|n| n == name) {
                    return Err(TyperError::cyclic_alias(name, def.span).into());
                }
                visiting.push(name.clone());
                let resolved = resolve_alias_shallow_inner(&def.base, aliases, visiting)?;
                visiting.pop();
                Ok(resolved)
            } else {
                Ok(ty.clone())
            }
        }
        _ => Ok(ty.clone()),
    }
}

pub(super) fn type_match_expr<'a>(
    scrutinee: &Expr,
    arms: &'a [MatchArm],
    span: Span,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    type_params: &HashSet<String>,
    bounds: &BoundsMap,
    depth: usize,
    expected: Option<&Type>,
) -> Result<Type> {
    let scrut_ty = type_of(
        scrutinee,
        env,
        tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        type_params,
        bounds,
        depth + 1,
        None,
    )?;
    let scrut_outer = resolve_alias_shallow(&scrut_ty, aliases)?;
    match scrut_outer {
        Type::Option(inner_ty) => {
            let baseline = tracker.clone();
            let mut seen_some = false;
            let mut seen_none = false;
            let mut seen_wildcard = false;
            let mut res_ty_opt: Option<Type> = None;
            let mut branch_trackers: Vec<(ResourceTracker, Span)> = Vec::new();
            for arm in arms {
                if seen_wildcard || (seen_some && seen_none) {
                    return Err(TyperError::match_unreachable_arm(span).into());
                }
                match &arm.pat {
                    MatchPat::Some(name) => {
                        if seen_some {
                            return Err(TyperError::match_duplicate_arm("Some", span).into());
                        }
                        seen_some = true;
                        if env.contains_key(name.as_str()) {
                            return Err(TyperError::binder_conflict(name, span).into());
                        }
                        let mut env2 = env.clone();
                        env2.insert(
                            name.as_str(),
                            LocalBinding {
                                ty: *inner_ty.clone(),
                                kind: ParamKind::Borrow,
                            },
                        );
                        let mut arm_tracker = baseline.clone();
                        let at = type_of(
                            &arm.expr,
                            &env2,
                            &mut arm_tracker,
                            fns,
                            trait_env,
                            aliases,
                            type_defs,
                            type_params,
                            bounds,
                            depth + 1,
                            expected,
                        )?;
                        branch_trackers.push((arm_tracker, expr_span(&arm.expr)));
                        if let Some(rt) = &res_ty_opt {
                            if &at != rt {
                                let sp = expr_span(&arm.expr);
                                return Err(
                                    TyperError::match_arm_type_mismatch(rt.clone(), at, sp).into(),
                                );
                            }
                        } else {
                            res_ty_opt = Some(at);
                        }
                    }
                    MatchPat::None => {
                        if seen_none {
                            return Err(TyperError::match_duplicate_arm("None", span).into());
                        }
                        seen_none = true;
                        let mut arm_tracker = baseline.clone();
                        let at = type_of(
                            &arm.expr,
                            env,
                            &mut arm_tracker,
                            fns,
                            trait_env,
                            aliases,
                            type_defs,
                            type_params,
                            bounds,
                            depth + 1,
                            expected,
                        )?;
                        branch_trackers.push((arm_tracker, expr_span(&arm.expr)));
                        if let Some(rt) = &res_ty_opt {
                            if &at != rt {
                                let sp = expr_span(&arm.expr);
                                return Err(
                                    TyperError::match_arm_type_mismatch(rt.clone(), at, sp).into(),
                                );
                            }
                        } else {
                            res_ty_opt = Some(at);
                        }
                    }
                    MatchPat::Wildcard => {
                        seen_wildcard = true;
                        let mut arm_tracker = baseline.clone();
                        let at = type_of(
                            &arm.expr,
                            env,
                            &mut arm_tracker,
                            fns,
                            trait_env,
                            aliases,
                            type_defs,
                            type_params,
                            bounds,
                            depth + 1,
                            expected,
                        )?;
                        branch_trackers.push((arm_tracker, expr_span(&arm.expr)));
                        if let Some(rt) = &res_ty_opt {
                            if &at != rt {
                                let sp = expr_span(&arm.expr);
                                return Err(
                                    TyperError::match_arm_type_mismatch(rt.clone(), at, sp).into(),
                                );
                            }
                        } else {
                            res_ty_opt = Some(at);
                        }
                    }
                    _ => {
                        return Err(TyperError::match_invalid_scrutinee(scrut_ty.clone(), span)
                            .into());
                    }
                }
            }
            if !seen_wildcard && !(seen_some && seen_none) {
                return Err(TyperError::match_non_exhaustive(span).into());
            }
            let result_ty = res_ty_opt.expect("match arms must not be empty");
            merge_match_trackers(&baseline, &branch_trackers, tracker)?;
            Ok(result_ty)
        }
        Type::Result(ok_ty, err_ty) => {
            let baseline = tracker.clone();
            let mut seen_ok = false;
            let mut seen_err = false;
            let mut seen_wildcard = false;
            let mut res_ty_opt: Option<Type> = None;
            let mut branch_trackers: Vec<(ResourceTracker, Span)> = Vec::new();
            for arm in arms {
                if seen_wildcard || (seen_ok && seen_err) {
                    return Err(TyperError::match_unreachable_arm(span).into());
                }
                match &arm.pat {
                    MatchPat::Ok(name) => {
                        if seen_ok {
                            return Err(TyperError::match_duplicate_arm("Ok", span).into());
                        }
                        seen_ok = true;
                        if env.contains_key(name.as_str()) {
                            return Err(TyperError::binder_conflict(name, span).into());
                        }
                        let mut env2 = env.clone();
                        env2.insert(
                            name.as_str(),
                            LocalBinding {
                                ty: *ok_ty.clone(),
                                kind: ParamKind::Borrow,
                            },
                        );
                        let mut arm_tracker = baseline.clone();
                        let at = type_of(
                            &arm.expr,
                            &env2,
                            &mut arm_tracker,
                            fns,
                            trait_env,
                            aliases,
                            type_defs,
                            type_params,
                            bounds,
                            depth + 1,
                            expected,
                        )?;
                        branch_trackers.push((arm_tracker, expr_span(&arm.expr)));
                        if let Some(rt) = &res_ty_opt {
                            if &at != rt {
                                let sp = expr_span(&arm.expr);
                                return Err(
                                    TyperError::match_arm_type_mismatch(rt.clone(), at, sp).into(),
                                );
                            }
                        } else {
                            res_ty_opt = Some(at);
                        }
                    }
                    MatchPat::Err(name) => {
                        if seen_err {
                            return Err(TyperError::match_duplicate_arm("Err", span).into());
                        }
                        seen_err = true;
                        if env.contains_key(name.as_str()) {
                            return Err(TyperError::binder_conflict(name, span).into());
                        }
                        let mut env2 = env.clone();
                        env2.insert(
                            name.as_str(),
                            LocalBinding {
                                ty: *err_ty.clone(),
                                kind: ParamKind::Borrow,
                            },
                        );
                        let mut arm_tracker = baseline.clone();
                        let at = type_of(
                            &arm.expr,
                            &env2,
                            &mut arm_tracker,
                            fns,
                            trait_env,
                            aliases,
                            type_defs,
                            type_params,
                            bounds,
                            depth + 1,
                            expected,
                        )?;
                        branch_trackers.push((arm_tracker, expr_span(&arm.expr)));
                        if let Some(rt) = &res_ty_opt {
                            if &at != rt {
                                let sp = expr_span(&arm.expr);
                                return Err(
                                    TyperError::match_arm_type_mismatch(rt.clone(), at, sp).into(),
                                );
                            }
                        } else {
                            res_ty_opt = Some(at);
                        }
                    }
                    MatchPat::Wildcard => {
                        seen_wildcard = true;
                        let mut arm_tracker = baseline.clone();
                        let at = type_of(
                            &arm.expr,
                            env,
                            &mut arm_tracker,
                            fns,
                            trait_env,
                            aliases,
                            type_defs,
                            type_params,
                            bounds,
                            depth + 1,
                            expected,
                        )?;
                        branch_trackers.push((arm_tracker, expr_span(&arm.expr)));
                        if let Some(rt) = &res_ty_opt {
                            if &at != rt {
                                let sp = expr_span(&arm.expr);
                                return Err(
                                    TyperError::match_arm_type_mismatch(rt.clone(), at, sp).into(),
                                );
                            }
                        } else {
                            res_ty_opt = Some(at);
                        }
                    }
                    _ => {
                        return Err(TyperError::match_invalid_scrutinee(scrut_ty.clone(), span)
                            .into());
                    }
                }
            }
            if !seen_wildcard && !(seen_ok && seen_err) {
                return Err(TyperError::match_non_exhaustive(span).into());
            }
            let result_ty = res_ty_opt.expect("match arms must not be empty");
            merge_match_trackers(&baseline, &branch_trackers, tracker)?;
            Ok(result_ty)
        }
        Type::Named { name, args } if type_defs.enums.contains_key(name.as_str()) => {
            let enum_info = type_defs
                .enums
                .get(name.as_str())
                .expect("enum definition missing");
            if enum_info.decl.type_params.len() != args.len() {
                return Err(TyperError::type_arg_count_mismatch(
                    name.as_str(),
                    enum_info.decl.type_params.len(),
                    args.len(),
                    Some(span),
                )
                .into());
            }
            let mut subst: TypeSubst = HashMap::new();
            for (param, arg) in enum_info.decl.type_params.iter().zip(args.iter()) {
                subst.insert(param.name.clone(), arg.clone());
            }
            let baseline = tracker.clone();
            let mut seen: HashSet<&str> = HashSet::new();
            let mut seen_wildcard = false;
            let mut res_ty_opt: Option<Type> = None;
            let mut branch_trackers: Vec<(ResourceTracker, Span)> = Vec::new();
            for arm in arms {
                if seen_wildcard || seen.len() == enum_info.decl.variants.len() {
                    return Err(TyperError::match_unreachable_arm(span).into());
                }
                match &arm.pat {
                    MatchPat::EnumVariant {
                        enum_name,
                        variant,
                        binders,
                    } => {
                        if enum_name.as_str() != name.as_str() {
                            let label = format!("{}::{}", enum_name, variant);
                            return Err(TyperError::unknown_enum_variant(&label, span).into());
                        }
                        let Some(variant_def) = enum_info.variants.get(variant.as_str()) else {
                            let label = format!("{}::{}", enum_name, variant);
                            return Err(TyperError::unknown_enum_variant(&label, span).into());
                        };
                        if seen.contains(variant.as_str()) {
                            let label = format!("{}::{}", enum_name, variant);
                            return Err(
                                TyperError::match_duplicate_arm(label.as_str(), span).into(),
                            );
                        }
                        if binders.len() != variant_def.fields.len() {
                            let label = format!("{}::{}", enum_name, variant);
                            return Err(TyperError::enum_variant_arity_mismatch(
                                &label,
                                variant_def.fields.len(),
                                binders.len(),
                                span,
                            )
                            .into());
                        }
                        let mut env2 = env.clone();
                        let mut seen_binders: HashSet<&str> = HashSet::new();
                        for (idx, binder) in binders.iter().enumerate() {
                            if !seen_binders.insert(binder.as_str())
                                || env2.contains_key(binder.as_str())
                            {
                                return Err(TyperError::binder_conflict(binder, span).into());
                            }
                            let ty = variant_def
                                .fields
                                .get(idx)
                                .map(|field| substitute_type(field, &subst))
                                .unwrap_or(Type::Int);
                            env2.insert(
                                binder.as_str(),
                                LocalBinding {
                                    ty,
                                    kind: ParamKind::Borrow,
                                },
                            );
                        }
                        seen.insert(variant.as_str());
                        let mut arm_tracker = baseline.clone();
                        let at = type_of(
                            &arm.expr,
                            &env2,
                            &mut arm_tracker,
                            fns,
                            trait_env,
                            aliases,
                            type_defs,
                            type_params,
                            bounds,
                            depth + 1,
                            expected,
                        )?;
                        branch_trackers.push((arm_tracker, expr_span(&arm.expr)));
                        if let Some(rt) = &res_ty_opt {
                            if &at != rt {
                                let sp = expr_span(&arm.expr);
                                return Err(
                                    TyperError::match_arm_type_mismatch(rt.clone(), at, sp).into(),
                                );
                            }
                        } else {
                            res_ty_opt = Some(at);
                        }
                    }
                    MatchPat::Wildcard => {
                        seen_wildcard = true;
                        let mut arm_tracker = baseline.clone();
                        let at = type_of(
                            &arm.expr,
                            env,
                            &mut arm_tracker,
                            fns,
                            trait_env,
                            aliases,
                            type_defs,
                            type_params,
                            bounds,
                            depth + 1,
                            expected,
                        )?;
                        branch_trackers.push((arm_tracker, expr_span(&arm.expr)));
                        if let Some(rt) = &res_ty_opt {
                            if &at != rt {
                                let sp = expr_span(&arm.expr);
                                return Err(
                                    TyperError::match_arm_type_mismatch(rt.clone(), at, sp).into(),
                                );
                            }
                        } else {
                            res_ty_opt = Some(at);
                        }
                    }
                    _ => {
                        let label = match &arm.pat {
                            MatchPat::Some(_) => "Some".to_string(),
                            MatchPat::None => "None".to_string(),
                            MatchPat::Ok(_) => "Ok".to_string(),
                            MatchPat::Err(_) => "Err".to_string(),
                            MatchPat::Wildcard => "_".to_string(),
                            MatchPat::EnumVariant {
                                enum_name,
                                variant,
                                ..
                            } => format!("{}::{}", enum_name, variant),
                        };
                        return Err(TyperError::unknown_enum_variant(&label, span).into());
                    }
                }
            }
            if !seen_wildcard && seen.len() != enum_info.decl.variants.len() {
                return Err(TyperError::match_non_exhaustive(span).into());
            }
            let result_ty = res_ty_opt.expect("match arms must not be empty");
            merge_match_trackers(&baseline, &branch_trackers, tracker)?;
            Ok(result_ty)
        }
        _ => Err(TyperError::match_invalid_scrutinee(scrut_ty.clone(), span).into()),
    }
}
