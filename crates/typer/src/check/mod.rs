mod aliases;
mod expr;
mod fast_path;
mod function_checks;
mod intrinsics;
mod monomorphize;
mod mut_guards;
mod pipeline;
mod refinement_predicate;
mod totality;
mod trait_env;
mod type_defs;
mod type_params;
mod type_resolve;
mod type_validation;

use self::aliases::{build_alias_map, validate_alias_predicates};
pub(crate) use self::expr::{infer_expr_type, show_ty};
use self::fast_path::fast_path_without_totality_with_std_and_external;
use self::function_checks::{check_func, check_impl_method, check_trait_default_method};
use self::intrinsics::{collect_called_functions, collect_used_intrinsics};
use self::monomorphize::{monomorphize_program, MonomorphizeOutput};
use self::pipeline::check_with_vcs_with_std_and_external_impl;
use self::totality::enforce_totality;
use self::trait_env::build_trait_env;
use self::type_defs::build_type_defs;
use self::type_params::{validate_bounds, validate_type_params};
use self::type_validation::{
    contains_named_resource, validate_equatable_collections, validate_known_types,
    validate_no_resource_collections, validate_supported_types,
};
use crate::builtins::{builtin_route, builtin_sigs, non_abi_builtin_sigs, BuiltinRoute};
use crate::errors::TyperError;
use crate::lower::{build_dispatcher_function, dispatcher_name, lower_func};
use crate::vc::{generate_vcs_with_dependencies, AssumptionDependencies, VerificationCondition};
use anyhow::{Context, Result};
use clg_ast::{
    Effect, EnumDecl, EnumVariant, Expr, Func, ImplDecl, Param, ParamKind, Program, Span,
    StructDecl, StructField, TraitBound, TraitDecl, TraitMethod, Type,
};
use clg_ir::{Instr, Module};
use std::collections::{BTreeMap, HashMap, HashSet};

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
    type_params: Vec<String>,
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
use self::totality::level_from_effect;
pub(crate) use self::type_params::{
    substitute_type, type_contains_params, type_param_names, unify_type_params,
};
pub(crate) use self::type_resolve::base_type;
pub(super) use self::type_resolve::{
    base_types_match, binding_compatible, is_resource_type, refinement_loss,
};
pub(super) use self::type_validation::find_resource_collection;

