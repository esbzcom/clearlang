use anyhow::{anyhow, Context, Result};
use clg_codegen_wasm::{
    emit_from_ir_with_opts, CodegenOpts, ExternalImport, StdCoreLinkMode as WasmStdCoreLinkMode,
};
use clg_ir::IrType;
use clg_typer::{check_with_vcs_with_std_and_external, ExternalBuiltinSig, TyperError};
use std::path::{Path, PathBuf};
use wasmtime as wt;
use wasmtime_wasi::sync::WasiCtxBuilder;

use super::{
    helpers::{extract_function_name, make_single_json_error, CommandError},
    modules::load_program,
};
use crate::logging::{Logger, StageTimings};

mod crypto;
mod env;
mod package_loader;
mod runtime_error;
mod wasm_state;

use package_loader::{
    load_runtime_packages_from_local_store_if_present, LoadedRuntimePackageSet, RuntimeLoaderConfig,
};
use runtime_error::{extract_runtime_error, extract_wasmtime_limit_error, RuntimeErrorDiag};

pub fn run(file: PathBuf, invoke: String, json_errors: bool, logger: Logger) -> Result<()> {
    let mut timings = StageTimings::new();
    let engine = wt::Engine::new(&wt::Config::new())?;
    let module = load_module_for_run(&engine, &file, json_errors, logger, &mut timings)?;
    let required_host_capabilities = required_host_capabilities_for_module(&module);
    let runtime_packages = {
        let _stage = timings.start(logger, "runtime_load_packages");
        let runtime_root = file.parent().unwrap_or(Path::new("."));
        let loaded = load_runtime_packages_from_local_store_if_present(
            runtime_root,
            RuntimeLoaderConfig {
                allow_remote_fetch: false,
                required_host_capabilities,
            },
        )
        .map_err(|err| runtime_loader_diag_to_error(Path::new(&file), json_errors, err))?;
        if let Some(loaded_packages) = loaded.as_ref() {
            let _ = loaded_packages.resolver_version;
            for pkg in &loaded_packages.packages {
                let _ = (
                    pkg.id.as_str(),
                    pkg.digest.as_str(),
                    pkg.abi_id.as_str(),
                    pkg.resolved_path.as_path(),
                );
            }
            for binding in &loaded_packages.bindings {
                let _ = (
                    binding.import_module.as_str(),
                    binding.import_name.as_str(),
                    binding.provider_package_id.as_str(),
                    binding.provider_symbol.as_str(),
                );
            }
        }
        loaded
    };
    let wasi = WasiCtxBuilder::new()
        .inherit_stdout()
        .inherit_stderr()
        .build();
    let mut store = wt::Store::new(&engine, wasi);
    let instance = {
        let _stage = timings.start(logger, "instantiate");
        let mut linker = wt::Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |cx| cx).context("linking WASI")?;
        env::add_env_stubs(&mut linker).context("linking env stubs")?;
        crypto::add_crypto_stubs(&mut linker).context("linking crypto stubs")?;
        if let Some(runtime_packages) = runtime_packages.as_ref() {
            link_runtime_packages(&engine, &module, &mut store, &mut linker, runtime_packages)
                .map_err(|err| runtime_loader_diag_to_error(Path::new(&file), json_errors, err))?;
        }
        linker
            .instantiate(&mut store, &module)
            .context("instantiating module")?
    };
    let func = instance
        .get_typed_func::<(), i32>(&mut store, &invoke)
        .with_context(|| format!("export `{}` not found or wrong type", invoke))?;

    match func.call(&mut store, ()) {
        Ok(result) => {
            if let Some(diag) = extract_runtime_error(&instance, &mut store) {
                return Err(runtime_diag_to_error(Path::new(&file), json_errors, diag));
            }
            println!("{}", result);
            logger.summary(&timings);
            Ok(())
        }
        Err(trap) => {
            if let Some(diag) = extract_runtime_error(&instance, &mut store) {
                return Err(runtime_diag_to_error(Path::new(&file), json_errors, diag));
            }
            if let Some(diag) = extract_wasmtime_limit_error(&trap) {
                return Err(runtime_diag_to_error(Path::new(&file), json_errors, diag));
            }
            Err(trap)
        }
    }
}

