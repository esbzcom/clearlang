mod expr;
mod intrinsics;
mod monomorphize;
mod refinement_predicate;
mod type_resolve;

use self::expr::{consume_var_expr, expr_span, max_effect, type_of, ResourceTracker};
pub(crate) use self::expr::{infer_expr_type, show_ty};
use self::intrinsics::collect_used_intrinsics;
use self::monomorphize::monomorphize_program;
use self::refinement_predicate::predicate_is_contradiction;
use self::type_resolve::resolve_aliases;
use crate::builtins::builtin_sigs;
use crate::errors::TyperError;
use crate::guards::{
    collect_mut_calls, collect_mut_guards, guard_callee_for_kind, MutCall, MutGuardKey,
};
use crate::lower::lower_func;
use crate::vc::{generate_vcs, VerificationCondition};
use anyhow::{Context, Result};
use clg_ast::{
    Block, Effect, EnumDecl, EnumVariant, Expr, Func, ImplDecl, Param, ParamKind, Program, Span,
    Stmt, StructDecl, StructField, TraitBound, TraitDecl, TraitMethod, Type, TypeParam,
};
use clg_ir::Module;
use std::collections::{HashMap, HashSet};

const RETURN_KEY: &str = "$return";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum EffectLevel {
    Pure,
    Mut,
    Io,
}

impl EffectLevel {
    fn join(self, other: EffectLevel) -> EffectLevel {
        if self >= other {
            self
        } else {
            other
        }
    }
}

pub(super) fn effect_label(level: EffectLevel) -> &'static str {
    match level {
        EffectLevel::Pure => "pure",
        EffectLevel::Mut => "mut",
        EffectLevel::Io => "io",
    }
}

#[derive(Clone)]
pub(crate) struct AliasDef {
    base: Type,
    predicate: Expr,
    binder: Option<String>,
    span: Span,
}

pub(crate) type AliasMap = HashMap<String, AliasDef>;

#[derive(Clone, Debug)]
pub struct StdTypeInfo {
    pub byte_len: u32,
    pub align: u32,
}

pub type StdTypeMap = HashMap<String, StdTypeInfo>;

pub(crate) struct StructInfo<'a> {
    pub decl: &'a StructDecl,
    pub fields: HashMap<&'a str, &'a StructField>,
}

pub(crate) struct EnumInfo<'a> {
    pub decl: &'a EnumDecl,
    pub variants: HashMap<&'a str, &'a EnumVariant>,
}

pub(crate) struct TypeDefs<'a> {
    pub resources: HashSet<&'a str>,
    pub structs: HashMap<&'a str, StructInfo<'a>>,
    pub enums: HashMap<&'a str, EnumInfo<'a>>,
}

pub(crate) struct TraitInfo<'a> {
    #[allow(dead_code)]
    pub decl: &'a TraitDecl,
    pub methods: HashMap<&'a str, &'a TraitMethod>,
}

pub(crate) struct ImplInfo<'a> {
    pub decl: &'a ImplDecl,
    #[allow(dead_code)]
    pub methods: HashMap<&'a str, &'a Func>,
}

pub(crate) struct TraitEnv<'a> {
    pub traits: HashMap<&'a str, TraitInfo<'a>>,
    pub impls: Vec<ImplInfo<'a>>,
}

pub(crate) type BoundsMap = HashMap<String, HashSet<String>>;
pub(crate) type TypeSubst = HashMap<String, Type>;
pub(crate) use self::type_resolve::base_type;
pub(super) use self::type_resolve::{
    base_types_match, binding_compatible, is_resource_type, refinement_loss,
};

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

