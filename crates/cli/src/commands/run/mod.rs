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
use crate::commands::modules::host_capability_policy::{
    collect_required_host_capabilities, runtime_import_capability,
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

type RuntimeBindingByImport =
    std::collections::HashMap<(String, String), package_loader::LoadedRuntimeBinding>;
type RuntimeBindingsByProvider =
    std::collections::HashMap<String, Vec<package_loader::LoadedRuntimeBinding>>;

pub fn run(file: PathBuf, invoke: String, json_errors: bool, logger: Logger) -> Result<()> {
    let mut timings = StageTimings::new();
    let engine = wt::Engine::new(&wt::Config::new())?;
    let module = load_module_for_run(&engine, &file, json_errors, logger, &mut timings)?;
    let required_host_capabilities = collect_required_host_capabilities(
        module
            .imports()
            .map(|import| (import.module(), import.name())),
    )
    .into_iter()
    .collect();
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

fn link_runtime_packages(
    engine: &wt::Engine,
    module: &wt::Module,
    store: &mut wt::Store<wasmtime_wasi::WasiCtx>,
    linker: &mut wt::Linker<wasmtime_wasi::WasiCtx>,
    runtime_packages: &LoadedRuntimePackageSet,
) -> Result<(), package_loader::RuntimePackageLoaderError> {
    let active_profile_capabilities: Option<std::collections::HashSet<&str>> = runtime_packages
        .active_profile_capabilities
        .as_ref()
        .map(|caps| caps.iter().map(String::as_str).collect());
    let provider_ids: std::collections::HashSet<&str> = runtime_packages
        .packages
        .iter()
        .map(|pkg| pkg.id.as_str())
        .collect();
    let (binding_by_import, bindings_by_provider) =
        index_runtime_bindings(runtime_packages.bindings.as_slice(), &provider_ids)?;
    validate_runtime_import_binding_coverage(module, &binding_by_import)?;

    let mut provider_units: std::collections::HashMap<String, wt::Module> =
        std::collections::HashMap::with_capacity(runtime_packages.packages.len());
    let mut provider_imports: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::with_capacity(runtime_packages.packages.len());
    let mut provider_exports: std::collections::HashMap<String, std::collections::HashSet<String>> =
        std::collections::HashMap::with_capacity(runtime_packages.packages.len());

    for pkg in &runtime_packages.packages {
        let provider_module = wt::Module::from_file(engine, &pkg.resolved_path).map_err(|err| {
            package_loader::RuntimePackageLoaderError::new(
                "R015",
                format!(
                    "runtime linker failed to load provider package `{}` module `{}` (cause={})",
                    pkg.id,
                    pkg.resolved_path.display(),
                    classify_wasmtime_linker_error(&err)
                ),
            )
        })?;
        let exports = provider_module
            .exports()
            .map(|export| export.name().to_string())
            .collect::<std::collections::HashSet<_>>();
        let imports = provider_module
            .imports()
            .map(|import| (import.module().to_string(), import.name().to_string()))
            .collect::<Vec<_>>();
        if let Some(configured_capabilities) = active_profile_capabilities.as_ref() {
            let required_for_provider = collect_required_host_capabilities(
                imports
                    .iter()
                    .map(|(module, name)| (module.as_str(), name.as_str())),
            );
            let missing = required_for_provider
                .iter()
                .filter(|capability| !configured_capabilities.contains(capability.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                return Err(package_loader::RuntimePackageLoaderError::new(
                    "R016",
                    format!(
                        "runtime host profile is missing required capabilities for provider package `{}`: {}",
                        pkg.id,
                        missing.join(", ")
                    ),
                ));
            }
        }
        provider_imports.insert(pkg.id.clone(), imports);
        provider_exports.insert(pkg.id.clone(), exports);
        provider_units.insert(pkg.id.clone(), provider_module);
    }
    validate_provider_symbol_coverage(&bindings_by_provider, &provider_exports)?;
    let provider_order =
        resolve_provider_link_order(&provider_imports, &binding_by_import, &provider_ids)?;

    for provider_id in provider_order {
        let provider_module = provider_units.remove(provider_id.as_str()).ok_or_else(|| {
            package_loader::RuntimePackageLoaderError::new(
                "R015",
                format!(
                    "runtime linker internal error: missing provider package `{provider_id}` during dependency-ordered instantiation"
                ),
            )
        })?;
        let instance = linker
            .instantiate(&mut *store, &provider_module)
            .map_err(|err| {
                package_loader::RuntimePackageLoaderError::new(
                    "R015",
                    format!(
                        "runtime linker failed to instantiate provider package `{}` (cause={})",
                        provider_id,
                        classify_wasmtime_linker_error(&err)
                    ),
                )
            })?;
        let provider_module_name = format!("__clg_runtime_pkg__{}", provider_id);
        linker
            .instance(&mut *store, provider_module_name.as_str(), instance)
            .map_err(|err| {
                package_loader::RuntimePackageLoaderError::new(
                    "R015",
                    format!(
                        "runtime linker failed to register provider package `{}` exports (cause={})",
                        provider_id,
                        classify_wasmtime_linker_error(&err)
                    ),
                )
            })?;
        if let Some(bindings) = bindings_by_provider.get(provider_id.as_str()) {
            for binding in bindings {
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
                                "runtime linker failed to bind `{}`::`{}` to `{}`::`{}` (cause={})",
                                binding.import_module,
                                binding.import_name,
                                binding.provider_package_id,
                                binding.provider_symbol,
                                classify_wasmtime_linker_error(&err)
                            ),
                        )
                    })?;
            }
        }
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

fn validate_runtime_import_binding_coverage(
    module: &wt::Module,
    binding_by_import: &std::collections::HashMap<
        (String, String),
        package_loader::LoadedRuntimeBinding,
    >,
) -> Result<(), package_loader::RuntimePackageLoaderError> {
    let mut missing = std::collections::BTreeSet::new();
    for import in module.imports() {
        if runtime_import_capability(import.module(), import.name()).is_some() {
            continue;
        }
        let key = (import.module().to_string(), import.name().to_string());
        if !binding_by_import.contains_key(&key) {
            missing.insert(format!("`{}`::`{}`", import.module(), import.name()));
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    Err(package_loader::RuntimePackageLoaderError::new(
        "R015",
        format!(
            "runtime linker missing import-map bindings for required imports: {}",
            missing.into_iter().collect::<Vec<_>>().join(", ")
        ),
    ))
}

fn validate_provider_symbol_coverage(
    bindings_by_provider: &std::collections::HashMap<
        String,
        Vec<package_loader::LoadedRuntimeBinding>,
    >,
    provider_exports: &std::collections::HashMap<String, std::collections::HashSet<String>>,
) -> Result<(), package_loader::RuntimePackageLoaderError> {
    let mut missing = std::collections::BTreeSet::new();
    for (provider_id, bindings) in bindings_by_provider {
        let Some(exports) = provider_exports.get(provider_id.as_str()) else {
            return Err(package_loader::RuntimePackageLoaderError::new(
                "R015",
                format!(
                    "runtime linker has bindings for provider package `{provider_id}` but no loaded module exports"
                ),
            ));
        };
        for binding in bindings {
            if !exports.contains(binding.provider_symbol.as_str()) {
                missing.insert(format!(
                    "`{}`::`{}` -> `{}`::`{}`",
                    binding.import_module,
                    binding.import_name,
                    binding.provider_package_id,
                    binding.provider_symbol
                ));
            }
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    Err(package_loader::RuntimePackageLoaderError::new(
        "R015",
        format!(
            "runtime linker bindings reference missing provider exports: {}",
            missing.into_iter().collect::<Vec<_>>().join(", ")
        ),
    ))
}

fn index_runtime_bindings(
    bindings: &[package_loader::LoadedRuntimeBinding],
    provider_ids: &std::collections::HashSet<&str>,
) -> Result<
    (RuntimeBindingByImport, RuntimeBindingsByProvider),
    package_loader::RuntimePackageLoaderError,
> {
    let mut by_import: RuntimeBindingByImport =
        std::collections::HashMap::with_capacity(bindings.len());
    let mut by_provider: RuntimeBindingsByProvider = std::collections::HashMap::new();

    for binding in bindings {
        if !provider_ids.contains(binding.provider_package_id.as_str()) {
            return Err(package_loader::RuntimePackageLoaderError::new(
                "R015",
                format!(
                    "runtime linker binding `{}`::`{}` references unknown provider package `{}`",
                    binding.import_module, binding.import_name, binding.provider_package_id
                ),
            ));
        }
        let key = (binding.import_module.clone(), binding.import_name.clone());
        if let Some(existing) = by_import.get(&key) {
            if existing.provider_package_id != binding.provider_package_id
                || existing.provider_symbol != binding.provider_symbol
            {
                return Err(package_loader::RuntimePackageLoaderError::new(
                    "R015",
                    format!(
                        "runtime linker import `{}`::`{}` has ambiguous provider bindings (`{}`::`{}` and `{}`::`{}`)",
                        binding.import_module,
                        binding.import_name,
                        existing.provider_package_id,
                        existing.provider_symbol,
                        binding.provider_package_id,
                        binding.provider_symbol
                    ),
                ));
            }
            continue;
        }
        by_import.insert(key, binding.clone());
        by_provider
            .entry(binding.provider_package_id.clone())
            .or_default()
            .push(binding.clone());
    }

    Ok((by_import, by_provider))
}

fn resolve_provider_link_order(
    provider_imports: &std::collections::HashMap<String, Vec<(String, String)>>,
    binding_by_import: &std::collections::HashMap<
        (String, String),
        package_loader::LoadedRuntimeBinding,
    >,
    provider_ids: &std::collections::HashSet<&str>,
) -> Result<Vec<String>, package_loader::RuntimePackageLoaderError> {
    let mut dependencies_by_provider: std::collections::HashMap<
        String,
        std::collections::BTreeSet<String>,
    > = std::collections::HashMap::with_capacity(provider_imports.len());
    for (provider_id, imports) in provider_imports {
        let mut dependencies = std::collections::BTreeSet::new();
        for (import_module, import_name) in imports {
            if runtime_import_capability(import_module.as_str(), import_name.as_str()).is_some() {
                continue;
            }
            let key = (import_module.clone(), import_name.clone());
            let Some(binding) = binding_by_import.get(&key) else {
                continue;
            };
            if !provider_ids.contains(binding.provider_package_id.as_str()) {
                return Err(package_loader::RuntimePackageLoaderError::new(
                    "R015",
                    format!(
                        "runtime linker import `{}`::`{}` resolves to unknown provider package `{}`",
                        import_module, import_name, binding.provider_package_id
                    ),
                ));
            }
            if binding.provider_package_id != *provider_id {
                dependencies.insert(binding.provider_package_id.clone());
            }
        }
        dependencies_by_provider.insert(provider_id.clone(), dependencies);
    }

    let mut indegree = std::collections::HashMap::with_capacity(dependencies_by_provider.len());
    let mut dependents: std::collections::HashMap<String, std::collections::BTreeSet<String>> =
        std::collections::HashMap::with_capacity(dependencies_by_provider.len());
    for (provider_id, deps) in &dependencies_by_provider {
        indegree.insert(provider_id.clone(), deps.len());
        for dep in deps {
            dependents
                .entry(dep.clone())
                .or_default()
                .insert(provider_id.clone());
        }
    }

    let mut ready = std::collections::BTreeSet::new();
    for (provider_id, count) in &indegree {
        if *count == 0 {
            ready.insert(provider_id.clone());
        }
    }

    let mut ordered = Vec::with_capacity(dependencies_by_provider.len());
    while let Some(next) = ready.iter().next().cloned() {
        ready.remove(next.as_str());
        ordered.push(next.clone());

        if let Some(children) = dependents.get(next.as_str()) {
            for child in children {
                if let Some(entry) = indegree.get_mut(child) {
                    *entry = entry.saturating_sub(1);
                    if *entry == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }
    }

    if ordered.len() == dependencies_by_provider.len() {
        return Ok(ordered);
    }

    let mut cycle_nodes = indegree
        .into_iter()
        .filter_map(|(provider_id, count)| if count > 0 { Some(provider_id) } else { None })
        .collect::<Vec<_>>();
    cycle_nodes.sort();
    Err(package_loader::RuntimePackageLoaderError::new(
        "R015",
        format!(
            "runtime linker detected a provider dependency cycle: {}",
            cycle_nodes.join(", ")
        ),
    ))
}

fn classify_wasmtime_linker_error(err: &impl std::fmt::Display) -> &'static str {
    let text = err.to_string().to_ascii_lowercase();
    if text.contains("unknown import") || text.contains("unknown module") {
        return "unknown_import";
    }
    if text.contains("incompatible import") || text.contains("type mismatch") {
        return "type_mismatch";
    }
    if text.contains("failed to parse")
        || text.contains("malformed")
        || text.contains("magic header not detected")
    {
        return "invalid_wasm";
    }
    if text.contains("link") {
        return "link_failure";
    }
    "other"
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
