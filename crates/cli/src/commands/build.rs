use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clg_ast::{Effect, Param, ParamKind, Program, Type};
use clg_codegen_wasm::{emit_from_ir_with_opts, CodegenOpts, ExportAlias, ExternalImport};
use clg_ir::IrType;
use clg_typer::{
    check_with_vcs_with_std_and_external, ExternalBuiltinSig, TypecheckOutput, TyperError,
};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::commands::helpers::{
    canonical_json_bytes, extract_function_name, make_single_json_error, sha256_hex, CommandError,
    JsonError, JsonErrorItem,
};
use crate::commands::modules::load_program;
use crate::logging::{LogLevel, Logger, StageTimings};
use crate::proofs::{
    build_assurance_manifest_payload, hash_module, module_bytes_with_zeroed_hash, ProofPackage,
};
use crate::signing::{self, SignScope};

mod strict;
mod strict_host_profile;
mod strict_lockfile;
mod strict_package_contract;
mod strict_package_signatures;
mod strict_preflight_input;
mod strict_trust_policy;
mod vcs_json;

use strict::{
    proof_strict_for_mode, strict_l3_claim_violation, strict_language_profile_violation,
    strict_proof_violation,
};
use strict_host_profile::StrictHostProfileV0;
use strict_lockfile::StrictLockfileV0;
use strict_package_contract::StrictPackageContractV0;
use strict_package_signatures::enforce_trust_gate_v0;
use strict_preflight_input::load_required_strict_preflight_input_v0;
use vcs_json::write_vcs_json;

const LEGACY_PACKAGE_METADATA_FILE: &str = "clg-packages.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum CompilerMode {
    /// Development mode: VC strictness defaults to false unless explicitly enabled.
    Permissive,
    /// Current default behavior: VC strictness defaults to true.
    Standard,
    /// Production mode: requires --emit-vcs and enforces VC strictness.
    Strict,
}

