mod expr;
mod intrinsics;
mod monomorphize;
mod refinement_predicate;
mod trait_env;
mod type_params;
mod type_resolve;
mod type_validation;

use self::expr::{consume_var_expr, expr_span, max_effect, type_of, ResourceTracker};
pub(crate) use self::expr::{infer_expr_type, show_ty};
use self::intrinsics::collect_used_intrinsics;
use self::monomorphize::monomorphize_program;
use self::refinement_predicate::predicate_is_contradiction;
use self::trait_env::build_trait_env;
use self::type_params::{validate_bounds, validate_type_params};
use self::type_resolve::resolve_aliases;
use self::type_validation::{
    contains_named_resource, ensure_equatable_collection_keys, ensure_no_resource_collections,
    validate_equatable_collections, validate_known_types, validate_no_resource_collections,
    validate_supported_types,
};
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
    Stmt, StructDecl, StructField, TraitBound, TraitDecl, TraitMethod, Type,
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
pub(crate) use self::type_params::{
    substitute_type, type_contains_params, type_param_names, unify_type_params,
};
pub(crate) use self::type_resolve::base_type;
pub(super) use self::type_resolve::{
    base_types_match, binding_compatible, is_resource_type, refinement_loss,
};
pub(super) use self::type_validation::find_resource_collection;

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
