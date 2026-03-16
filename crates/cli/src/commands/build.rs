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
    let mut strict_package_contract_for_link: Option<StrictPackageContractV0> = None;
    let mut strict_host_profile_for_caps: Option<StrictHostProfileV0> = None;

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
        strict_package_contract_for_link = Some(strict_package_contract);
        strict_host_profile_for_caps = Some(strict_host_profile);
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
    if compiler_mode == CompilerMode::Strict {
        if let Some(package_contract) = strict_package_contract_for_link.as_ref() {
            if let Some(message) =
                strict_abi_link_violation(package_contract, external_typer_sigs.as_slice())
            {
                fail_build("C105", &message, None)?;
            }
            if let Some(host_profile) = strict_host_profile_for_caps.as_ref() {
                if let Some(message) =
                    strict_runtime_capability_violation(package_contract, host_profile)
                {
                    fail_build("C106", &message, None)?;
                }
            }
            if let Some(message) =
                strict_determinism_violation(package_contract, external_typer_sigs.as_slice())
            {
                fail_build("C107", &message, None)?;
            }
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

fn strict_abi_link_violation(
    package_contract: &StrictPackageContractV0,
    external_imports: &[ExternalBuiltinSig],
) -> Option<String> {
    use std::collections::BTreeMap;

    let mut expected: BTreeMap<&str, (&str, AbiLinkProfile)> = BTreeMap::new();
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
            if let Some((existing_abi_id, existing)) = expected.get(import.symbol.as_str()) {
                if existing != &profile {
                    return Some(format!(
                        "strict ABI/link mismatch for symbol `{}`: conflicting ABI profiles between `{}` and `{}`",
                        import.symbol, existing_abi_id, contract.abi_id
                    ));
                }
                continue;
            }
            expected.insert(import.symbol.as_str(), (contract.abi_id.as_str(), profile));
        }
    }

    let mut actual: BTreeMap<&str, AbiLinkProfile> = BTreeMap::new();
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
        if let Some(existing) = actual.get(import.name.as_str()) {
            if existing != &profile {
                return Some(format!(
                    "strict ABI/link mismatch for symbol `{}`: conflicting resolved external import signatures",
                    import.name
                ));
            }
            continue;
        }
        actual.insert(import.name.as_str(), profile);
    }

    for (symbol, (_, expected_profile)) in &expected {
        let Some(actual_profile) = actual.get(symbol) else {
            return Some(format!(
                "strict ABI/link mismatch: symbol `{}` declared in strict package ABI is not present in resolved external imports",
                symbol
            ));
        };
        if expected_profile.effect != actual_profile.effect
            || expected_profile.params != actual_profile.params
            || expected_profile.ret != actual_profile.ret
        {
            return Some(format!(
                "strict ABI/link mismatch for symbol `{}`: expected effect/signature `{}({}) -> {}` but resolved `{}({}) -> {}`",
                symbol,
                expected_profile.effect,
                expected_profile.params.join(", "),
                expected_profile.ret,
                actual_profile.effect,
                actual_profile.params.join(", "),
                actual_profile.ret
            ));
        }
    }

    for symbol in actual.keys() {
        if !expected.contains_key(symbol) {
            return Some(format!(
                "strict ABI/link mismatch: resolved external import `{}` is not declared in strict package ABI",
                symbol
            ));
        }
    }

    None
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

fn strict_runtime_capability_violation(
    package_contract: &StrictPackageContractV0,
    host_profile: &StrictHostProfileV0,
) -> Option<String> {
    use std::collections::{BTreeMap, HashSet};

    let host_caps: HashSet<&str> = host_profile
        .capabilities
        .iter()
        .map(String::as_str)
        .collect();
    let mut required_caps: BTreeMap<&str, &str> = BTreeMap::new();
    for contract in &package_contract.contracts {
        for import in &contract.imports {
            if let Some(capability) = import.capability.as_deref() {
                required_caps
                    .entry(capability)
                    .or_insert(import.symbol.as_str());
            }
        }
    }

    for (capability, symbol) in required_caps {
        if !host_caps.contains(capability) {
            return Some(format!(
                "strict runtime capability mismatch: symbol `{}` requires capability `{}` not present in host profile `{}`",
                symbol, capability, host_profile.profile
            ));
        }
    }

    None
}