impl CompilerMode {
    fn as_str(self) -> &'static str {
        match self {
            CompilerMode::Permissive => "permissive",
            CompilerMode::Standard => "standard",
            CompilerMode::Strict => "strict",
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    file: PathBuf,
    out: PathBuf,
    contract: bool,
    validate: bool,
    debug_names: bool,
    emit_vcs: Option<PathBuf>,
    compiler_mode: CompilerMode,
    proof_strict: Option<bool>,
    sign: bool,
    key: Option<PathBuf>,
    key_id: Option<String>,
    scope: SignScope,
    sig_out: Option<PathBuf>,
    assurance_manifest_out: Option<PathBuf>,
    lean_checker_version: Option<String>,
    coq_checker_version: Option<String>,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    if sign && emit_vcs.is_none() {
        anyhow::bail!("--sign requires --emit-vcs");
    }

    let fail_preflight = |code: &'static str, message: &str| -> Result<()> {
        if json_errors {
            let json = make_single_json_error(code, "build", message, &file, 0, 0, None);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(message.to_string()))
        }
    };
    let mut strict_host_profile_for_caps: Option<StrictHostProfileV0> = None;
    let mut strict_external_bindings_for_link: Option<StrictExternalBindings> = None;

    if compiler_mode == CompilerMode::Strict && emit_vcs.is_none() {
        fail_preflight(
            "C029",
            "`--compiler-mode strict` requires `--emit-vcs <FILE>`",
        )?;
    }
    if compiler_mode == CompilerMode::Strict {
        let module_root = file.parent().unwrap_or_else(|| Path::new("."));
        let legacy_source = module_root.join(LEGACY_PACKAGE_METADATA_FILE);
        if legacy_source.exists() {
            fail_preflight(
                "C101",
                &format!(
                    "strict mode source-of-truth violation: `{}` is not allowed; use strict preflight inputs (`clg.lock.json`, `clg.package-metadata.json`, `clg.package-abi.json`) and trusted local store artifacts only",
                    legacy_source.display()
                ),
            )?;
        }
        let strict_preflight = match load_required_strict_preflight_input_v0(module_root) {
            Ok(value) => value,
            Err(err) => return fail_preflight(err.code(), err.message()),
        };
        let strict_lockfile = strict_preflight.lockfile;
        let strict_trust_policy = strict_preflight.trust_policy;
        let strict_package_contract = strict_preflight.package_contract;
        let strict_host_profile = strict_preflight.host_profile;
        if let Some(message) =
            strict_artifact_identity_violation(&strict_lockfile, &strict_package_contract)
        {
            fail_preflight("C102", &message)?;
        }
        if let Err(err) =
            enforce_trust_gate_v0(module_root, &strict_package_contract, &strict_trust_policy)
        {
            fail_preflight(err.code(), err.message())?;
        }
        let strict_external_bindings =
            match strict_external_bindings_from_contract(&strict_package_contract) {
                Ok(value) => value,
                Err(message) => return fail_preflight("C105", &message),
            };
        strict_host_profile_for_caps = Some(strict_host_profile);
        strict_external_bindings_for_link = Some(strict_external_bindings);
    }

    let mut timings = StageTimings::new();
    let loaded = {
        let _stage = timings.start(logger, "parse");
        load_program(&file, json_errors)?
    };
    let ast = &loaded.program;
    let (external_typer_sigs, external_codegen_imports) = if compiler_mode == CompilerMode::Strict {
        if let Some(bindings) = strict_external_bindings_for_link.as_ref() {
            (
                bindings.external_typer_sigs.clone(),
                bindings.external_codegen_imports.clone(),
            )
        } else {
            (Vec::new(), Vec::new())
        }
    } else {
        let typer_sigs = loaded
            .external_imports
            .iter()
            .map(|binding| ExternalBuiltinSig {
                name: binding.function.clone(),
                params: binding.params.clone(),
                ret: binding.ret.clone(),
                effect: binding.effect,
            })
            .collect();
        let codegen_imports = loaded
            .external_imports
            .iter()
            .map(|binding| ExternalImport {
                function: binding.function.clone(),
                import_module: binding.import_module.clone(),
                import_name: binding.import_name.clone(),
            })
            .collect();
        (typer_sigs, codegen_imports)
    };

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
                            &file,
                            typer.start,
                            typer.end,
                            function,
                        );
                        return Err(CommandError::json(json).into());
                    }
                    let json =
                        make_single_json_error("T000", "type", format!("{e:#}"), &file, 0, 0, None);
                    return Err(CommandError::json(json).into());
                } else {
                    return Err(e.context("type-check failed"));
                }
            }
        }
    };

    let TypecheckOutput {
        ir,
        vcs,
        mono_program,
        mangled_name_origins,
    } = type_output;

    let fail_build = |code: &'static str, message: &str, function: Option<String>| -> Result<()> {
        if json_errors {
            let json = make_single_json_error(code, "build", message, &file, 0, 0, function);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(message.to_string()))
        }
    };
    let fail_build_violations = |violations: &[StrictGateViolation]| -> Result<()> {
        if violations.is_empty() {
            return Ok(());
        }
        if json_errors {
            let errors = violations
                .iter()
                .map(|violation| JsonErrorItem {
                    code: violation.code,
                    stage: "build",
                    message: violation.message.clone(),
                    file: file.display().to_string(),
                    start: 0,
                    end: 0,
                    function: None,
                })
                .collect();
            let json = JsonError { ok: false, errors };
            Err(CommandError::json(json).into())
        } else {
            let mut message = String::from("strict mode gate violations:");
            for violation in violations {
                message.push_str("\n - [");
                message.push_str(violation.code);
                message.push_str("] ");
                message.push_str(violation.message.as_str());
            }
            Err(anyhow!(message))
        }
    };
    if compiler_mode == CompilerMode::Strict {
        if let (Some(host_profile), Some(bindings)) = (
            strict_host_profile_for_caps.as_ref(),
            strict_external_bindings_for_link.as_ref(),
        ) {
            let baseline_outcome = strict_gate_outcome_from_linked_import_resolution(
                &bindings.expected_profiles,
                host_profile,
                linked_external_import_profiles_from_ir(&ir, external_typer_sigs.as_slice()),
            );
            let mut replay_external_typer_sigs = external_typer_sigs.clone();
            replay_external_typer_sigs.reverse();
            let mut replay_outcome = strict_gate_outcome_from_linked_import_resolution(
                &bindings.expected_profiles,
                host_profile,
                linked_external_import_profiles_from_ir(&ir, replay_external_typer_sigs.as_slice()),
            );
            // Test-only hook to force deterministic replay mismatch coverage in CLI IT.
            if std::env::var_os("CLG_TEST_FORCE_STRICT_DETERMINISM_MISMATCH").is_some() {
                replay_outcome.violations.push(StrictGateViolation {
                    code: "C105",
                    package: "_".to_string(),
                    symbol: "_".to_string(),
                    message: "strict determinism test hook: forced replay mismatch".to_string(),
                });
                sort_strict_gate_violations(replay_outcome.violations.as_mut_slice());
            }
            let mut violations = baseline_outcome.violations.clone();
            let strict_import_map_artifact = match strict_import_map_artifact_with_determinism_check(
                &bindings.expected_profiles,
                host_profile,
                &baseline_outcome,
                &replay_outcome,
            ) {
                Ok(artifact) => Some(artifact),
                Err(message) => {
                    violations.push(StrictGateViolation {
                        code: "C107",
                        package: "_".to_string(),
                        symbol: "_".to_string(),
                        message,
                    });
                    None
                }
            };
            if let Some(artifact) = strict_import_map_artifact.as_ref() {
                match write_strict_import_map_artifact(&out, artifact) {
                    Ok(path) => {
                        if logger.enabled(LogLevel::Debug) {
                            logger.event(
                                LogLevel::Debug,
                                "strict_import_map",
                                "strict_preflight",
                                &[
                                    ("path", path.display().to_string()),
                                    ("sha256", artifact.canonical_hash.clone()),
                                ],
                            );
                        }
                    }
                    Err(err) => violations.push(StrictGateViolation {
                        code: "C108",
                        package: "_".to_string(),
                        symbol: "_".to_string(),
                        message: format!("strict import-map artifact emission failed: {err:#}"),
                    }),
                }
            }
            sort_strict_gate_violations(violations.as_mut_slice());
            fail_build_violations(violations.as_slice())?;
        }
    }

    if (lean_checker_version.is_some() || coq_checker_version.is_some()) && !sign {
        fail_build(
            "C032",
            "`--lean-checker-version`/`--coq-checker-version` require `--sign`",
            None,
        )?;
    }
    if assurance_manifest_out.is_some() && !sign {
        fail_build("C034", "`--assurance-manifest-out` requires `--sign`", None)?;
    }
    if lean_checker_version.is_some() ^ coq_checker_version.is_some() {
        fail_build(
            "C032",
            "`--lean-checker-version` and `--coq-checker-version` must be provided together",
            None,
        )?;
    }
    if let (Some(lean_checker_version), Some(coq_checker_version)) =
        (lean_checker_version.as_ref(), coq_checker_version.as_ref())
    {
        if lean_checker_version.trim().is_empty() || coq_checker_version.trim().is_empty() {
            fail_build(
                "C032",
                "`--lean-checker-version` and `--coq-checker-version` must be non-empty",
                None,
            )?;
        }
    }

    let proof_strict_enabled = match proof_strict_for_mode(compiler_mode, proof_strict) {
        Ok(value) => value,
        Err(message) => {
            fail_build("C030", message, None)?;
            false
        }
    };

    if proof_strict_enabled && emit_vcs.is_some() {
        if let Some(message) = strict_proof_violation(&vcs) {
            fail_build("C014", &message, None)?;
        }
    }
    if compiler_mode == CompilerMode::Strict && emit_vcs.is_some() {
        if let Some(message) = strict_l3_claim_violation(&vcs) {
            fail_build("C031", &message, None)?;
        }
        if let Some(message) = strict_language_profile_violation(&vcs) {
            fail_build("C033", &message, None)?;
        }
    }

    let export_aliases = if contract {
        contract_exports(ast, &file, json_errors)?
    } else {
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
        Vec::new()
    };

    let toolchain = format!("clg-cli/{}", env!("CARGO_PKG_VERSION"));
    let proof_package = emit_vcs.as_ref().map(|_| {
        ProofPackage::from_program(
            &mono_program,
            &vcs,
            &mangled_name_origins,
            toolchain.clone(),
        )
    });

    let zero_section = proof_package
        .as_ref()
        .map(|pkg| pkg.encode_section(&[0u8; 32]))
        .transpose()?;

    let (wasm_bytes, module_hash_bytes) = {
        let _stage = timings.start(logger, "codegen");
        let mut wasm_bytes = emit_from_ir_with_opts(
            &ir,
            CodegenOpts {
                debug_names,
                proof_section: zero_section.clone(),
                export_aliases: export_aliases.clone(),
                external_imports: external_codegen_imports.clone(),
            },
        )
        .context("codegen (IR+Wasm) failed")?;

        let mut module_hash_bytes: Option<[u8; 32]> = None;
        if let Some(pkg) = &proof_package {
            let hash_bytes = hash_module(&wasm_bytes);
            let proof_section = pkg.encode_section(&hash_bytes)?;
            wasm_bytes = emit_from_ir_with_opts(
                &ir,
                CodegenOpts {
                    debug_names,
                    proof_section: Some(proof_section),
                    export_aliases: export_aliases.clone(),
                    external_imports: external_codegen_imports.clone(),
                },
            )
            .context("codegen (IR+Wasm) failed")?;
            let zeroed = module_bytes_with_zeroed_hash(&wasm_bytes)?;
            module_hash_bytes = Some(hash_module(&zeroed));
        }
        (wasm_bytes, module_hash_bytes)
    };

    {
        let _stage = timings.start(logger, "write_wasm");
        if let Some(parent) = out.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
        }
        fs::write(&out, &wasm_bytes).with_context(|| format!("writing {}", out.display()))?;
    }

    if let Some(vcs_path) = emit_vcs {
        let _stage = timings.start(logger, "emit_vcs");
        write_vcs_json(&vcs, &mangled_name_origins, &vcs_path, &file)?;
    }

    if sign {
        let _stage = timings.start(logger, "sign");
        let pkg = proof_package
            .as_ref()
            .ok_or_else(|| anyhow!("proof data unavailable for signing"))?;
        let hash_bytes =
            module_hash_bytes.ok_or_else(|| anyhow!("module hash unavailable for signing"))?;
        let key_path = key.expect("clap ensures key when sign");
        let key_id = key_id.expect("clap ensures key_id when sign");
        let sig_path = sig_out.expect("clap ensures sig_out when sign");
        let manifest_path =
            assurance_manifest_out.unwrap_or_else(|| default_assurance_manifest_path(&sig_path));
        let module_hash_hex = hex::encode(hash_bytes);
        let proofs_hash_hex = pkg.proofs_hash_hex();
        let timestamp = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
        signing::sign_bundle(
            pkg,
            &module_hash_hex,
            scope,
            &key_path,
            &key_id,
            &sig_path,
            &timestamp,
            lean_checker_version.as_deref(),
            coq_checker_version.as_deref(),
        )?;
        let manifest_payload = build_assurance_manifest_payload(
            &vcs,
            &toolchain,
            compiler_mode.as_str(),
            proof_strict_enabled,
            &module_hash_hex,
            &proofs_hash_hex,
            &timestamp,
        );
        signing::sign_assurance_manifest(manifest_payload, &key_path, &key_id, &manifest_path)?;
        if logger.enabled(LogLevel::Debug) {
            logger.event(
                LogLevel::Debug,
                "sign_detail",
                "sign",
                &[
                    ("module_hash", module_hash_hex.clone()),
                    ("proofs_hash", proofs_hash_hex),
                    ("sig_path", sig_path.display().to_string()),
                    ("assurance_manifest", manifest_path.display().to_string()),
                ],
            );
        }
    }

    if validate {
        let _stage = timings.start(logger, "validate_wasm");
        let status = std::process::Command::new("wasm-tools")
            .arg("validate")
            .arg(&out)
            .status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => anyhow::bail!("wasm-tools validate failed with status {:?}", s.code()),
            Err(e) => anyhow::bail!("failed to run wasm-tools: {}", e),
        }
    }

    logger.summary(&timings);
    Ok(())
}

