mod expr;
mod intrinsics;

pub(crate) use self::expr::show_ty;
use self::expr::{expr_span, type_of};
use self::intrinsics::collect_used_intrinsics;
use crate::builtins::builtin_sigs;
use crate::errors::TyperError;
use crate::lower::lower_func;
use crate::vc::{generate_vcs, VerificationCondition};
use anyhow::{Context, Result};
use clg_ast::{Effect, Func, Param, Program, Type};
use clg_ir::Module;
use std::collections::HashMap;

type FnSig<'a> = (&'a [Param], Type);

pub struct TypecheckOutput {
    pub ir: Module,
    pub vcs: Vec<VerificationCondition>,
}

pub fn check_with_vcs(ast: &Program) -> Result<TypecheckOutput> {
    let mut fns: HashMap<&str, FnSig> = HashMap::new();

    let builtins = builtin_sigs();
    for (name, params, ret) in &builtins {
        fns.insert(name.as_str(), (&params[..], ret.clone()));
    }

    for f in &ast.funcs {
        if fns
            .insert(f.name.as_str(), (&f.params, f.ret.clone()))
            .is_some()
        {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    for f in &ast.funcs {
        check_func(f, &fns).with_context(|| format!("in function `{}`", f.name))?;
    }

    // Collect used intrinsics
    let used_intrinsics = collect_used_intrinsics(ast);

    // Order of function indices: all user-defined first, then intrinsics used (stable order)
    let mut fn_indices: HashMap<&str, u32> = HashMap::new();
    for (i, f) in ast.funcs.iter().enumerate() {
        fn_indices.insert(f.name.as_str(), i as u32);
    }
    // Stable intrinsic order
    let intrinsic_order = ["std::str::len", "std::str::eq", "std::str::concat"];
    let mut intrinsic_defs: Vec<clg_ir::Function> = Vec::new();
    for name in intrinsic_order.iter() {
        if used_intrinsics.contains(*name) {
            let idx = (ast.funcs.len() + intrinsic_defs.len()) as u32;
            fn_indices.insert(name, idx);
            // Define IR function signature for the intrinsic
            let (params, ret) = match *name {
                "std::str::len" => (vec![clg_ir::IrType::Int], Some(clg_ir::IrType::Int)),
                "std::str::eq" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Bool),
                ),
                "std::str::concat" => (
                    vec![clg_ir::IrType::Int, clg_ir::IrType::Int],
                    Some(clg_ir::IrType::Int),
                ),
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

    let mut module = Module::default();
    for f in &ast.funcs {
        module.funcs.push(lower_func(f, &fns, &fn_indices)?);
    }
    // Append intrinsic function declarations at the end
    module.funcs.extend(intrinsic_defs);
    let vcs = generate_vcs(ast);
    Ok(TypecheckOutput { ir: module, vcs })
}

pub fn check(ast: &Program) -> Result<Module> {
    Ok(check_with_vcs(ast)?.ir)
}

pub fn type_check_only(ast: &Program) -> Result<()> {
    let mut fns: HashMap<&str, FnSig> = HashMap::new();

    let builtins = builtin_sigs();
    for (name, params, ret) in &builtins {
        fns.insert(name.as_str(), (&params[..], ret.clone()));
    }

    for f in &ast.funcs {
        if fns
            .insert(f.name.as_str(), (&f.params, f.ret.clone()))
            .is_some()
        {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    for f in &ast.funcs {
        check_func(f, &fns).with_context(|| format!("in function `{}`", f.name))?;
    }
    Ok(())
}

fn check_func<'a>(f: &'a Func, fns: &HashMap<&'a str, FnSig<'a>>) -> Result<()> {
    let mut env: HashMap<&str, Type> = HashMap::new();
    for p in &f.params {
        if env.insert(p.name.as_str(), p.ty.clone()).is_some() {
            return Err(TyperError::duplicate_parameter(&p.name).into());
        }
    }

    match f.effect {
        Effect::None | Effect::Pure => {}
        Effect::Mut | Effect::Io => return Err(TyperError::effect_not_supported(f.effect).into()),
    }

    for req in &f.requires {
        let ty = type_of(&req.expr, &env, fns, 0)?;
        if ty != Type::Bool {
            return Err(TyperError::contract_not_bool("require", ty, req.span).into());
        }
    }

    let mut ensure_env = env.clone();
    if !ensure_env.contains_key("result") {
        ensure_env.insert("result", f.ret.clone());
    }
    for ens in &f.ensures {
        let ty = type_of(&ens.expr, &ensure_env, fns, 0)?;
        if ty != Type::Bool {
            return Err(TyperError::contract_not_bool("ensure", ty, ens.span).into());
        }
    }

    let body_ty = type_of(&f.body, &env, fns, 0)?;
    if body_ty != f.ret {
        let sp = expr_span(&f.body);
        return Err(TyperError::return_type_mismatch(f.ret.clone(), body_ty, sp).into());
    }
    Ok(())
}