fn strict_determinism_violation(
    package_contract: &StrictPackageContractV0,
    external_imports: &[ExternalBuiltinSig],
) -> Option<String> {
    let baseline = strict_canonical_import_map(package_contract, external_imports);
    let mut replay_inputs = external_imports.to_vec();
    replay_inputs.reverse();
    let replay = strict_canonical_import_map(package_contract, replay_inputs.as_slice());
    if baseline != replay {
        return Some(
            "strict determinism replay failed: identical inputs produced different canonical direct-dependency import map"
                .to_string(),
        );
    }
    None
}

fn strict_canonical_import_map(
    package_contract: &StrictPackageContractV0,
    external_imports: &[ExternalBuiltinSig],
) -> String {
    use std::collections::BTreeMap;

    let mut expected: BTreeMap<String, AbiLinkProfile> = BTreeMap::new();
    for contract in &package_contract.contracts {
        for import in &contract.imports {
            expected
                .entry(import.symbol.clone())
                .or_insert_with(|| AbiLinkProfile {
                    effect: import.effect.clone(),
                    params: import
                        .params
                        .iter()
                        .map(|ty| normalize_type_contract(ty))
                        .collect(),
                    ret: normalize_type_contract(&import.ret),
                    capability: import.capability.clone(),
                });
        }
    }

    let mut resolved: BTreeMap<String, AbiLinkProfile> = BTreeMap::new();
    for import in external_imports {
        resolved.insert(
            import.name.clone(),
            AbiLinkProfile {
                effect: effect_to_canonical(import.effect).to_string(),
                params: import
                    .params
                    .iter()
                    .map(|param| type_to_canonical(&param.ty))
                    .collect(),
                ret: type_to_canonical(&import.ret),
                capability: None,
            },
        );
    }

    let mut lines = Vec::new();
    for (symbol, expected_profile) in &expected {
        let resolved_profile = resolved.get(symbol);
        let resolved_sig = resolved_profile.map_or_else(
            || "<missing>".to_string(),
            |profile| {
                format!(
                    "{}({})->{}",
                    profile.effect,
                    profile.params.join(","),
                    profile.ret
                )
            },
        );
        lines.push(format!(
            "expected|{}|{}({})->{}|{}|{}",
            symbol,
            expected_profile.effect,
            expected_profile.params.join(","),
            expected_profile.ret,
            expected_profile.capability.as_deref().unwrap_or("-"),
            resolved_sig
        ));
    }
    for (symbol, resolved_profile) in &resolved {
        if !expected.contains_key(symbol) {
            lines.push(format!(
                "extra|{}|{}({})->{}",
                symbol,
                resolved_profile.effect,
                resolved_profile.params.join(","),
                resolved_profile.ret
            ));
        }
    }
    lines.join("\n")
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
        let err =
            strict_abi_link_violation(&package_contract, &[]).expect("missing symbol should fail");
        assert!(err.contains("not present in resolved external imports"));
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
        assert!(strict_runtime_capability_violation(&package_contract, &host_profile).is_none());
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
        let err = strict_runtime_capability_violation(&package_contract, &host_profile)
            .expect("missing capability should fail");
        assert!(err.contains("not present in host profile"));
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
        assert!(strict_determinism_violation(&package_contract, &actual).is_none());
    }

    #[test]
    fn strict_determinism_gate_detects_order_dependent_external_conflict() {
        let package_contract = abi_contract_with_imports(vec![]);
        let actual = vec![
            external_sig(
                "std::core::math::add",
                vec![Type::Int],
                Type::Int,
                Effect::Pure,
            ),
            external_sig(
                "std::core::math::add",
                vec![Type::Int],
                Type::Int,
                Effect::Mut,
            ),
        ];
        let err = strict_determinism_violation(&package_contract, &actual)
            .expect("order-dependent conflict should fail determinism replay");
        assert!(err.contains("determinism replay failed"));
    }
}