fn contract_exports(ast: &Program, file: &Path, json_errors: bool) -> Result<Vec<ExportAlias>> {
    let fail = |code: &'static str, message: &str, function: Option<String>| -> Result<()> {
        if json_errors {
            let json = make_single_json_error(code, "build", message, file, 0, 0, function);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(message.to_string()))
        }
    };

    if ast
        .funcs
        .iter()
        .any(|f| f.name == "init" || f.name == "handle")
    {
        fail(
            "C013",
            "contract build reserves `init` and `handle`; define `apply` instead",
            None,
        )?;
    }

    let apply = ast.funcs.iter().find(|f| f.name == "apply");
    let query = ast.funcs.iter().find(|f| f.name == "query");
    let Some(apply_fn) = apply else {
        fail("C010", "missing `apply` function for contract build", None)?;
        return Ok(Vec::new());
    };
    let Some(query_fn) = query else {
        fail("C011", "missing `query` function for contract build", None)?;
        return Ok(Vec::new());
    };

    let expect = "pure function apply(state: Bytes, msg: Bytes) -> Bytes";
    if !matches!(apply_fn.effect, Effect::None | Effect::Pure)
        || apply_fn.params.len() != 2
        || apply_fn.params[0].ty != Type::Bytes
        || apply_fn.params[1].ty != Type::Bytes
        || apply_fn.ret != Type::Bytes
    {
        fail(
            "C012",
            &format!("`apply` must have signature `{}`", expect),
            Some("apply".to_string()),
        )?;
    }

    let expect = "pure function query(state: Bytes, msg: Bytes) -> Bytes";
    if !matches!(query_fn.effect, Effect::None | Effect::Pure)
        || query_fn.params.len() != 2
        || query_fn.params[0].ty != Type::Bytes
        || query_fn.params[1].ty != Type::Bytes
        || query_fn.ret != Type::Bytes
    {
        fail(
            "C012",
            &format!("`query` must have signature `{}`", expect),
            Some("query".to_string()),
        )?;
    }

    Ok(vec![
        ExportAlias {
            export: "init".to_string(),
            target: "apply".to_string(),
        },
        ExportAlias {
            export: "handle".to_string(),
            target: "apply".to_string(),
        },
        ExportAlias {
            export: "query".to_string(),
            target: "query".to_string(),
        },
    ])
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct AbiLinkProfile {
    effect: String,
    params: Vec<String>,
    ret: String,
    capability: Option<String>,
}