fn validate_type_params(
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

fn validate_bounds(
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
        Type::Option(inner) => {
            Type::Option(Box::new(substitute_type(inner, subst)))
        }
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

fn ensure_no_resource_collections(
    ty: &Type,
    span: Option<Span>,
    resource_names: &HashSet<&str>,
    type_params: &HashSet<String>,
) -> Result<()> {
    if let Some(offending) = find_resource_collection(ty, resource_names, type_params) {
        return Err(TyperError::resource_in_collection(offending, span).into());
    }
    Ok(())
}

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

fn validate_supported_types(
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
        let mut impl_params = validate_type_params(&imp.type_params, type_defs, aliases, trait_env)?;
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

fn find_resource_collection(
    ty: &Type,
    resource_names: &HashSet<&str>,
    type_params: &HashSet<String>,
) -> Option<Type> {
    match ty {
        Type::List(inner) | Type::Set(inner) => {
            if contains_resource(inner, resource_names, type_params) {
                Some(ty.clone())
            } else {
                None
            }
        }
        Type::Map(key, val) => {
            if contains_resource(key, resource_names, type_params)
                || contains_resource(val, resource_names, type_params)
            {
                Some(ty.clone())
            } else {
                None
            }
        }
        Type::Option(inner) | Type::Slice(inner) => {
            find_resource_collection(inner, resource_names, type_params)
        }
        Type::Result(ok, err) => {
            find_resource_collection(ok, resource_names, type_params)
                .or_else(|| find_resource_collection(err, resource_names, type_params))
        }
        Type::Array(inner, _) => find_resource_collection(inner, resource_names, type_params),
        Type::Tuple(elements) => {
            for elem in elements {
                if let Some(found) = find_resource_collection(elem, resource_names, type_params) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn contains_resource(
    ty: &Type,
    resource_names: &HashSet<&str>,
    type_params: &HashSet<String>,
) -> bool {
    match ty {
        Type::Named { name, args } => {
            if type_params.contains(name.as_str()) {
                return false;
            }
            resource_names.contains(name.as_str())
                || args
                    .iter()
                    .any(|arg| contains_resource(arg, resource_names, type_params))
        }
        Type::Option(inner) | Type::List(inner) | Type::Set(inner) | Type::Slice(inner) => {
            contains_resource(inner, resource_names, type_params)
        }
        Type::Result(ok, err) | Type::Map(ok, err) => {
            contains_resource(ok, resource_names, type_params)
                || contains_resource(err, resource_names, type_params)
        }
        Type::Array(inner, _) => contains_resource(inner, resource_names, type_params),
        Type::Tuple(elements) => elements
            .iter()
            .any(|elem| contains_resource(elem, resource_names, type_params)),
        _ => false,
    }
}

fn ensure_equatable_collection_keys(
    ty: &Type,
    span: Option<Span>,
    type_defs: &TypeDefs,
    aliases: &AliasMap,
    type_params: &HashSet<String>,
) -> Result<()> {
    let mut seen = HashSet::new();
    if let Some(offending) =
        find_non_equatable_collection_key(ty, type_defs, aliases, type_params, &mut seen)?
    {
        return Err(TyperError::non_equatable_key(offending, span).into());
    }
    Ok(())
}

fn find_non_equatable_collection_key(
    ty: &Type,
    type_defs: &TypeDefs,
    aliases: &AliasMap,
    type_params: &HashSet<String>,
    seen: &mut HashSet<String>,
) -> Result<Option<Type>> {
    let ty = base_type(ty, aliases)?;
    match ty {
        Type::Set(inner) => {
            if !is_equatable_type(&inner, type_defs, aliases, type_params, seen)? {
                Ok(Some(*inner))
            } else {
                Ok(None)
            }
        }
        Type::Map(key, val) => {
            if !is_equatable_type(&key, type_defs, aliases, type_params, seen)? {
                Ok(Some(*key))
            } else {
                find_non_equatable_collection_key(&val, type_defs, aliases, type_params, seen)
            }
        }
        Type::Option(inner) | Type::List(inner) | Type::Array(inner, _) | Type::Slice(inner) => {
            find_non_equatable_collection_key(&inner, type_defs, aliases, type_params, seen)
        }
        Type::Result(ok, err) => {
            if let Some(offending) =
                find_non_equatable_collection_key(&ok, type_defs, aliases, type_params, seen)?
            {
                Ok(Some(offending))
            } else {
                find_non_equatable_collection_key(&err, type_defs, aliases, type_params, seen)
            }
        }
        Type::Tuple(elements) => {
            for elem in elements {
                if let Some(offending) =
                    find_non_equatable_collection_key(&elem, type_defs, aliases, type_params, seen)?
                {
                    return Ok(Some(offending));
                }
            }
            Ok(None)
        }
        Type::Named { name, args } => {
            if type_params.contains(&name) || name == "Self" {
                return Ok(None);
            }
            if let Some(info) = type_defs.structs.get(name.as_str()) {
                if !seen.insert(format!("struct:{name}")) {
                    return Ok(None);
                }
                let subst = build_type_subst(&info.decl.type_params, &args);
                for field in &info.decl.fields {
                    let field_ty = substitute_type(&field.ty, &subst);
                    if let Some(offending) = find_non_equatable_collection_key(
                        &field_ty, type_defs, aliases, type_params, seen,
                    )? {
                        seen.remove(&format!("struct:{name}"));
                        return Ok(Some(offending));
                    }
                }
                seen.remove(&format!("struct:{name}"));
                Ok(None)
            } else if let Some(info) = type_defs.enums.get(name.as_str()) {
                if !seen.insert(format!("enum:{name}")) {
                    return Ok(None);
                }
                let subst = build_type_subst(&info.decl.type_params, &args);
                for variant in &info.decl.variants {
                    for field_ty in &variant.fields {
                        let field_ty = substitute_type(field_ty, &subst);
                        if let Some(offending) = find_non_equatable_collection_key(
                            &field_ty, type_defs, aliases, type_params, seen,
                        )? {
                            seen.remove(&format!("enum:{name}"));
                            return Ok(Some(offending));
                        }
                    }
                }
                seen.remove(&format!("enum:{name}"));
                Ok(None)
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn is_equatable_type(
    ty: &Type,
    type_defs: &TypeDefs,
    aliases: &AliasMap,
    type_params: &HashSet<String>,
    seen: &mut HashSet<String>,
) -> Result<bool> {
    let ty = base_type(ty, aliases)?;
    Ok(match ty {
        Type::Int
        | Type::Bool
        | Type::U8
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::String
        | Type::Bytes => true,
        Type::Option(inner) => is_equatable_type(&inner, type_defs, aliases, type_params, seen)?,
        Type::Result(ok, err) => {
            is_equatable_type(&ok, type_defs, aliases, type_params, seen)?
                && is_equatable_type(&err, type_defs, aliases, type_params, seen)?
        }
        Type::Array(_, _) | Type::Slice(_) => false,
        Type::Tuple(elements) => {
            let mut ok = true;
            for elem in elements {
                if !is_equatable_type(&elem, type_defs, aliases, type_params, seen)? {
                    ok = false;
                    break;
                }
            }
            ok
        }
        Type::List(_) | Type::Set(_) | Type::Map(_, _) => false,
        Type::Named { name, args } => {
            if type_params.contains(&name) || name == "Self" {
                return Ok(true);
            }
            if type_defs.resources.contains(name.as_str()) {
                return Ok(false);
            }
            if let Some(info) = type_defs.structs.get(name.as_str()) {
                if !seen.insert(format!("struct:{name}")) {
                    return Ok(true);
                }
                let subst = build_type_subst(&info.decl.type_params, &args);
                for field in &info.decl.fields {
                    let field_ty = substitute_type(&field.ty, &subst);
                    if !is_equatable_type(&field_ty, type_defs, aliases, type_params, seen)? {
                        seen.remove(&format!("struct:{name}"));
                        return Ok(false);
                    }
                }
                seen.remove(&format!("struct:{name}"));
                true
            } else if let Some(info) = type_defs.enums.get(name.as_str()) {
                if !seen.insert(format!("enum:{name}")) {
                    return Ok(true);
                }
                let subst = build_type_subst(&info.decl.type_params, &args);
                for variant in &info.decl.variants {
                    for field_ty in &variant.fields {
                        let field_ty = substitute_type(field_ty, &subst);
                        if !is_equatable_type(&field_ty, type_defs, aliases, type_params, seen)? {
                            seen.remove(&format!("enum:{name}"));
                            return Ok(false);
                        }
                    }
                }
                seen.remove(&format!("enum:{name}"));
                true
            } else {
                true
            }
        }
    })
}

fn build_type_subst(params: &[TypeParam], args: &[Type]) -> TypeSubst {
    let mut subst = TypeSubst::with_capacity(params.len());
    for (param, arg) in params.iter().zip(args.iter()) {
        subst.insert(param.name.clone(), arg.clone());
    }
    subst
}

fn contains_named_resource(
    ty: &Type,
    resource_names: &HashSet<&str>,
    type_params: &HashSet<String>,
) -> bool {
    match ty {
        Type::Named { name, args } => {
            if type_params.contains(name.as_str()) {
                return false;
            }
            resource_names.contains(name.as_str())
                || args
                    .iter()
                    .any(|arg| contains_named_resource(arg, resource_names, type_params))
        }
        Type::Option(inner) | Type::List(inner) | Type::Set(inner) | Type::Slice(inner) => {
            contains_named_resource(inner, resource_names, type_params)
        }
        Type::Result(ok, err) | Type::Map(ok, err) => {
            contains_named_resource(ok, resource_names, type_params)
                || contains_named_resource(err, resource_names, type_params)
        }
        Type::Array(inner, _) => contains_named_resource(inner, resource_names, type_params),
        Type::Tuple(elements) => elements
            .iter()
            .any(|elem| contains_named_resource(elem, resource_names, type_params)),
        _ => false,
    }
}

fn validate_no_resource_collections(
    program: &Program,
    resource_names: &HashSet<&str>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    trait_env: &TraitEnv,
) -> Result<()> {
    for res in &program.resources {
        for field in &res.fields {
            ensure_no_resource_collections(
                &field.ty,
                Some(field.span),
                resource_names,
                &HashSet::new(),
            )?;
        }
    }
    for s in &program.structs {
        let type_params = validate_type_params(&s.type_params, type_defs, aliases, trait_env)?;
        for field in &s.fields {
            ensure_no_resource_collections(
                &field.ty,
                Some(field.span),
                resource_names,
                &type_params,
            )?;
        }
    }
    for e in &program.enums {
        let type_params = validate_type_params(&e.type_params, type_defs, aliases, trait_env)?;
        for variant in &e.variants {
            for ty in &variant.fields {
                ensure_no_resource_collections(
                    ty,
                    Some(variant.span),
                    resource_names,
                    &type_params,
                )?;
            }
        }
    }
    for func in &program.funcs {
        let type_params = validate_type_params(&func.type_params, type_defs, aliases, trait_env)?;
        ensure_no_resource_collections(&func.ret, None, resource_names, &type_params)?;
        for param in &func.params {
            ensure_no_resource_collections(&param.ty, None, resource_names, &type_params)?;
        }
    }
    Ok(())
}

fn validate_equatable_collections(
    program: &Program,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    trait_env: &TraitEnv,
) -> Result<()> {
    for res in &program.resources {
        for field in &res.fields {
            ensure_equatable_collection_keys(
                &field.ty,
                Some(field.span),
                type_defs,
                aliases,
                &HashSet::new(),
            )?;
        }
    }
    for alias in &program.refined_aliases {
        ensure_equatable_collection_keys(
            &alias.base,
            Some(alias.span),
            type_defs,
            aliases,
            &HashSet::new(),
        )?;
    }
    for s in &program.structs {
        let type_params = validate_type_params(&s.type_params, type_defs, aliases, trait_env)?;
        for field in &s.fields {
            ensure_equatable_collection_keys(
                &field.ty,
                Some(field.span),
                type_defs,
                aliases,
                &type_params,
            )?;
        }
    }
    for e in &program.enums {
        let type_params = validate_type_params(&e.type_params, type_defs, aliases, trait_env)?;
        for variant in &e.variants {
            for ty in &variant.fields {
                ensure_equatable_collection_keys(
                    ty,
                    Some(variant.span),
                    type_defs,
                    aliases,
                    &type_params,
                )?;
            }
        }
    }
    for func in &program.funcs {
        let type_params = validate_type_params(&func.type_params, type_defs, aliases, trait_env)?;
        ensure_equatable_collection_keys(&func.ret, None, type_defs, aliases, &type_params)?;
        for param in &func.params {
            ensure_equatable_collection_keys(&param.ty, None, type_defs, aliases, &type_params)?;
        }
    }
    Ok(())
}

fn build_type_defs(program: &Program) -> Result<TypeDefs<'_>> {
    let mut resources: HashSet<&str> = HashSet::with_capacity(program.resources.len());
    for res in &program.resources {
        resources.insert(res.name.as_str());
    }

    let alias_names: HashSet<&str> = program
        .refined_aliases
        .iter()
        .map(|alias| alias.name.as_str())
        .collect();

    let mut structs: HashMap<&str, StructInfo<'_>> = HashMap::with_capacity(program.structs.len());
    let mut enums: HashMap<&str, EnumInfo<'_>> = HashMap::with_capacity(program.enums.len());

    for s in &program.structs {
        let name = s.name.as_str();
        if resources.contains(name) {
            return Err(TyperError::type_conflicts_with_resource(name, s.name_span).into());
        }
        if alias_names.contains(name) || structs.contains_key(name) || enums.contains_key(name) {
            return Err(TyperError::duplicate_type(name, s.name_span).into());
        }
        let mut fields: HashMap<&str, &StructField> = HashMap::with_capacity(s.fields.len());
        for field in &s.fields {
            if fields.insert(field.name.as_str(), field).is_some() {
                return Err(
                    TyperError::duplicate_struct_field(field.name.as_str(), field.span).into(),
                );
            }
        }
        structs.insert(
            name,
            StructInfo {
                decl: s,
                fields,
            },
        );
    }

    for e in &program.enums {
        let name = e.name.as_str();
        if resources.contains(name) {
            return Err(TyperError::type_conflicts_with_resource(name, e.name_span).into());
        }
        if alias_names.contains(name) || structs.contains_key(name) || enums.contains_key(name) {
            return Err(TyperError::duplicate_type(name, e.name_span).into());
        }
        let mut variants: HashMap<&str, &EnumVariant> =
            HashMap::with_capacity(e.variants.len());
        for variant in &e.variants {
            if variants.insert(variant.name.as_str(), variant).is_some() {
                return Err(
                    TyperError::duplicate_enum_variant(variant.name.as_str(), variant.span).into(),
                );
            }
        }
        enums.insert(
            name,
            EnumInfo {
                decl: e,
                variants,
            },
        );
    }

    Ok(TypeDefs {
        resources,
        structs,
        enums,
    })
}

fn build_trait_env<'a>(
    program: &'a Program,
    aliases: &AliasMap,
    type_defs: &TypeDefs<'a>,
    std_types: &StdTypeMap,
) -> Result<TraitEnv<'a>> {
    let mut traits: HashMap<&'a str, TraitInfo<'a>> =
        HashMap::with_capacity(program.traits.len());

    for decl in &program.traits {
        if traits.contains_key(decl.name.as_str()) {
            return Err(TyperError::duplicate_trait(&decl.name, decl.name_span).into());
        }
        if !decl.type_params.is_empty() {
            return Err(TyperError::trait_type_params_not_supported(
                decl.name.as_str(),
                decl.span,
            )
            .into());
        }
        let mut methods: HashMap<&str, &TraitMethod> =
            HashMap::with_capacity(decl.methods.len());
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
        traits.insert(
            decl.name.as_str(),
            TraitInfo {
                decl,
                methods,
            },
        );
    }

    let trait_env = TraitEnv {
        traits,
        impls: Vec::new(),
    };

    let mut impls: Vec<ImplInfo<'a>> = Vec::with_capacity(program.impls.len());
    for decl in &program.impls {
        let Some(trait_info) = trait_env.traits.get(decl.trait_name.as_str()) else {
            return Err(
                TyperError::unknown_trait(&decl.trait_name, decl.trait_name_span).into(),
            );
        };
        let type_params =
            validate_type_params(&decl.type_params, type_defs, aliases, &trait_env)?;
        let _ = validate_bounds(&decl.where_bounds, &type_params, &trait_env)?;
        ensure_known_type(
            &decl.for_type,
            aliases,
            type_defs,
            &type_params,
            std_types,
            Some(decl.span),
        )?;

        let mut methods: HashMap<&str, &Func> =
            HashMap::with_capacity(decl.methods.len());
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
                    return Err(TyperError::overlapping_impl(
                        trait_name,
                        left.span,
                        right.span,
                    )
                    .into());
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
                la.iter().zip(ra.iter()).all(|(l, r)| {
                    overlaps(l, left_params, r, right_params, seen)
                })
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

    overlaps(
        left,
        left_params,
        right,
        right_params,
        &mut HashMap::new(),
    )
}

fn ensure_known_type(
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
                    return Err(TyperError::type_arg_count_mismatch(name, 0, args.len(), span).into());
                }
                return Ok(());
            }
            if type_defs.resources.contains(name) {
                if !args.is_empty() {
                    return Err(TyperError::type_arg_count_mismatch(name, 0, args.len(), span).into());
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
            if std_types.contains_key(name) {
                if !args.is_empty() {
                    return Err(TyperError::type_arg_count_mismatch(
                        name,
                        0,
                        args.len(),
                        span,
                    )
                    .into());
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
        _ => Ok(()),
    }
}

fn validate_known_types(
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

fn validate_struct_enum_resources(
    program: &Program,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    trait_env: &TraitEnv,
) -> Result<()> {
    for s in &program.structs {
        let type_params = validate_type_params(&s.type_params, type_defs, aliases, trait_env)?;
        for field in &s.fields {
            let resolved = base_type(&field.ty, aliases)?;
            if contains_named_resource(&resolved, &type_defs.resources, &type_params) {
                return Err(
                    TyperError::resource_field_not_supported("struct", &s.name, field.span).into(),
                );
            }
        }
    }
    for e in &program.enums {
        let type_params = validate_type_params(&e.type_params, type_defs, aliases, trait_env)?;
        for variant in &e.variants {
            for ty in &variant.fields {
                let resolved = base_type(ty, aliases)?;
                if contains_named_resource(&resolved, &type_defs.resources, &type_params) {
                    return Err(TyperError::resource_field_not_supported(
                        "enum",
                        &e.name,
                        variant.span,
                    )
                    .into());
                }
            }
        }
    }
    Ok(())
}

fn enforce_totality(func: &Func) -> Result<()> {
    if matches!(func.effect, Effect::None | Effect::Pure) {
        check_totality_expr(&func.body, &func.name)?;
    }
    Ok(())
}

fn check_totality_expr(expr: &Expr, self_name: &str) -> Result<()> {
    match expr {
        Expr::Block { block } => check_totality_block(block, self_name)?,
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            check_totality_expr(cond, self_name)?;
            check_totality_expr(then_br, self_name)?;
            check_totality_expr(else_br, self_name)?;
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            check_totality_expr(scrutinee, self_name)?;
            for arm in arms {
                check_totality_expr(&arm.expr, self_name)?;
            }
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                check_totality_expr(elem, self_name)?;
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                check_totality_expr(&field.expr, self_name)?;
            }
        }
        Expr::FieldAccess { base, .. } => {
            check_totality_expr(base, self_name)?;
        }
        Expr::Index { base, index, .. } => {
            check_totality_expr(base, self_name)?;
            check_totality_expr(index, self_name)?;
        }
        Expr::Unary { expr, .. } | Expr::Return { expr, .. } | Expr::Try { expr, .. } => {
            check_totality_expr(expr, self_name)?;
        }
        Expr::Bin { lhs, rhs, .. } => {
            check_totality_expr(lhs, self_name)?;
            check_totality_expr(rhs, self_name)?;
        }
        Expr::Call { callee, args, span } => {
            if callee == self_name {
                return Err(TyperError::recursion_requires_measure(callee, *span).into());
            }
            for arg in args {
                check_totality_expr(arg, self_name)?;
            }
        }
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
    }
    Ok(())
}

fn check_totality_block(block: &Block, self_name: &str) -> Result<()> {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                check_totality_expr(expr.as_ref(), self_name)?;
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => {
                if variant.is_none() {
                    return Err(TyperError::while_variant_required(*span).into());
                }
                check_totality_expr(cond.as_ref(), self_name)?;
                check_totality_expr(invariant.as_ref(), self_name)?;
                if let Some(v) = variant {
                    check_totality_expr(v.as_ref(), self_name)?;
                    if let Expr::Int(_, sp) = v.as_ref() {
                        return Err(TyperError::variant_not_decreasing(*sp).into());
                    }
                }
                check_totality_block(body.as_ref(), self_name)?;
            }
        }
    }
    if let Some(tail) = &block.tail {
        check_totality_expr(tail.as_ref(), self_name)?;
    }
    Ok(())
}
fn level_from_effect(effect: Effect) -> EffectLevel {
    match effect {
        Effect::None | Effect::Pure => EffectLevel::Pure,
        Effect::Mut => EffectLevel::Mut,
        Effect::Io => EffectLevel::Io,
    }
}

#[derive(Clone)]
pub(crate) struct FnSig {
    pub params: Vec<Param>,
    pub ret: Type,
    pub effect: EffectLevel,
    pub type_params: Vec<String>,
    pub bounds: Vec<TraitBound>,
}

#[derive(Clone)]
pub(crate) struct LocalBinding {
    pub ty: Type,
    #[allow(dead_code)]
    pub kind: ParamKind,
}

pub struct TypecheckOutput {
    pub ir: Module,
    pub vcs: Vec<VerificationCondition>,
    pub mono_program: Program,
}

pub fn check_with_vcs(ast: &Program) -> Result<TypecheckOutput> {
    let std_types = StdTypeMap::new();
    check_with_vcs_with_std(ast, &std_types)
}

pub fn check_with_vcs_with_std(
    ast: &Program,
    std_types: &StdTypeMap,
) -> Result<TypecheckOutput> {
    if std::env::var("CLG_DISABLE_TOTALITY").is_ok() {
        return fast_path_without_totality_with_std(ast, std_types);
    }
    let type_defs = build_type_defs(ast)?;
    let alias_map = build_alias_map(ast, &type_defs)?;
    let trait_env = build_trait_env(ast, &alias_map, &type_defs, std_types)?;
    validate_no_resource_collections(ast, &type_defs.resources, &alias_map, &type_defs, &trait_env)?;
    validate_equatable_collections(ast, &alias_map, &type_defs, &trait_env)?;
    validate_supported_types(ast, &alias_map, &type_defs, &trait_env)?;
    validate_known_types(ast, &alias_map, &type_defs, &trait_env, std_types)?;
    validate_struct_enum_resources(ast, &alias_map, &type_defs, &trait_env)?;
    let builtins = builtin_sigs();
    let mut fns: HashMap<&str, FnSig> = HashMap::with_capacity(builtins.len() + ast.funcs.len());
    for (name, params, ret, eff) in &builtins {
        fns.insert(
            name.as_str(),
            FnSig {
                params: params.clone(),
                ret: ret.clone(),
                effect: level_from_effect(*eff),
                type_params: Vec::new(),
                bounds: Vec::new(),
            },
        );
    }

    for f in &ast.funcs {
        if alias_map.contains_key(f.name.as_str()) {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
        let type_params = validate_type_params(&f.type_params, &type_defs, &alias_map, &trait_env)?;
        let _ = validate_bounds(&f.where_bounds, &type_params, &trait_env)?;
        if fns
            .insert(
                f.name.as_str(),
                FnSig {
                    params: f.params.clone(),
                    ret: f.ret.clone(),
                    effect: level_from_effect(f.effect),
                    type_params: f.type_params.iter().map(|p| p.name.clone()).collect(),
                    bounds: f.where_bounds.clone(),
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    validate_alias_predicates(&alias_map, &fns, &type_defs, &trait_env)?;

    for f in &ast.funcs {
        check_func(f, &fns, &trait_env, &alias_map, &type_defs)
            .with_context(|| format!("in function `{}`", f.name))?;
    }
    for imp in &trait_env.impls {
        for method in &imp.decl.methods {
            check_impl_method(method, imp, &fns, &trait_env, &alias_map, &type_defs)
                .with_context(|| {
                    format!(
                        "in impl `{}` method `{}`",
                        imp.decl.trait_name, method.name
                    )
                })?;
        }
    }
    for f in &ast.funcs {
        enforce_totality(f)?;
    }

    let mono_program =
        monomorphize_program(ast, &fns, &trait_env, &alias_map, &type_defs)?;
    let mut mono_fns: HashMap<&str, FnSig> =
        HashMap::with_capacity(builtins.len() + mono_program.funcs.len());
    for (name, params, ret, eff) in &builtins {
        mono_fns.insert(
            name.as_str(),
            FnSig {
                params: params.clone(),
                ret: ret.clone(),
                effect: level_from_effect(*eff),
                type_params: Vec::new(),
                bounds: Vec::new(),
            },
        );
    }
    for f in &mono_program.funcs {
        mono_fns.insert(
            f.name.as_str(),
            FnSig {
                params: f.params.clone(),
                ret: f.ret.clone(),
                effect: level_from_effect(f.effect),
                type_params: Vec::new(),
                bounds: Vec::new(),
            },
        );
    }

    // Collect used intrinsics
    let used_intrinsics = collect_used_intrinsics(&mono_program);

    // Order of function indices: all user-defined first, then intrinsics used (stable order)
    let mut fn_indices: HashMap<&str, u32> =
        HashMap::with_capacity(mono_program.funcs.len() + used_intrinsics.len());
    for (i, f) in mono_program.funcs.iter().enumerate() {
        fn_indices.insert(f.name.as_str(), i as u32);
    }
    // Stable intrinsic order
    let intrinsic_order = [
        "std::bytes::len",
        "std::bytes::eq",
        "std::bytes::eq_ct",
        "std::bytes::concat",
        "std::bytes::from_string",
        "std::bytes::to_string",
        "std::wasi::print",
        "std::env::time",
        "std::env::random",
        "std::crypto::hash",
        "std::crypto::hmac",
        "std::crypto::verify",
        "std::str::len",
        "std::str::eq",
        "std::str::concat",
        "std::u64::rotl",
        "std::u64::rotr",
        "std::u64::to_bytes_le",
        "std::u64::to_bytes_be",
        "std::u64::from_bytes_le",
        "std::u64::from_bytes_be",
    ];
    let mut intrinsic_defs: Vec<clg_ir::Function> = Vec::with_capacity(used_intrinsics.len());
    for name in intrinsic_order.iter() {
        if used_intrinsics.contains(*name) {
            let idx = (mono_program.funcs.len() + intrinsic_defs.len()) as u32;
            fn_indices.insert(name, idx);
            // Define IR function signature for the intrinsic
            let (params, ret) = match *name {
                "std::bytes::len" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::bytes::eq" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::bytes::eq_ct" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::bytes::concat" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Int),
                ),
                "std::bytes::from_string" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::bytes::to_string" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::wasi::print" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::env::time" => (vec![], Some(clg_ir::IrType::Int)),
                "std::env::random" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::crypto::hash" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Int),
                ),
                "std::crypto::hmac" => (
                    vec![
                        clg_ir::IrType::Int,
                        clg_ir::IrType::Int,
                        clg_ir::IrType::Int,
                    ],
                    Some(clg_ir::IrType::Int),
                ),
                "std::crypto::verify" => (
                    vec![
                        clg_ir::IrType::Int,
                        clg_ir::IrType::Int,
                        clg_ir::IrType::Int,
                        clg_ir::IrType::Int,
                    ],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::str::len" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::str::eq" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::str::concat" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Int),
                ),
                "std::u64::rotl" | "std::u64::rotr" => (
                    vec![clg_ir::IrType::U64, clg_ir::IrType::U64],
                    Some(clg_ir::IrType::U64),
                ),
                "std::u64::to_bytes_le" | "std::u64::to_bytes_be" => {
                    (vec![clg_ir::IrType::U64], Some(clg_ir::IrType::Int))
                }
                "std::u64::from_bytes_le" | "std::u64::from_bytes_be" => {
                    (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::U64))
                }
                _ => (vec![], None),
            };
            intrinsic_defs.push(clg_ir::Function {
                name: (*name).to_string(),
                params,
                ret,
                body: vec![],
            });
        }
    }

    let mut module = Module {
        funcs: Vec::with_capacity(mono_program.funcs.len() + intrinsic_defs.len()),
    };
    for f in &mono_program.funcs {
        module
            .funcs
            .push(lower_func(
                f,
                &mono_fns,
                &fn_indices,
                &alias_map,
                &trait_env,
                &type_defs,
                std_types,
            )?);
    }
    // Append intrinsic function declarations at the end
    module.funcs.extend(intrinsic_defs);
    let vcs = generate_vcs(&mono_program);
    Ok(TypecheckOutput {
        ir: module,
        vcs,
        mono_program,
    })
}

pub fn check(ast: &Program) -> Result<Module> {
    Ok(check_with_vcs(ast)?.ir)
}

pub fn type_check_only(ast: &Program) -> Result<()> {
    let std_types = StdTypeMap::new();
    type_check_only_with_std(ast, &std_types)
}

pub fn type_check_only_with_std(
    ast: &Program,
    std_types: &StdTypeMap,
) -> Result<()> {
    if std::env::var("CLG_DISABLE_TOTALITY").is_ok() {
        return fast_path_without_totality_with_std(ast, std_types).map(|_| ());
    }
    let type_defs = build_type_defs(ast)?;
    let alias_map = build_alias_map(ast, &type_defs)?;
    let trait_env = build_trait_env(ast, &alias_map, &type_defs, std_types)?;
    validate_no_resource_collections(ast, &type_defs.resources, &alias_map, &type_defs, &trait_env)?;
    validate_equatable_collections(ast, &alias_map, &type_defs, &trait_env)?;
    validate_supported_types(ast, &alias_map, &type_defs, &trait_env)?;
    validate_known_types(ast, &alias_map, &type_defs, &trait_env, std_types)?;
    validate_struct_enum_resources(ast, &alias_map, &type_defs, &trait_env)?;
    let builtins = builtin_sigs();
    let mut fns: HashMap<&str, FnSig> = HashMap::with_capacity(builtins.len() + ast.funcs.len());
    for (name, params, ret, eff) in &builtins {
        fns.insert(
            name.as_str(),
            FnSig {
                params: params.clone(),
                ret: ret.clone(),
                effect: level_from_effect(*eff),
                type_params: Vec::new(),
                bounds: Vec::new(),
            },
        );
    }

    for f in &ast.funcs {
        if alias_map.contains_key(f.name.as_str()) {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
        let type_params = validate_type_params(&f.type_params, &type_defs, &alias_map, &trait_env)?;
        let _ = validate_bounds(&f.where_bounds, &type_params, &trait_env)?;
        if fns
            .insert(
                f.name.as_str(),
                FnSig {
                    params: f.params.clone(),
                    ret: f.ret.clone(),
                    effect: level_from_effect(f.effect),
                    type_params: f.type_params.iter().map(|p| p.name.clone()).collect(),
                    bounds: f.where_bounds.clone(),
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    validate_alias_predicates(&alias_map, &fns, &type_defs, &trait_env)?;

    for f in &ast.funcs {
        check_func(f, &fns, &trait_env, &alias_map, &type_defs)
            .with_context(|| format!("in function `{}`", f.name))?;
    }
    for imp in &trait_env.impls {
        for method in &imp.decl.methods {
            check_impl_method(method, imp, &fns, &trait_env, &alias_map, &type_defs)
                .with_context(|| {
                    format!(
                        "in impl `{}` method `{}`",
                        imp.decl.trait_name, method.name
                    )
                })?;
        }
    }
    for f in &ast.funcs {
        enforce_totality(f)?;
    }
    Ok(())
}

fn fast_path_without_totality_with_std(
    ast: &Program,
    std_types: &StdTypeMap,
) -> Result<TypecheckOutput> {
    let type_defs = build_type_defs(ast)?;
    let alias_map = build_alias_map(ast, &type_defs)?;
    let trait_env = build_trait_env(ast, &alias_map, &type_defs, std_types)?;
    validate_no_resource_collections(ast, &type_defs.resources, &alias_map, &type_defs, &trait_env)?;
    validate_equatable_collections(ast, &alias_map, &type_defs, &trait_env)?;
    validate_supported_types(ast, &alias_map, &type_defs, &trait_env)?;
    validate_known_types(ast, &alias_map, &type_defs, &trait_env, std_types)?;
    validate_struct_enum_resources(ast, &alias_map, &type_defs, &trait_env)?;
    let builtins = builtin_sigs();
    let mut fns: HashMap<&str, FnSig> = HashMap::with_capacity(builtins.len() + ast.funcs.len());
    for (name, params, ret, eff) in &builtins {
        fns.insert(
            name.as_str(),
            FnSig {
                params: params.clone(),
                ret: ret.clone(),
                effect: level_from_effect(*eff),
                type_params: Vec::new(),
                bounds: Vec::new(),
            },
        );
    }

    for f in &ast.funcs {
        if alias_map.contains_key(f.name.as_str()) {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
        let type_params = validate_type_params(&f.type_params, &type_defs, &alias_map, &trait_env)?;
        let _ = validate_bounds(&f.where_bounds, &type_params, &trait_env)?;
        if fns
            .insert(
                f.name.as_str(),
                FnSig {
                    params: f.params.clone(),
                    ret: f.ret.clone(),
                    effect: level_from_effect(f.effect),
                    type_params: f.type_params.iter().map(|p| p.name.clone()).collect(),
                    bounds: f.where_bounds.clone(),
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    validate_alias_predicates(&alias_map, &fns, &type_defs, &trait_env)?;

    for f in &ast.funcs {
        check_func(f, &fns, &trait_env, &alias_map, &type_defs)
            .with_context(|| format!("in function `{}`", f.name))?;
    }
    for imp in &trait_env.impls {
        for method in &imp.decl.methods {
            check_impl_method(method, imp, &fns, &trait_env, &alias_map, &type_defs)
                .with_context(|| {
                    format!(
                        "in impl `{}` method `{}`",
                        imp.decl.trait_name, method.name
                    )
                })?;
        }
    }

    let mono_program =
        monomorphize_program(ast, &fns, &trait_env, &alias_map, &type_defs)?;
    let mut mono_fns: HashMap<&str, FnSig> =
        HashMap::with_capacity(builtins.len() + mono_program.funcs.len());
    for (name, params, ret, eff) in &builtins {
        mono_fns.insert(
            name.as_str(),
            FnSig {
                params: params.clone(),
                ret: ret.clone(),
                effect: level_from_effect(*eff),
                type_params: Vec::new(),
                bounds: Vec::new(),
            },
        );
    }
    for f in &mono_program.funcs {
        mono_fns.insert(
            f.name.as_str(),
            FnSig {
                params: f.params.clone(),
                ret: f.ret.clone(),
                effect: level_from_effect(f.effect),
                type_params: Vec::new(),
                bounds: Vec::new(),
            },
        );
    }

    let used_intrinsics = collect_used_intrinsics(&mono_program);
    let mut fn_indices: HashMap<&str, u32> =
        HashMap::with_capacity(mono_program.funcs.len() + used_intrinsics.len());
    for (i, f) in mono_program.funcs.iter().enumerate() {
        fn_indices.insert(f.name.as_str(), i as u32);
    }
    let intrinsic_order = [
        "std::bytes::len",
        "std::bytes::eq",
        "std::bytes::eq_ct",
        "std::bytes::concat",
        "std::bytes::from_string",
        "std::bytes::to_string",
        "std::wasi::print",
        "std::env::time",
        "std::env::random",
        "std::crypto::hash",
        "std::crypto::hmac",
        "std::crypto::verify",
        "std::str::len",
        "std::str::eq",
        "std::str::concat",
        "std::u64::rotl",
        "std::u64::rotr",
        "std::u64::to_bytes_le",
        "std::u64::to_bytes_be",
        "std::u64::from_bytes_le",
        "std::u64::from_bytes_be",
    ];
    let mut intrinsic_defs: Vec<clg_ir::Function> = Vec::with_capacity(used_intrinsics.len());
    for name in intrinsic_order.iter() {
        if used_intrinsics.contains(*name) {
            let idx = (mono_program.funcs.len() + intrinsic_defs.len()) as u32;
            fn_indices.insert(name, idx);
            let (params, ret) = match *name {
                "std::bytes::len" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::bytes::eq" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::bytes::eq_ct" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::bytes::concat" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Int),
                ),
                "std::bytes::from_string" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::bytes::to_string" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::wasi::print" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::env::time" => (vec![], Some(clg_ir::IrType::Int)),
                "std::env::random" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::crypto::hash" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Int),
                ),
                "std::crypto::hmac" => (
                    vec![
                        clg_ir::IrType::Int,
                        clg_ir::IrType::Int,
                        clg_ir::IrType::Int,
                    ],
                    Some(clg_ir::IrType::Int),
                ),
                "std::crypto::verify" => (
                    vec![
                        clg_ir::IrType::Int,
                        clg_ir::IrType::Int,
                        clg_ir::IrType::Int,
                        clg_ir::IrType::Int,
                    ],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::str::len" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::str::eq" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::str::concat" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Int),
                ),
                "std::u64::rotl" | "std::u64::rotr" => (
                    vec![clg_ir::IrType::U64, clg_ir::IrType::U64],
                    Some(clg_ir::IrType::U64),
                ),
                "std::u64::to_bytes_le" | "std::u64::to_bytes_be" => {
                    (vec![clg_ir::IrType::U64], Some(clg_ir::IrType::Int))
                }
                "std::u64::from_bytes_le" | "std::u64::from_bytes_be" => {
                    (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::U64))
                }
                _ => (vec![], None),
            };
            intrinsic_defs.push(clg_ir::Function {
                name: (*name).to_string(),
                params,
                ret,
                body: vec![],
            });
        }
    }

    let mut module = Module {
        funcs: Vec::with_capacity(mono_program.funcs.len() + intrinsic_defs.len()),
    };
    for f in &mono_program.funcs {
        module
            .funcs
            .push(lower_func(
                f,
                &mono_fns,
                &fn_indices,
                &alias_map,
                &trait_env,
                &type_defs,
                std_types,
            )?);
    }
    module.funcs.extend(intrinsic_defs);
    let vcs = generate_vcs(&mono_program);
    Ok(TypecheckOutput {
        ir: module,
        vcs,
        mono_program,
    })
}

fn check_func<'a>(
    f: &'a Func,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
) -> Result<()> {
    let type_params = type_param_names(&f.type_params);
    let bounds = validate_bounds(&f.where_bounds, &type_params, trait_env)?;
    let mut env: HashMap<&str, LocalBinding> = HashMap::with_capacity(f.params.len() + 1);
    let ret_ty = f.ret.clone();
    env.insert(
        RETURN_KEY,
        LocalBinding {
            ty: ret_ty.clone(),
            kind: ParamKind::Borrow,
        },
    );
    let mut tracker = ResourceTracker::with_capacity(f.params.len());
    for p in &f.params {
        let resolved_ty = base_type(&p.ty, aliases)?;
        if env
            .insert(
                p.name.as_str(),
                LocalBinding {
                    ty: p.ty.clone(),
                    kind: p.kind,
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_parameter(&p.name).into());
        }
        let is_resource = is_resource_type(&resolved_ty, aliases, type_defs)?;
        tracker.register_param(p.name.as_str(), p.kind, is_resource);
    }

    let allowed_effect = level_from_effect(f.effect);

    for req in &f.requires {
        let mut req_tracker = tracker.clone();
        let ty = type_of(
            &req.expr,
            &env,
            &mut req_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            &type_params,
            &bounds,
            0,
            None,
        )?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("require", ty, req.span).into());
        }
        max_effect(&req.expr, fns, trait_env, EffectLevel::Pure)?;
    }

    let guard_keys = collect_mut_guards(&f.requires);

    let mut ensure_env = env.clone();
    if !ensure_env.contains_key("result") {
        ensure_env.insert(
            "result",
            LocalBinding {
                ty: f.ret.clone(),
                kind: ParamKind::Borrow,
            },
        );
    }
    for ens in &f.ensures {
        let mut ensure_tracker = tracker.clone();
        let ty = type_of(
            &ens.expr,
            &ensure_env,
            &mut ensure_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            &type_params,
            &bounds,
            0,
            None,
        )?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("ensure", ty, ens.span).into());
        }
        max_effect(&ens.expr, fns, trait_env, EffectLevel::Pure)?;
    }

    let body_ty = type_of(
        &f.body,
        &env,
        &mut tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        &type_params,
        &bounds,
        0,
        Some(&ret_ty),
    )?;
    if !binding_compatible(&ret_ty, &body_ty, aliases)? {
        let sp = expr_span(&f.body);
        let allow_unsigned_literal = expr::literal_can_coerce_unsigned(&ret_ty, &body_ty, &f.body);
        if !allow_unsigned_literal {
            if let Some(err) = expr::unsigned_literal_range_error(&ret_ty, &body_ty, &f.body) {
                return Err(err.into());
            }
            if base_types_match(&ret_ty, &body_ty, aliases)?
                && refinement_loss(&ret_ty, &body_ty, aliases)
            {
                return Err(TyperError::refinement_loss(ret_ty.clone(), body_ty, sp).into());
            }
            return Err(TyperError::return_type_mismatch(ret_ty.clone(), body_ty, sp).into());
        }
    }
    if is_resource_type(&ret_ty, aliases, type_defs)? {
        consume_var_expr(&mut tracker, &f.body)?;
    }
    tracker.ensure_consumed()?;
    if allowed_effect >= EffectLevel::Mut {
        enforce_mut_guards(&f.body, &guard_keys)?;
    }
    max_effect(&f.body, fns, trait_env, allowed_effect)?;
    Ok(())
}

fn check_impl_method<'a>(
    method: &'a Func,
    imp: &ImplInfo<'a>,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
) -> Result<()> {
    let type_params = type_param_names(&imp.decl.type_params);
    let bounds = validate_bounds(&imp.decl.where_bounds, &type_params, trait_env)?;
    let mut subst: TypeSubst = HashMap::new();
    subst.insert("Self".to_string(), imp.decl.for_type.clone());

    let ret_ty = substitute_type(&method.ret, &subst);
    let mut env: HashMap<&str, LocalBinding> = HashMap::with_capacity(method.params.len() + 1);
    env.insert(
        RETURN_KEY,
        LocalBinding {
            ty: ret_ty.clone(),
            kind: ParamKind::Borrow,
        },
    );
    let mut tracker = ResourceTracker::with_capacity(method.params.len());
    for param in &method.params {
        let param_ty = substitute_type(&param.ty, &subst);
        let resolved_ty = base_type(&param_ty, aliases)?;
        if env
            .insert(
                param.name.as_str(),
                LocalBinding {
                    ty: param_ty.clone(),
                    kind: param.kind,
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_parameter(&param.name).into());
        }
        let is_resource = is_resource_type(&resolved_ty, aliases, type_defs)?;
        tracker.register_param(param.name.as_str(), param.kind, is_resource);
    }

    let allowed_effect = level_from_effect(method.effect);

    for req in &method.requires {
        let mut req_tracker = tracker.clone();
        let ty = type_of(
            &req.expr,
            &env,
            &mut req_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            &type_params,
            &bounds,
            0,
            None,
        )?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("require", ty, req.span).into());
        }
        max_effect(&req.expr, fns, trait_env, EffectLevel::Pure)?;
    }

    let mut ensure_env = env.clone();
    if !ensure_env.contains_key("result") {
        ensure_env.insert(
            "result",
            LocalBinding {
                ty: ret_ty.clone(),
                kind: ParamKind::Borrow,
            },
        );
    }
    for ens in &method.ensures {
        let mut ensure_tracker = tracker.clone();
        let ty = type_of(
            &ens.expr,
            &ensure_env,
            &mut ensure_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            &type_params,
            &bounds,
            0,
            None,
        )?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("ensure", ty, ens.span).into());
        }
        max_effect(&ens.expr, fns, trait_env, EffectLevel::Pure)?;
    }

    let body_ty = type_of(
        &method.body,
        &env,
        &mut tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        &type_params,
        &bounds,
        0,
        Some(&ret_ty),
    )?;
    if !binding_compatible(&ret_ty, &body_ty, aliases)? {
        let sp = expr_span(&method.body);
        let allow_unsigned_literal =
            expr::literal_can_coerce_unsigned(&ret_ty, &body_ty, &method.body);
        if !allow_unsigned_literal {
            if let Some(err) = expr::unsigned_literal_range_error(&ret_ty, &body_ty, &method.body)
            {
                return Err(err.into());
            }
            if base_types_match(&ret_ty, &body_ty, aliases)?
                && refinement_loss(&ret_ty, &body_ty, aliases)
            {
                return Err(TyperError::refinement_loss(ret_ty.clone(), body_ty, sp).into());
            }
            return Err(TyperError::return_type_mismatch(ret_ty.clone(), body_ty, sp).into());
        }
    }
    if is_resource_type(&ret_ty, aliases, type_defs)? {
        consume_var_expr(&mut tracker, &method.body)?;
    }
    tracker.ensure_consumed()?;
    if allowed_effect >= EffectLevel::Mut {
        let guard_keys = collect_mut_guards(&method.requires);
        enforce_mut_guards(&method.body, &guard_keys)?;
    }
    max_effect(&method.body, fns, trait_env, allowed_effect)?;
    Ok(())
}
fn enforce_mut_guards(expr: &Expr, guards: &HashSet<MutGuardKey>) -> Result<()> {
    let mut calls: Vec<MutCall> = Vec::new();
    collect_mut_calls(expr, &mut calls);
    for call in calls {
        let guard_name = guard_callee_for_kind(call.kind);
        let arg_name = match call.target {
            Some(name) => name,
            None => {
                return Err(TyperError::mut_guard_requires_variable(
                    call.callee.as_str(),
                    guard_name,
                    call.span,
                )
                .into());
            }
        };
        let key = MutGuardKey {
            kind: call.kind,
            target: arg_name.clone(),
        };
        if !guards.contains(&key) {
            return Err(TyperError::mut_guard_missing(
                call.callee.as_str(),
                guard_name,
                &arg_name,
                call.span,
            )
            .into());
        }
    }
    Ok(())
}

fn build_alias_map(program: &Program, type_defs: &TypeDefs) -> Result<AliasMap> {
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

fn validate_alias_predicates(
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
