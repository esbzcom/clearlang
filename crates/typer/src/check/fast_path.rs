use anyhow::{Context, Result};
use clg_ast::Program;
use clg_ir::{Instr, Module};
use std::collections::HashMap;

use super::aliases::{build_alias_map, validate_alias_predicates};
use super::function_checks::{check_func, check_impl_method, check_trait_default_method};
use super::intrinsics::collect_used_intrinsics;
use super::monomorphize::monomorphize_program;
use super::totality::level_from_effect;
use super::trait_env::build_trait_env;
use super::type_defs::build_type_defs;
use super::type_params::{validate_bounds, validate_type_params};
use super::type_validation::{
    validate_equatable_collections, validate_known_types, validate_no_resource_collections,
    validate_supported_types,
};
use super::{
    ensure_user_function_name_allowed, validate_struct_enum_resources, FnSig, StdTypeMap,
    TypecheckOutput,
};
use crate::builtins::builtin_sigs;
use crate::errors::TyperError;
use crate::lower::{build_dispatcher_function, dispatcher_name, lower_func};
use crate::vc::generate_vcs;

pub(super) fn fast_path_without_totality_with_std(
    ast: &Program,
    std_types: &StdTypeMap,
) -> Result<TypecheckOutput> {
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
                format!("in interface `{}` default method `{}`", tr.name, method.name)
            })?;
        }
    }
    for imp in &trait_env.impls {
        for method in &imp.decl.methods {
            check_impl_method(method, imp, &fns, &trait_env, &alias_map, &type_defs).with_context(
                || format!("in implementation `{}` method `{}`", imp.decl.trait_name, method.name),
            )?;
        }
    }

    let mono_program = monomorphize_program(ast, &fns, &trait_env, &alias_map, &type_defs)?;
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

    let mut lowered_funcs: Vec<clg_ir::Function> =
        Vec::with_capacity(mono_program.funcs.len() + intrinsic_defs.len());
    let mut generated_lambda_funcs: Vec<clg_ir::Function> = Vec::new();
    let mut lambda_cases = Vec::new();
    let mut dispatcher_patches = Vec::new();
    let mut next_closure_code_id: u32 = 1;
    for f in &mono_program.funcs {
        let lowered = lower_func(
            f,
            &mono_fns,
            &fn_indices,
            &alias_map,
            &trait_env,
            &type_defs,
            std_types,
            &mut next_closure_code_id,
        )?;
        lowered_funcs.push(lowered.function);
        generated_lambda_funcs.extend(lowered.generated_functions);
        lambda_cases.extend(lowered.lambda_cases);
        dispatcher_patches.extend(lowered.dispatcher_patches);
    }
    lowered_funcs.extend(intrinsic_defs);

    let lambda_start_idx = lowered_funcs.len() as u32;
    let mut lambda_fn_indices: HashMap<String, u32> =
        HashMap::with_capacity(generated_lambda_funcs.len());
    for (i, func) in generated_lambda_funcs.iter().enumerate() {
        if lambda_fn_indices
            .insert(func.name.clone(), lambda_start_idx + i as u32)
            .is_some()
        {
            anyhow::bail!("duplicate generated lambda function `{}`", func.name);
        }
    }
    lowered_funcs.extend(generated_lambda_funcs);

    let mut dispatch_groups: HashMap<
        String,
        (
            crate::lower::DispatcherSignature,
            Vec<crate::lower::LambdaDispatchCase>,
        ),
    > = HashMap::new();
    for patch in &dispatcher_patches {
        let key = dispatcher_name(&patch.signature);
        dispatch_groups
            .entry(key)
            .or_insert_with(|| (patch.signature.clone(), Vec::new()));
    }
    for case in lambda_cases {
        let key = dispatcher_name(&case.signature);
        let entry = dispatch_groups
            .entry(key)
            .or_insert_with(|| (case.signature.clone(), Vec::new()));
        entry.1.push(case);
    }

    let mut dispatcher_names = dispatch_groups.keys().cloned().collect::<Vec<_>>();
    dispatcher_names.sort();
    let mut dispatcher_defs: Vec<clg_ir::Function> = Vec::with_capacity(dispatcher_names.len());
    let mut dispatcher_indices: HashMap<String, u32> =
        HashMap::with_capacity(dispatcher_names.len());
    for name in &dispatcher_names {
        let (sig, cases) = dispatch_groups
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("missing dispatcher group `{name}`"))?;
        let idx = (lowered_funcs.len() + dispatcher_defs.len()) as u32;
        dispatcher_indices.insert(name.clone(), idx);
        dispatcher_defs.push(build_dispatcher_function(sig, cases, &lambda_fn_indices)?);
    }

    let mut function_indices_by_name: HashMap<String, usize> =
        HashMap::with_capacity(lowered_funcs.len());
    for (idx, func) in lowered_funcs.iter().enumerate() {
        if function_indices_by_name
            .insert(func.name.clone(), idx)
            .is_some()
        {
            anyhow::bail!("duplicate lowered function `{}`", func.name);
        }
    }
    for patch in dispatcher_patches {
        let dispatcher_key = dispatcher_name(&patch.signature);
        let dispatcher_idx = dispatcher_indices
            .get(&dispatcher_key)
            .copied()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "missing dispatcher function for signature in `{}`",
                    patch.function_name
                )
            })?;
        let func_idx = function_indices_by_name
            .get(patch.function_name.as_str())
            .copied()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "missing function `{}` for dispatcher patch",
                    patch.function_name
                )
            })?;
        let Some(instr) = lowered_funcs
            .get_mut(func_idx)
            .and_then(|f| f.body.get_mut(patch.instr_index))
        else {
            anyhow::bail!(
                "invalid dispatcher patch index {} in function `{}`",
                patch.instr_index,
                patch.function_name
            );
        };
        match instr {
            Instr::Call { callee, .. } => *callee = dispatcher_idx,
            _ => anyhow::bail!(
                "dispatcher patch in `{}` did not target a call instruction",
                patch.function_name
            ),
        }
    }
    lowered_funcs.extend(dispatcher_defs);
    let module = Module {
        funcs: lowered_funcs,
    };
    let vcs = generate_vcs(&mono_program);
    Ok(TypecheckOutput {
        ir: module,
        vcs,
        mono_program,
    })
}