#[derive(Clone, Debug)]
struct StrictExternalBindings {
    expected_profiles: std::collections::BTreeMap<String, AbiLinkProfile>,
    external_typer_sigs: Vec<ExternalBuiltinSig>,
    external_codegen_imports: Vec<ExternalImport>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct StrictGateViolation {
    code: &'static str,
    package: String,
    symbol: String,
    message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StrictImportMapArtifact {
    canonical_bytes: Vec<u8>,
    canonical_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StrictGateOutcome {
    linked_imports: Vec<(String, AbiLinkProfile)>,
    violations: Vec<StrictGateViolation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StrictResolvedImportBinding {
    profile: AbiLinkProfile,
    params_typed: Vec<Type>,
    ret_typed: Type,
}

fn strict_external_bindings_from_contract(
    package_contract: &StrictPackageContractV0,
) -> Result<StrictExternalBindings, String> {
    use std::collections::BTreeMap;

    let mut resolved: BTreeMap<String, (String, StrictResolvedImportBinding)> = BTreeMap::new();
    for contract in &package_contract.contracts {
        for import in &contract.imports {
            let profile = AbiLinkProfile {
                effect: import.effect.clone(),
                params: import
                    .params
                    .iter()
                    .map(|ty| normalize_type_contract(ty))
                    .collect(),
                ret: normalize_type_contract(&import.ret),
                capability: import.capability.clone(),
            };
            let params_typed: Vec<Type> = profile
                .params
                .iter()
                .map(|ty| parse_contract_type(ty))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|msg| {
                    format!(
                        "strict ABI/link mismatch for symbol `{}`: invalid parameter type in strict package ABI: {}",
                        import.symbol, msg
                    )
                })?;
            let ret_typed = parse_contract_type(profile.ret.as_str()).map_err(|msg| {
                format!(
                    "strict ABI/link mismatch for symbol `{}`: invalid return type in strict package ABI: {}",
                    import.symbol, msg
                )
            })?;
            let binding = StrictResolvedImportBinding {
                profile,
                params_typed,
                ret_typed,
            };
            if let Some((existing_abi_id, existing_binding)) = resolved.get(&import.symbol) {
                if existing_binding.profile != binding.profile {
                    return Err(format!(
                        "strict ABI/link mismatch for symbol `{}`: conflicting ABI profiles between `{}` and `{}`",
                        import.symbol, existing_abi_id, contract.abi_id
                    ));
                }
                continue;
            }
            resolved.insert(import.symbol.clone(), (contract.abi_id.clone(), binding));
        }
    }

    let mut expected_profiles = BTreeMap::new();
    let mut external_typer_sigs = Vec::with_capacity(resolved.len());
    let mut external_codegen_imports = Vec::with_capacity(resolved.len());
    for (symbol, (_, binding)) in resolved {
        let effect = parse_contract_effect(binding.profile.effect.as_str())
            .map_err(|msg| format!("strict ABI/link mismatch for symbol `{}`: {}", symbol, msg))?;
        let params: Vec<Param> = binding
            .params_typed
            .iter()
            .enumerate()
            .map(|(idx, ty)| Param {
                kind: ParamKind::Borrow,
                name: format!("p{idx}"),
                ty: ty.clone(),
            })
            .collect();
        let (import_module, import_name) = split_symbol_import_target(symbol.as_str())?;

        expected_profiles.insert(symbol.clone(), binding.profile);
        external_typer_sigs.push(ExternalBuiltinSig {
            name: symbol.clone(),
            params,
            ret: binding.ret_typed,
            effect,
        });
        external_codegen_imports.push(ExternalImport {
            function: symbol,
            import_module,
            import_name,
        });
    }

    Ok(StrictExternalBindings {
        expected_profiles,
        external_typer_sigs,
        external_codegen_imports,
    })
}

fn split_symbol_import_target(symbol: &str) -> Result<(String, String), String> {
    if let Some((module, name)) = symbol.rsplit_once("::") {
        if module.trim().is_empty() || name.trim().is_empty() {
            return Err(format!(
                "strict ABI/link mismatch for symbol `{}`: symbol must use non-empty `module::name` segments",
                symbol
            ));
        }
        Ok((module.to_string(), name.to_string()))
    } else {
        Err(format!(
            "strict ABI/link mismatch for symbol `{}`: symbol must contain `::` and end with function name",
            symbol
        ))
    }
}

fn parse_contract_effect(raw: &str) -> Result<Effect, String> {
    match raw {
        "pure" => Ok(Effect::Pure),
        "mut" => Ok(Effect::Mut),
        "io" => Ok(Effect::Io),
        "none" => Ok(Effect::None),
        other => Err(format!(
            "unsupported effect `{other}` in strict package ABI"
        )),
    }
}

fn parse_contract_type(raw: &str) -> Result<Type, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err("type is empty".to_string());
    }
    if value.contains('<')
        || value.contains('>')
        || value.contains('[')
        || value.contains(']')
        || value.contains('(')
        || value.contains(')')
        || value.contains(',')
        || value.contains(';')
    {
        return Err(format!(
            "type `{}` uses unsupported generic/compound syntax in strict package ABI v0",
            value
        ));
    }
    let ty = match value {
        "Int" => Type::Int,
        "Bool" => Type::Bool,
        "String" => Type::String,
        "Bytes" => Type::Bytes,
        "U8" => Type::U8,
        "U64" => Type::U64,
        "U128" => Type::U128,
        "U256" => Type::U256,
        other => {
            validate_named_type_path(other)?;
            Type::Named {
                name: other.to_string(),
                args: Vec::new(),
            }
        }
    };
    Ok(ty)
}

fn validate_named_type_path(value: &str) -> Result<(), String> {
    for segment in value.split("::") {
        if segment.is_empty() {
            return Err("contains empty `::` segment".to_string());
        }
        let mut chars = segment.chars();
        let first = chars.next().expect("segment non-empty");
        if !(first == '_' || first.is_ascii_alphabetic()) {
            return Err(format!(
                "segment `{segment}` must start with ASCII letter or `_`"
            ));
        }
        for ch in chars {
            if !(ch == '_' || ch.is_ascii_alphanumeric()) {
                return Err(format!(
                    "segment `{segment}` contains invalid character `{ch}`"
                ));
            }
        }
    }
    Ok(())
}

fn linked_external_import_profiles_from_ir(
    ir: &clg_ir::Module,
    external_imports: &[ExternalBuiltinSig],
) -> Result<Vec<(String, AbiLinkProfile)>, String> {
    use std::collections::BTreeSet;

    let resolved_profiles = resolved_external_import_profiles(external_imports)?;

    let mut linked_symbol_names = BTreeSet::new();
    for func in &ir.funcs {
        if resolved_profiles.contains_key(func.name.as_str()) {
            linked_symbol_names.insert(func.name.as_str());
        }
    }

    let mut linked = Vec::with_capacity(linked_symbol_names.len());
    for symbol in linked_symbol_names {
        let profile = resolved_profiles
            .get(symbol)
            .expect("linked symbol must exist in resolved map");
        linked.push((symbol.to_string(), profile.clone()));
    }
    Ok(linked)
}

fn resolved_external_import_profiles(
    external_imports: &[ExternalBuiltinSig],
) -> Result<std::collections::BTreeMap<String, AbiLinkProfile>, String> {
    use std::collections::BTreeMap;

    let mut resolved_profiles: BTreeMap<String, AbiLinkProfile> = BTreeMap::new();
    for import in external_imports {
        let profile = AbiLinkProfile {
            effect: effect_to_canonical(import.effect).to_string(),
            params: import
                .params
                .iter()
                .map(|param| type_to_canonical(&param.ty))
                .collect(),
            ret: type_to_canonical(&import.ret),
            capability: None,
        };
        if let Some(existing) = resolved_profiles.get(import.name.as_str()) {
            if !abi_signatures_match(existing, &profile) {
                return Err(format!(
                    "strict ABI/link mismatch for symbol `{}`: conflicting resolved external import signatures",
                    import.name
                ));
            }
            continue;
        }
        resolved_profiles.insert(import.name.clone(), profile);
    }
    Ok(resolved_profiles)
}

fn evaluate_strict_gates(
    expected_profiles: &std::collections::BTreeMap<String, AbiLinkProfile>,
    linked_imports: &[(String, AbiLinkProfile)],
    host_profile: &StrictHostProfileV0,
) -> Vec<StrictGateViolation> {
    use std::collections::{HashMap, HashSet};

    let symbol_package_index: HashMap<&str, String> = expected_profiles
        .keys()
        .map(|symbol| (symbol.as_str(), package_id_from_symbol(symbol)))
        .collect();

    let host_caps: HashSet<&str> = host_profile
        .capabilities
        .iter()
        .map(String::as_str)
        .collect();
    let mut diagnostics = Vec::new();
    for (symbol, linked_profile) in linked_imports {
        let Some(expected) = expected_profiles.get(symbol) else {
            diagnostics.push(StrictGateViolation {
                code: "C105",
                package: package_id_from_symbol(symbol),
                symbol: symbol.clone(),
                message: format!(
                    "strict ABI/link mismatch: linked import `{}` is not declared in strict package ABI",
                    symbol
                ),
            });
            continue;
        };

        if !abi_signatures_match(expected, linked_profile) {
            diagnostics.push(StrictGateViolation {
                code: "C105",
                package: symbol_package_index
                    .get(symbol.as_str())
                    .cloned()
                    .unwrap_or_else(|| package_id_from_symbol(symbol)),
                symbol: symbol.clone(),
                message: format!(
                    "strict ABI/link mismatch for symbol `{}`: expected effect/signature `{}({}) -> {}` but resolved `{}({}) -> {}`",
                    symbol,
                    expected.effect,
                    expected.params.join(", "),
                    expected.ret,
                    linked_profile.effect,
                    linked_profile.params.join(", "),
                    linked_profile.ret
                ),
            });
            continue;
        }

        if let Some(capability) = expected.capability.as_deref() {
            if !strict_capability_allowed_for_profile(host_profile.profile.as_str(), capability) {
                diagnostics.push(StrictGateViolation {
                    code: "C106",
                    package: symbol_package_index
                        .get(symbol.as_str())
                        .cloned()
                        .unwrap_or_else(|| package_id_from_symbol(symbol)),
                    symbol: symbol.clone(),
                    message: format!(
                        "strict runtime capability mismatch: symbol `{}` requires capability `{}` denied by strict deterministic policy for profile `{}`",
                        symbol, capability, host_profile.profile
                    ),
                });
                continue;
            }
            if !host_caps.contains(capability) {
                diagnostics.push(StrictGateViolation {
                    code: "C106",
                    package: symbol_package_index
                        .get(symbol.as_str())
                        .cloned()
                        .unwrap_or_else(|| package_id_from_symbol(symbol)),
                    symbol: symbol.clone(),
                    message: format!(
                        "strict runtime capability mismatch: symbol `{}` requires capability `{}` not present in host profile `{}`",
                        symbol, capability, host_profile.profile
                    ),
                });
            }
        }
    }

    sort_strict_gate_violations(diagnostics.as_mut_slice());
    diagnostics
}

fn strict_capability_allowed_for_profile(profile: &str, capability: &str) -> bool {
    match (profile, capability) {
        // Phase 21 lock: strict mode denies nondeterministic env capabilities.
        ("contract_static", "std::env::time")
        | ("contract_static", "std::env::random")
        | ("shared_app", "std::env::time")
        | ("shared_app", "std::env::random") => false,
        _ => true,
    }
}

fn sort_strict_gate_violations(violations: &mut [StrictGateViolation]) {
    violations.sort_by(|lhs, rhs| {
        lhs.code
            .cmp(rhs.code)
            .then(lhs.package.cmp(&rhs.package))
            .then(lhs.symbol.cmp(&rhs.symbol))
            .then(lhs.message.cmp(&rhs.message))
    });
}

fn strict_gate_outcome_from_linked_import_resolution(
    expected_profiles: &std::collections::BTreeMap<String, AbiLinkProfile>,
    host_profile: &StrictHostProfileV0,
    linked_import_resolution: std::result::Result<Vec<(String, AbiLinkProfile)>, String>,
) -> StrictGateOutcome {
    let (linked_imports, mut violations) = match linked_import_resolution {
        Ok(linked_imports) => (linked_imports, Vec::new()),
        Err(message) => (
            Vec::new(),
            vec![StrictGateViolation {
                code: "C105",
                package: "_".to_string(),
                symbol: "_".to_string(),
                message,
            }],
        ),
    };
    violations.extend(evaluate_strict_gates(
        expected_profiles,
        linked_imports.as_slice(),
        host_profile,
    ));
    sort_strict_gate_violations(violations.as_mut_slice());
    StrictGateOutcome {
        linked_imports,
        violations,
    }
}

fn package_id_from_symbol(symbol: &str) -> String {
    let mut segments = symbol.split("::");
    let first = segments.next().unwrap_or_default();
    let second = segments.next().unwrap_or_default();
    if first.is_empty() {
        "_".to_string()
    } else if second.is_empty() {
        first.to_string()
    } else {
        format!("{first}::{second}")
    }
}

#[cfg(test)]
fn strict_abi_link_violation(
    package_contract: &StrictPackageContractV0,
    linked_imports: &[ExternalBuiltinSig],
) -> Option<String> {
    let bindings = match strict_external_bindings_from_contract(package_contract) {
        Ok(value) => value,
        Err(message) => return Some(message),
    };
    let linked = match resolved_external_import_profiles(linked_imports) {
        Ok(value) => value.into_iter().collect::<Vec<_>>(),
        Err(message) => return Some(message),
    };
    let diagnostics = evaluate_strict_gates(
        &bindings.expected_profiles,
        linked.as_slice(),
        &StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: Vec::new(),
        },
    );
    diagnostics
        .into_iter()
        .find(|diag| diag.code == "C105")
        .map(|diag| diag.message)
}