fn required_host_capabilities_for_module(module: &wt::Module) -> Vec<String> {
    let mut required = std::collections::BTreeSet::new();
    for import in module.imports() {
        if let Some(capability) = runtime_import_capability(import.module(), import.name()) {
            required.insert(capability.to_string());
        }
    }
    required.into_iter().collect()
}

fn runtime_import_capability(module: &str, name: &str) -> Option<&'static str> {
    match (module, name) {
        ("clearlang_crypto", "crypto_hash") => Some("std::crypto::hash"),
        ("clearlang_crypto", "crypto_hmac") => Some("std::crypto::hmac"),
        ("clearlang_crypto", "crypto_verify") => Some("std::crypto::verify"),
        ("clearlang_env", "env_time") => Some("std::env::time"),
        ("clearlang_env", "env_random") => Some("std::env::random"),
        ("clearlang_env", "env_chain_id") => Some("std::env::chain_id"),
        ("wasi_snapshot_preview1", "fd_write") => Some("std::wasi::print"),
        _ => None,
    }
}

fn link_runtime_packages(
    engine: &wt::Engine,
    module: &wt::Module,
    store: &mut wt::Store<wasmtime_wasi::WasiCtx>,
    linker: &mut wt::Linker<wasmtime_wasi::WasiCtx>,
    runtime_packages: &LoadedRuntimePackageSet,
) -> Result<(), package_loader::RuntimePackageLoaderError> {
    let mut provider_module_names: std::collections::HashMap<&str, String> =
        std::collections::HashMap::with_capacity(runtime_packages.packages.len());

    for pkg in &runtime_packages.packages {
        let provider_module = wt::Module::from_file(engine, &pkg.resolved_path).map_err(|err| {
            package_loader::RuntimePackageLoaderError::new(
                "R015",
                format!(
                    "runtime linker failed to load provider package `{}` module `{}`: {err}",
                    pkg.id,
                    pkg.resolved_path.display()
                ),
            )
        })?;
        let instance = linker
            .instantiate(&mut *store, &provider_module)
            .map_err(|err| {
                package_loader::RuntimePackageLoaderError::new(
                    "R015",
                    format!(
                        "runtime linker failed to instantiate provider package `{}`: {err}",
                        pkg.id
                    ),
                )
            })?;
        let provider_module_name = format!("__clg_runtime_pkg__{}", pkg.id);
        linker
            .instance(&mut *store, provider_module_name.as_str(), instance)
            .map_err(|err| {
                package_loader::RuntimePackageLoaderError::new(
                    "R015",
                    format!(
                        "runtime linker failed to register provider package `{}` exports: {err}",
                        pkg.id
                    ),
                )
            })?;
        provider_module_names.insert(pkg.id.as_str(), provider_module_name);
    }

    for binding in &runtime_packages.bindings {
        let Some(provider_module_name) =
            provider_module_names.get(binding.provider_package_id.as_str())
        else {
            return Err(package_loader::RuntimePackageLoaderError::new(
                "R015",
                format!(
                    "runtime linker binding `{}`::`{}` references unknown provider package `{}`",
                    binding.import_module, binding.import_name, binding.provider_package_id
                ),
            ));
        };
        linker
            .alias(
                provider_module_name.as_str(),
                binding.provider_symbol.as_str(),
                binding.import_module.as_str(),
                binding.import_name.as_str(),
            )
            .map_err(|err| {
                package_loader::RuntimePackageLoaderError::new(
                    "R015",
                    format!(
                        "runtime linker failed to bind `{}`::`{}` to `{}`::`{}`: {err}",
                        binding.import_module,
                        binding.import_name,
                        binding.provider_package_id,
                        binding.provider_symbol
                    ),
                )
            })?;
    }

    for import in module.imports() {
        if linker
            .get(&mut *store, import.module(), import.name())
            .is_none()
        {
            return Err(package_loader::RuntimePackageLoaderError::new(
                "R015",
                format!(
                    "runtime linker has no provider for import `{}`::`{}`",
                    import.module(),
                    import.name()
                ),
            ));
        }
    }

    Ok(())
}