fn validate_struct_enum_resources(
    program: &Program,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    trait_env: &TraitEnv,
) -> Result<()> {
    for s in &program.structs {
        let type_params = validate_type_params(&s.type_params, type_defs, aliases, trait_env)?;
        if s.is_resource {
            continue;
        }
        for field in &s.fields {
            let resolved = base_type(&field.ty, aliases)?;
            if contains_named_resource(&resolved, &type_defs.resources, &type_params) {
                return Err(TyperError::resource_field_not_supported(
                    "struct", &s.name, field.span,
                )
                .into());
            }
        }
    }
    for e in &program.enums {
        let type_params = validate_type_params(&e.type_params, type_defs, aliases, trait_env)?;
        if e.is_resource {
            continue;
        }
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

#[derive(Clone)]
pub(crate) struct FnSig {
    pub params: Vec<Param>,
    pub ret: Type,
    pub effect: EffectLevel,
    pub type_params: Vec<String>,
    pub bounds: Vec<TraitBound>,
}

#[derive(Clone, Debug)]
pub struct ExternalBuiltinSig {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Type,
    pub effect: Effect,
    pub route: BuiltinRoute,
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
    pub mangled_name_origins: HashMap<String, String>,
}

pub(super) fn ensure_user_function_name_allowed(name: &str) -> Result<()> {
    if name.starts_with("__clg_") {
        return Err(TyperError::reserved_function_namespace(name).into());
    }
    Ok(())
}

pub fn check_with_vcs(ast: &Program) -> Result<TypecheckOutput> {
    let std_types = StdTypeMap::new();
    check_with_vcs_with_std(ast, &std_types)
}

pub fn check_with_vcs_with_std(ast: &Program, std_types: &StdTypeMap) -> Result<TypecheckOutput> {
    check_with_vcs_with_std_and_external(ast, std_types, &[])
}

pub fn check_with_vcs_with_std_and_external(
    ast: &Program,
    std_types: &StdTypeMap,
    external_builtins: &[ExternalBuiltinSig],
) -> Result<TypecheckOutput> {
    let merged = merged_external_builtin_sigs(external_builtins)?;
    check_with_vcs_with_std_and_external_impl(ast, std_types, merged.as_slice())
}

pub fn check(ast: &Program) -> Result<Module> {
    Ok(check_with_vcs(ast)?.ir)
}

pub fn type_check_only(ast: &Program) -> Result<()> {
    let std_types = StdTypeMap::new();
    type_check_only_with_std(ast, &std_types)
}

pub fn type_check_only_with_std(ast: &Program, std_types: &StdTypeMap) -> Result<()> {
    let external_builtins = bundled_non_abi_external_builtin_sigs();
    if std::env::var("CLG_DISABLE_TOTALITY").is_ok() {
        return fast_path_without_totality_with_std_and_external(
            ast,
            std_types,
            external_builtins.as_slice(),
        )
        .map(|_| ());
    }
    let type_defs = build_type_defs(ast)?;
    let alias_map = build_alias_map(ast, &type_defs)?;
    let trait_env = build_trait_env(ast, &alias_map, &type_defs, std_types)?;
    validate_no_resource_collections(
        ast,
        &type_defs.resources,
        &alias_map,
        &type_defs,
        &trait_env,
    )?;
    validate_equatable_collections(ast, &alias_map, &type_defs, &trait_env)?;
    validate_supported_types(ast, &alias_map, &type_defs, &trait_env)?;
    validate_known_types(ast, &alias_map, &type_defs, &trait_env, std_types)?;
    validate_struct_enum_resources(ast, &alias_map, &type_defs, &trait_env)?;
    let builtins = builtin_sigs();
    let mut fns: HashMap<&str, FnSig> =
        HashMap::with_capacity(builtins.len() + external_builtins.len() + ast.funcs.len());
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
    for sig in &external_builtins {
        if fns
            .insert(
                sig.name.as_str(),
                FnSig {
                    params: sig.params.clone(),
                    ret: sig.ret.clone(),
                    effect: level_from_effect(sig.effect),
                    type_params: Vec::new(),
                    bounds: Vec::new(),
                },
            )
            .is_some()
        {
            anyhow::bail!("duplicate external function `{}`", sig.name);
        }
    }

    for f in &ast.funcs {
        ensure_user_function_name_allowed(&f.name)?;
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
    for tr in &ast.traits {
        for method in &tr.methods {
            check_trait_default_method(
                tr.name.as_str(),
                method,
                &fns,
                &trait_env,
                &alias_map,
                &type_defs,
            )
            .with_context(|| {
                format!(
                    "in interface `{}` default method `{}`",
                    tr.name, method.name
                )
            })?;
        }
    }
    for imp in &trait_env.impls {
        for method in &imp.decl.methods {
            check_impl_method(method, imp, &fns, &trait_env, &alias_map, &type_defs).with_context(
                || {
                    format!(
                        "in implementation `{}` method `{}`",
                        imp.decl.trait_name, method.name
                    )
                },
            )?;
        }
    }
    for f in &ast.funcs {
        enforce_totality(f)?;
    }
    Ok(())
}

fn bundled_non_abi_external_builtin_sigs() -> Vec<ExternalBuiltinSig> {
    non_abi_builtin_sigs()
        .into_iter()
        .map(|(name, params, ret, effect)| ExternalBuiltinSig {
            route: builtin_route(name.as_str()),
            name,
            params,
            ret,
            effect,
        })
        .collect()
}

fn merged_external_builtin_sigs(
    external_builtins: &[ExternalBuiltinSig],
) -> Result<Vec<ExternalBuiltinSig>> {
    let mut merged: BTreeMap<String, ExternalBuiltinSig> = BTreeMap::new();
    for sig in bundled_non_abi_external_builtin_sigs()
        .into_iter()
        .chain(external_builtins.iter().cloned())
    {
        if let Some(existing) = merged.get(sig.name.as_str()) {
            if !external_builtin_sigs_match(existing, &sig) {
                anyhow::bail!("conflicting external function `{}`", sig.name);
            }
            continue;
        }
        merged.insert(sig.name.clone(), sig);
    }
    Ok(merged.into_values().collect())
}

fn external_builtin_sigs_match(lhs: &ExternalBuiltinSig, rhs: &ExternalBuiltinSig) -> bool {
    lhs.name == rhs.name
        && lhs.ret == rhs.ret
        && lhs.effect == rhs.effect
        && lhs.route == rhs.route
        && lhs.params.len() == rhs.params.len()
        && lhs
            .params
            .iter()
            .zip(rhs.params.iter())
            .all(|(l, r)| l.kind == r.kind && l.ty == r.ty)
}