#[cfg(test)]
fn strict_runtime_capability_violation(
    package_contract: &StrictPackageContractV0,
    host_profile: &StrictHostProfileV0,
    linked_imports: &[ExternalBuiltinSig],
) -> Option<String> {
    let bindings = match strict_external_bindings_from_contract(package_contract) {
        Ok(value) => value,
        Err(_) => return None,
    };
    let linked = match resolved_external_import_profiles(linked_imports) {
        Ok(value) => value.into_iter().collect::<Vec<_>>(),
        Err(_) => return None,
    };
    let diagnostics =
        evaluate_strict_gates(&bindings.expected_profiles, linked.as_slice(), host_profile);
    diagnostics
        .into_iter()
        .find(|diag| diag.code == "C106")
        .map(|diag| diag.message)
}

fn abi_signatures_match(expected: &AbiLinkProfile, actual: &AbiLinkProfile) -> bool {
    expected.effect == actual.effect
        && expected.params == actual.params
        && expected.ret == actual.ret
}

fn effect_to_canonical(effect: Effect) -> &'static str {
    match effect {
        Effect::Pure => "pure",
        Effect::Mut => "mut",
        Effect::Io => "io",
        Effect::None => "none",
    }
}

fn type_to_canonical(ty: &Type) -> String {
    match ty {
        Type::Int => "Int".to_string(),
        Type::U8 => "U8".to_string(),
        Type::U64 => "U64".to_string(),
        Type::U128 => "U128".to_string(),
        Type::U256 => "U256".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::String => "String".to_string(),
        Type::Bytes => "Bytes".to_string(),
        Type::Named { name, args } => {
            if args.is_empty() {
                name.clone()
            } else {
                let args = args
                    .iter()
                    .map(type_to_canonical)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{name}<{args}>")
            }
        }
        Type::Option(inner) => format!("Option<{}>", type_to_canonical(inner)),
        Type::Result(ok, err) => format!(
            "Result<{},{}>",
            type_to_canonical(ok),
            type_to_canonical(err)
        ),
        Type::List(inner) => format!("List<{}>", type_to_canonical(inner)),
        Type::Set(inner) => format!("Set<{}>", type_to_canonical(inner)),
        Type::Map(key, value) => format!(
            "Map<{},{}>",
            type_to_canonical(key),
            type_to_canonical(value)
        ),
        Type::Array(inner, len) => match len {
            Some(len) => format!("[{}; {}]", type_to_canonical(inner), len),
            None => format!("[{}]", type_to_canonical(inner)),
        },
        Type::Slice(inner) => format!("Slice<{}>", type_to_canonical(inner)),
        Type::Tuple(items) => format!(
            "({})",
            items
                .iter()
                .map(type_to_canonical)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Type::Fn { params, ret } => format!(
            "Fn({})->{}",
            params
                .iter()
                .map(type_to_canonical)
                .collect::<Vec<_>>()
                .join(","),
            type_to_canonical(ret)
        ),
    }
}

fn normalize_type_contract(raw: &str) -> String {
    raw.chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect::<String>()
}

fn strict_import_map_artifact_path(out: &Path) -> PathBuf {
    let file_name = out
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("out.wasm");
    let artifact_name = if let Some(stem) = file_name.strip_suffix(".wasm") {
        format!("{stem}.strict-import-map.json")
    } else {
        format!("{file_name}.strict-import-map.json")
    };
    match out.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(artifact_name),
        _ => PathBuf::from(artifact_name),
    }
}

