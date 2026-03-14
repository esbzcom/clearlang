use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clg_ast::{Effect, Program, Type};
use clg_codegen_wasm::{emit_from_ir_with_opts, CodegenOpts, ExportAlias, ExternalImport};
use clg_ir::IrType;
use clg_typer::{
    check_with_vcs_with_std_and_external, ExternalBuiltinSig, TypecheckOutput, TyperError,
};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::commands::helpers::{extract_function_name, make_single_json_error, CommandError};
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
mod strict_trust_policy;
mod vcs_json;

use strict::{
    proof_strict_for_mode, strict_l3_claim_violation, strict_language_profile_violation,
    strict_proof_violation,
};
use strict_host_profile::load_required_host_profile_v0;
use strict_lockfile::{load_required_strict_lockfile_v0, StrictLockfileV0};
use strict_package_contract::{load_required_package_metadata_abi_v0, StrictPackageContractV0};
use strict_package_signatures::enforce_trust_gate_v0;
use strict_trust_policy::load_required_trust_policy_v0;
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
        let strict_lockfile = match load_required_strict_lockfile_v0(module_root) {
            Ok(value) => value,
            Err(err) => return fail_preflight(err.code(), err.message()),
        };
        let strict_trust_policy = match load_required_trust_policy_v0(module_root) {
            Ok(value) => value,
            Err(err) => return fail_preflight(err.code(), err.message()),
        };
        let strict_package_contract = match load_required_package_metadata_abi_v0(module_root) {
            Ok(value) => value,
            Err(err) => return fail_preflight(err.code(), err.message()),
        };
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
        if let Err(err) = load_required_host_profile_v0(module_root) {
            fail_preflight(err.code(), err.message())?;
        }
    }

    let mut timings = StageTimings::new();
    let loaded = {
        let _stage = timings.start(logger, "parse");
        load_program(&file, json_errors)?
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
}