fn load_module_for_run(
    engine: &wt::Engine,
    file: &Path,
    json_errors: bool,
    logger: Logger,
    timings: &mut StageTimings,
) -> Result<wt::Module> {
    if !is_clear_source(file) {
        let _stage = timings.start(logger, "load_module");
        return wt::Module::from_file(engine, file)
            .with_context(|| format!("loading {}", file.display()));
    }

    let loaded = {
        let _stage = timings.start(logger, "parse");
        load_program(file, json_errors)?
    };
    let ast = &loaded.program;
    let external_typer_sigs: Vec<ExternalBuiltinSig> = loaded
        .external_imports
        .iter()
        .map(|binding| ExternalBuiltinSig {
            name: binding.function.clone(),
            params: binding.params.clone(),
            ret: binding.ret.clone(),
            effect: binding.effect,
        })
        .collect();
    let external_codegen_imports: Vec<ExternalImport> = loaded
        .external_imports
        .iter()
        .map(|binding| ExternalImport {
            function: binding.function.clone(),
            import_module: binding.import_module.clone(),
            import_name: binding.import_name.clone(),
        })
        .collect();
    let type_output = {
        let _stage = timings.start(logger, "typecheck");
        match check_with_vcs_with_std_and_external(ast, &loaded.std_types, &external_typer_sigs) {
            Ok(result) => result,
            Err(e) => {
                if json_errors {
                    if let Some((typer, function)) = find_typer_error(&e) {
                        let json = make_single_json_error(
                            typer.code,
                            "type",
                            typer.message.clone(),
                            file,
                            typer.start,
                            typer.end,
                            function,
                        );
                        return Err(CommandError::json(json).into());
                    }
                    let json =
                        make_single_json_error("T000", "type", format!("{e:#}"), file, 0, 0, None);
                    return Err(CommandError::json(json).into());
                }
                return Err(e.context("type-check failed"));
            }
        }
    };
    let ir = type_output.ir;
    let fail_build = |code: &'static str, message: &str, function: Option<String>| -> Result<()> {
        if json_errors {
            let json = make_single_json_error(code, "build", message, file, 0, 0, function);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(message.to_string()))
        }
    };
    match ir.funcs.iter().find(|f| f.name == "main") {
        Some(f) => {
            if f.ret != Some(IrType::Int) || !f.params.is_empty() {
                fail_build(
                    "C001",
                    "only `main() -> Int` is supported in this phase",
                    Some("main".to_string()),
                )?;
            }
        }
        None => {
            fail_build("C002", "missing `main` function", None)?;
        }
    }

    let wasm_bytes = {
        let _stage = timings.start(logger, "codegen");
        emit_from_ir_with_opts(
            &ir,
            CodegenOpts {
                debug_names: false,
                proof_section: None,
                export_aliases: Vec::new(),
                external_imports: external_codegen_imports,
                std_core_link_mode: WasmStdCoreLinkMode::Intrinsic,
            },
        )
        .context("codegen (IR+Wasm) failed")?
    };

    {
        let _stage = timings.start(logger, "load_module");
        wt::Module::from_binary(engine, &wasm_bytes).context("loading compiled module")
    }
}

fn is_clear_source(file: &Path) -> bool {
    file.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("clear"))
}

fn find_typer_error(err: &anyhow::Error) -> Option<(&TyperError, Option<String>)> {
    let mut function: Option<String> = None;
    for cause in err.chain() {
        if function.is_none() {
            let msg = cause.to_string();
            if let Some(name) = extract_function_name(&msg) {
                function = Some(name);
            }
        }
        if let Some(typer) = cause.downcast_ref::<TyperError>() {
            return Some((typer, function));
        }
    }
    None
}

fn runtime_diag_to_error(file: &Path, json_errors: bool, diag: RuntimeErrorDiag) -> anyhow::Error {
    if json_errors {
        let json = make_single_json_error(
            diag.code,
            "runtime",
            diag.message,
            file,
            diag.start,
            diag.end,
            diag.detail,
        );
        CommandError::json(json).into()
    } else {
        let span = if diag.start != 0 || diag.end != 0 {
            format!(" at {}..{}", diag.start, diag.end)
        } else {
            String::new()
        };
        anyhow!("{}{} (code {})", diag.message, span, diag.code)
    }
}

fn runtime_loader_diag_to_error(
    file: &Path,
    json_errors: bool,
    diag: package_loader::RuntimePackageLoaderError,
) -> anyhow::Error {
    if json_errors {
        let json = make_single_json_error(diag.code(), "runtime", diag.message(), file, 0, 0, None);
        CommandError::json(json).into()
    } else {
        anyhow!("{} (code {})", diag.message(), diag.code())
    }
}