fn write_strict_import_map_artifact(
    out: &Path,
    artifact: &StrictImportMapArtifact,
) -> Result<PathBuf> {
    let path = strict_import_map_artifact_path(out);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    fs::write(&path, artifact.canonical_bytes.as_slice())
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

fn strict_import_map_artifact_with_determinism_check(
    expected_profiles: &std::collections::BTreeMap<String, AbiLinkProfile>,
    host_profile: &StrictHostProfileV0,
    baseline_outcome: &StrictGateOutcome,
    replay_outcome: &StrictGateOutcome,
) -> std::result::Result<StrictImportMapArtifact, String> {
    let baseline_json = strict_import_map_artifact_json(
        expected_profiles,
        host_profile,
        baseline_outcome.linked_imports.as_slice(),
        baseline_outcome.violations.as_slice(),
    );
    let baseline_bytes = canonical_json_bytes(&baseline_json);
    let baseline_hash = sha256_hex(baseline_bytes.as_slice());

    let replay_json = strict_import_map_artifact_json(
        expected_profiles,
        host_profile,
        replay_outcome.linked_imports.as_slice(),
        replay_outcome.violations.as_slice(),
    );
    let replay_bytes = canonical_json_bytes(&replay_json);
    let replay_hash = sha256_hex(replay_bytes.as_slice());

    if baseline_outcome.violations.as_slice() != replay_outcome.violations.as_slice()
        || baseline_bytes != replay_bytes
        || baseline_hash != replay_hash
    {
        return Err(
            "strict determinism replay failed: identical inputs produced different canonical direct-dependency import map or diagnostics ordering"
                .to_string(),
        );
    }

    Ok(StrictImportMapArtifact {
        canonical_bytes: baseline_bytes,
        canonical_hash: baseline_hash,
    })
}

fn strict_import_map_artifact_json(
    expected_profiles: &std::collections::BTreeMap<String, AbiLinkProfile>,
    host_profile: &StrictHostProfileV0,
    linked_imports: &[(String, AbiLinkProfile)],
    diagnostics: &[StrictGateViolation],
) -> serde_json::Value {
    use serde_json::json;

    let mut imports = linked_imports
        .iter()
        .map(|(symbol, linked_profile)| {
            let required_capability = expected_profiles
                .get(symbol)
                .and_then(|profile| profile.capability.as_deref());
            json!({
                "symbol": symbol,
                "package": package_id_from_symbol(symbol),
                "effect": linked_profile.effect,
                "params": linked_profile.params,
                "ret": linked_profile.ret,
                "required_capability": required_capability,
            })
        })
        .collect::<Vec<_>>();
    imports.sort_by(|lhs, rhs| {
        let lhs_symbol = lhs
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let rhs_symbol = rhs
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        lhs_symbol.cmp(rhs_symbol)
    });

    let diagnostics_json = diagnostics
        .iter()
        .map(|diagnostic| {
            json!({
                "code": diagnostic.code,
                "package": diagnostic.package,
                "symbol": diagnostic.symbol,
                "message": diagnostic.message,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "schema_version": 0,
        "kind": "clg.strict_direct_dependency_import_map.v0",
        "host_profile": host_profile.profile,
        "imports": imports,
        "diagnostics": diagnostics_json,
    })
}

#[cfg(test)]
fn strict_determinism_violation(
    expected_profiles: &std::collections::BTreeMap<String, AbiLinkProfile>,
    host_profile: &StrictHostProfileV0,
    linked_imports: &[(String, AbiLinkProfile)],
    baseline_diagnostics: &[StrictGateViolation],
) -> Option<String> {
    let baseline_outcome = StrictGateOutcome {
        linked_imports: linked_imports.to_vec(),
        violations: baseline_diagnostics.to_vec(),
    };
    let mut replay_inputs = linked_imports.to_vec();
    replay_inputs.reverse();
    let replay_outcome = StrictGateOutcome {
        linked_imports: replay_inputs.clone(),
        violations: evaluate_strict_gates(
            expected_profiles,
            replay_inputs.as_slice(),
            host_profile,
        ),
    };
    strict_import_map_artifact_with_determinism_check(
        expected_profiles,
        host_profile,
        &baseline_outcome,
        &replay_outcome,
    )
    .err()
}

fn strict_artifact_identity_violation(
    lockfile: &StrictLockfileV0,
    package_contract: &StrictPackageContractV0,
) -> Option<String> {
    let mut lock_idx = 0usize;
    let mut package_idx = 0usize;
    while lock_idx < lockfile.dependencies.len() && package_idx < package_contract.packages.len() {
        let lock = &lockfile.dependencies[lock_idx];
        let package = &package_contract.packages[package_idx];
        match lock.name.cmp(&package.name) {
            std::cmp::Ordering::Less => {
                return Some(format!(
                    "strict artifact identity mismatch: lockfile dependency `{}` is missing from package metadata",
                    lock.name
                ));
            }
            std::cmp::Ordering::Greater => {
                return Some(format!(
                    "strict artifact identity mismatch: package metadata entry `{}` is not pinned in lockfile",
                    package.name
                ));
            }
            std::cmp::Ordering::Equal => {
                if lock.version != package.version {
                    return Some(format!(
                        "strict artifact identity mismatch for `{}`: lockfile version `{}` does not match package metadata version `{}`",
                        lock.name, lock.version, package.version
                    ));
                }
                if lock.digest != package.digest {
                    return Some(format!(
                        "strict artifact identity mismatch for `{}`: lockfile digest `{}` does not match package metadata digest `{}`",
                        lock.name, lock.digest, package.digest
                    ));
                }
                lock_idx += 1;
                package_idx += 1;
            }
        }
    }
    if let Some(lock) = lockfile.dependencies.get(lock_idx) {
        return Some(format!(
            "strict artifact identity mismatch: lockfile dependency `{}` is missing from package metadata",
            lock.name
        ));
    }
    if let Some(package) = package_contract.packages.get(package_idx) {
        return Some(format!(
            "strict artifact identity mismatch: package metadata entry `{}` is not pinned in lockfile",
            package.name
        ));
    }
    None
}

fn default_assurance_manifest_path(sig_path: &Path) -> PathBuf {
    let file_name = sig_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("assurance-manifest.json");
    let replacement = if let Some(stripped) = file_name.strip_suffix(".sig.json") {
        format!("{stripped}.assurance.json")
    } else if let Some(stem) = sig_path.file_stem().and_then(|stem| stem.to_str()) {
        format!("{stem}.assurance.json")
    } else {
        "assurance-manifest.json".to_string()
    };
    match sig_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(replacement),
        _ => PathBuf::from(replacement),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clg_ast::{Param, ParamKind};

    #[test]
    fn default_assurance_manifest_path_rewrites_sig_suffix() {
        let path = Path::new("artifacts/out.sig.json");
        assert_eq!(
            default_assurance_manifest_path(path),
            PathBuf::from("artifacts/out.assurance.json")
        );
    }

    #[test]
    fn default_assurance_manifest_path_appends_when_sig_suffix_missing() {
        let path = Path::new("artifacts/signature.json");
        assert_eq!(
            default_assurance_manifest_path(path),
            PathBuf::from("artifacts/signature.assurance.json")
        );
    }

    fn abi_contract_with_imports(
        imports: Vec<strict_package_contract::StrictAbiImportEntry>,
    ) -> StrictPackageContractV0 {
        StrictPackageContractV0 {
            packages: vec![strict_package_contract::StrictPackageMetadataEntry {
                name: "std::core".to_string(),
                version: "1.0.0".to_string(),
                digest: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
                artifact_format: "wasm".to_string(),
                artifact_path: "store/std-core-1.0.0.wasm".to_string(),
                abi_id: "abi:std::core:1.0.0".to_string(),
            }],
            contracts: vec![strict_package_contract::StrictAbiContractEntry {
                abi_id: "abi:std::core:1.0.0".to_string(),
                package: "std::core".to_string(),
                version: "1.0.0".to_string(),
                imports,
            }],
        }
    }

    fn external_sig(
        name: &str,
        params: Vec<Type>,
        ret: Type,
        effect: Effect,
    ) -> ExternalBuiltinSig {
        ExternalBuiltinSig {
            name: name.to_string(),
            params: params
                .into_iter()
                .enumerate()
                .map(|(idx, ty)| Param {
                    kind: ParamKind::Borrow,
                    name: format!("p{idx}"),
                    ty,
                })
                .collect(),
            ret,
            effect,
        }
    }

    #[test]
    fn strict_abi_link_accepts_exact_symbol_signature_match() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::core::math::add".to_string(),
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: None,
            }]);
        let actual = vec![external_sig(
            "std::core::math::add",
            vec![Type::Int, Type::Int],
            Type::Int,
            Effect::Pure,
        )];
        assert!(strict_abi_link_violation(&package_contract, &actual).is_none());
    }

    #[test]
    fn strict_abi_link_rejects_missing_expected_symbol() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::core::math::add".to_string(),
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: None,
            }]);
        assert!(
            strict_abi_link_violation(&package_contract, &[]).is_none(),
            "unlinked symbols should not fail strict ABI/link gate"
        );
    }

    #[test]
    fn strict_abi_link_rejects_effect_or_signature_mismatch() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::core::math::add".to_string(),
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: None,
            }]);
        let actual = vec![external_sig(
            "std::core::math::add",
            vec![Type::Int, Type::Int],
            Type::Int,
            Effect::Mut,
        )];
        let err =
            strict_abi_link_violation(&package_contract, &actual).expect("signature mismatch");
        assert!(err.contains("expected effect/signature"));
    }

    #[test]
    fn strict_abi_link_rejects_conflicting_profiles_for_same_symbol() {
        let package_contract = StrictPackageContractV0 {
            packages: vec![
                strict_package_contract::StrictPackageMetadataEntry {
                    name: "std::core".to_string(),
                    version: "1.0.0".to_string(),
                    digest:
                        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    artifact_format: "wasm".to_string(),
                    artifact_path: "store/std-core-1.0.0.wasm".to_string(),
                    abi_id: "abi:std::core:1.0.0".to_string(),
                },
                strict_package_contract::StrictPackageMetadataEntry {
                    name: "std::math".to_string(),
                    version: "1.0.0".to_string(),
                    digest:
                        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                            .to_string(),
                    artifact_format: "wasm".to_string(),
                    artifact_path: "store/std-math-1.0.0.wasm".to_string(),
                    abi_id: "abi:std::math:1.0.0".to_string(),
                },
            ],
            contracts: vec![
                strict_package_contract::StrictAbiContractEntry {
                    abi_id: "abi:std::core:1.0.0".to_string(),
                    package: "std::core".to_string(),
                    version: "1.0.0".to_string(),
                    imports: vec![strict_package_contract::StrictAbiImportEntry {
                        symbol: "std::crypto::hash".to_string(),
                        effect: "pure".to_string(),
                        params: vec!["Bytes".to_string()],
                        ret: "Bytes".to_string(),
                        capability: Some("std::crypto::hash".to_string()),
                    }],
                },
                strict_package_contract::StrictAbiContractEntry {
                    abi_id: "abi:std::math:1.0.0".to_string(),
                    package: "std::math".to_string(),
                    version: "1.0.0".to_string(),
                    imports: vec![strict_package_contract::StrictAbiImportEntry {
                        symbol: "std::crypto::hash".to_string(),
                        effect: "pure".to_string(),
                        params: vec!["Bytes".to_string()],
                        ret: "Bytes".to_string(),
                        capability: None,
                    }],
                },
            ],
        };
        let err = strict_abi_link_violation(&package_contract, &[])
            .expect("conflicting ABI profiles should fail");
        assert!(err.contains("conflicting ABI profiles"));
    }

    #[test]
    fn strict_runtime_capability_accepts_required_capability_present() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::crypto::hash".to_string(),
                effect: "pure".to_string(),
                params: vec!["Bytes".to_string()],
                ret: "Bytes".to_string(),
                capability: Some("std::crypto::hash".to_string()),
            }]);
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: vec!["std::crypto::hash".to_string()],
        };
        let linked = vec![external_sig(
            "std::crypto::hash",
            vec![Type::Bytes],
            Type::Bytes,
            Effect::Pure,
        )];
        assert!(
            strict_runtime_capability_violation(&package_contract, &host_profile, &linked)
                .is_none()
        );
    }

    #[test]
    fn strict_runtime_capability_rejects_missing_required_capability() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::crypto::hash".to_string(),
                effect: "pure".to_string(),
                params: vec!["Bytes".to_string()],
                ret: "Bytes".to_string(),
                capability: Some("std::crypto::hash".to_string()),
            }]);
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: vec!["std::wasi::print".to_string()],
        };
        let linked = vec![external_sig(
            "std::crypto::hash",
            vec![Type::Bytes],
            Type::Bytes,
            Effect::Pure,
        )];
        let err = strict_runtime_capability_violation(&package_contract, &host_profile, &linked)
            .expect("missing capability should fail");
        assert!(err.contains("not present in host profile"));
    }

    #[test]
    fn strict_runtime_capability_rejects_env_time_in_contract_static_even_if_present() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::env::time".to_string(),
                effect: "io".to_string(),
                params: vec![],
                ret: "Int".to_string(),
                capability: Some("std::env::time".to_string()),
            }]);
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: vec!["std::env::time".to_string()],
        };
        let linked = vec![external_sig(
            "std::env::time",
            vec![],
            Type::Int,
            Effect::Io,
        )];
        let err = strict_runtime_capability_violation(&package_contract, &host_profile, &linked)
            .expect("env time should be denied in strict contract_static");
        assert!(err.contains("denied by strict deterministic policy"));
    }

    #[test]
    fn strict_runtime_capability_rejects_env_random_in_shared_app_even_if_present() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::env::random".to_string(),
                effect: "io".to_string(),
                params: vec!["Int".to_string()],
                ret: "Bytes".to_string(),
                capability: Some("std::env::random".to_string()),
            }]);
        let host_profile = StrictHostProfileV0 {
            profile: "shared_app".to_string(),
            capabilities: vec!["std::env::random".to_string()],
        };
        let linked = vec![external_sig(
            "std::env::random",
            vec![Type::Int],
            Type::Bytes,
            Effect::Io,
        )];
        let err = strict_runtime_capability_violation(&package_contract, &host_profile, &linked)
            .expect("env random should be denied in strict shared_app");
        assert!(err.contains("denied by strict deterministic policy"));
    }

    #[test]
    fn strict_determinism_gate_accepts_stable_import_map() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::core::math::add".to_string(),
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: None,
            }]);
        let actual = vec![external_sig(
            "std::core::math::add",
            vec![Type::Int, Type::Int],
            Type::Int,
            Effect::Pure,
        )];
        let bindings =
            strict_external_bindings_from_contract(&package_contract).expect("strict bindings");
        let linked = resolved_external_import_profiles(&actual)
            .expect("resolve profiles")
            .into_iter()
            .collect::<Vec<_>>();
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: Vec::new(),
        };
        let diagnostics = evaluate_strict_gates(
            &bindings.expected_profiles,
            linked.as_slice(),
            &host_profile,
        );
        assert!(strict_determinism_violation(
            &bindings.expected_profiles,
            &host_profile,
            linked.as_slice(),
            diagnostics.as_slice(),
        )
        .is_none());
    }

    #[test]
    fn strict_determinism_gate_detects_order_dependent_external_conflict() {
        let package_contract = abi_contract_with_imports(vec![
            strict_package_contract::StrictAbiImportEntry {
                symbol: "std::core::math::add".to_string(),
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: Some("std::crypto::hash".to_string()),
            },
            strict_package_contract::StrictAbiImportEntry {
                symbol: "std::core::math::sub".to_string(),
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: Some("std::crypto::hash".to_string()),
            },
        ]);
        let actual = vec![
            external_sig(
                "std::core::math::add",
                vec![Type::Int, Type::Int],
                Type::Int,
                Effect::Pure,
            ),
            external_sig(
                "std::core::math::sub",
                vec![Type::Int, Type::Int],
                Type::Int,
                Effect::Pure,
            ),
        ];
        let bindings =
            strict_external_bindings_from_contract(&package_contract).expect("strict bindings");
        let linked = resolved_external_import_profiles(&actual)
            .expect("resolve profiles")
            .into_iter()
            .collect::<Vec<_>>();
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: Vec::new(),
        };
        let mut diagnostics = evaluate_strict_gates(
            &bindings.expected_profiles,
            linked.as_slice(),
            &host_profile,
        );
        diagnostics.reverse();
        let err = strict_determinism_violation(
            &bindings.expected_profiles,
            &host_profile,
            linked.as_slice(),
            diagnostics.as_slice(),
        )
        .expect("diagnostics-order drift should fail determinism replay");
        assert!(err.contains("determinism replay failed"));
    }

    #[test]
    fn strict_import_map_artifact_includes_pre_evaluation_violations() {
        let expected_profiles = std::collections::BTreeMap::new();
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: Vec::new(),
        };
        let linked_imports: Vec<(String, AbiLinkProfile)> = Vec::new();
        let pre_eval_violations = vec![StrictGateViolation {
            code: "C105",
            package: "_".to_string(),
            symbol: "_".to_string(),
            message: "strict ABI/link mismatch: synthetic pre-eval failure".to_string(),
        }];
        let baseline_outcome = StrictGateOutcome {
            linked_imports: linked_imports.clone(),
            violations: pre_eval_violations.clone(),
        };
        let replay_outcome = baseline_outcome.clone();
        let artifact = strict_import_map_artifact_with_determinism_check(
            &expected_profiles,
            &host_profile,
            &baseline_outcome,
            &replay_outcome,
        )
        .expect("artifact generation should include pre-evaluation violations");
        let value: serde_json::Value =
            serde_json::from_slice(artifact.canonical_bytes.as_slice()).expect("artifact json");
        let diagnostics = value
            .get("diagnostics")
            .and_then(|items| items.as_array())
            .expect("diagnostics array");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0]
                .get("code")
                .and_then(|code| code.as_str())
                .unwrap_or_default(),
            "C105"
        );
    }

    #[test]
    fn evaluate_strict_gates_orders_by_code_then_package_then_symbol() {
        let expected_profiles = std::collections::BTreeMap::from([
            (
                "zzz::alpha::f".to_string(),
                AbiLinkProfile {
                    effect: "pure".to_string(),
                    params: vec!["Int".to_string()],
                    ret: "Int".to_string(),
                    capability: Some("std::crypto::hash".to_string()),
                },
            ),
            (
                "aaa::beta::g".to_string(),
                AbiLinkProfile {
                    effect: "pure".to_string(),
                    params: vec!["Int".to_string()],
                    ret: "Int".to_string(),
                    capability: Some("std::crypto::hash".to_string()),
                },
            ),
        ]);
        let linked = vec![
            (
                "zzz::alpha::f".to_string(),
                AbiLinkProfile {
                    effect: "mut".to_string(),
                    params: vec!["Int".to_string()],
                    ret: "Int".to_string(),
                    capability: None,
                },
            ),
            (
                "aaa::beta::g".to_string(),
                AbiLinkProfile {
                    effect: "pure".to_string(),
                    params: vec!["Int".to_string()],
                    ret: "Int".to_string(),
                    capability: None,
                },
            ),
            (
                "mmm::orphan::h".to_string(),
                AbiLinkProfile {
                    effect: "pure".to_string(),
                    params: vec!["Int".to_string()],
                    ret: "Int".to_string(),
                    capability: None,
                },
            ),
        ];
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: Vec::new(),
        };
        let violations = evaluate_strict_gates(&expected_profiles, &linked, &host_profile);
        let order: Vec<(&str, &str, &str)> = violations
            .iter()
            .map(|v| (v.code, v.package.as_str(), v.symbol.as_str()))
            .collect();
        assert_eq!(
            order,
            vec![
                ("C105", "mmm::orphan", "mmm::orphan::h"),
                ("C105", "zzz::alpha", "zzz::alpha::f"),
                ("C106", "aaa::beta", "aaa::beta::g"),
            ]
        );
    }

    #[test]
    fn evaluate_strict_gates_snapshot_stable_output() {
        let expected_profiles = std::collections::BTreeMap::from([(
            "std::core::math::add".to_string(),
            AbiLinkProfile {
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: Some("std::crypto::hash".to_string()),
            },
        )]);
        let linked = vec![
            (
                "std::core::math::add".to_string(),
                AbiLinkProfile {
                    effect: "mut".to_string(),
                    params: vec!["Int".to_string(), "Int".to_string()],
                    ret: "Int".to_string(),
                    capability: None,
                },
            ),
            (
                "std::orphan::noop".to_string(),
                AbiLinkProfile {
                    effect: "pure".to_string(),
                    params: vec!["Int".to_string()],
                    ret: "Int".to_string(),
                    capability: None,
                },
            ),
        ];
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: Vec::new(),
        };
        let violations = evaluate_strict_gates(&expected_profiles, &linked, &host_profile);
        let snapshot = violations
            .iter()
            .map(|v| format!("{}|{}|{}", v.code, v.package, v.symbol))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            snapshot,
            "C105|std::core|std::core::math::add\nC105|std::orphan|std::orphan::noop"
        );
    }
}
