use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::{Cursor, Read};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::write::FileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

const SOLVER_VENDOR_SIGNING_KEY_ENV: &str = "CLG_SOLVER_VENDOR_SIGNING_KEY_HEX";
const DEFAULT_SOLVER_VENDOR_KEY_ID: &str = "z3-vendor-k7-2026q2";
const BINARY_RELEASE_SIGNING_KEY_ENV: &str = "CLG_BINARY_RELEASE_SIGNING_KEY_HEX";
const DEFAULT_BINARY_RELEASE_KEY_ID: &str = "milestone3-binary-ed25519-2026q2";
const SHARED_STD_SIGNING_KEY_ENV: &str = "CLG_SHARED_STD_SIGNING_KEY_HEX";
const DEFAULT_SHARED_STD_KEY_ID: &str = "shared-std-ed25519-2026q2";

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "help".to_string());
    let root = repo_root();

    match cmd.as_str() {
        "fmt" => cargo_cmd(&root, &["fmt", "--all"])?,
        "clippy" => cargo_cmd(
            &root,
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        )?,
        "test" => cargo_cmd(&root, &["test", "--workspace"])?,
        "release-precheck" => run_release_precheck(&root)?,
        "pilot-precheck" => run_pilot_precheck(&root, args.collect())?,
        "validate" => validate_samples(&root)?,
        "emit-vcs" => emit_vcs_sample(&root)?,
        "std-core-artifact" => emit_std_core_artifact(&root, args.collect())?,
        "shared-std-publish" => publish_shared_std_artifact(&root, args.collect())?,
        "solver-vendor-stage" => stage_solver_vendor(&root, args.collect())?,
        "manifest-lock-drift-check" => run_manifest_lock_drift_gate(&root, args.collect())?,
        "std-surface-drift-check" => check_std_surface_drift(&root, args.collect())?,
        "std-arch-sync" => run_std_arch_sync(&root, args.collect())?,
        "std-arch-conformance-check" => run_std_arch_conformance_check(&root, args.collect())?,
        "std-first-production-readiness-check" => {
            run_std_first_production_readiness_check(&root, args.collect())?
        }
        "shared-std-distribution-check" => {
            run_shared_std_distribution_check(&root, args.collect())?
        }
        "host-capability-policy-artifact" => {
            emit_host_capability_policy_artifact(&root, args.collect())?
        }
        "binary-repro-witness" => run_binary_repro_witness(&root, args.collect())?,
        "milestone3-binary-bundle" => emit_milestone3_binary_bundle(&root, args.collect())?,
        "milestone3-binary-bundle-verify" => {
            verify_milestone3_binary_bundle(&root, args.collect())?
        }
        "contract-verifier-package" => {
            emit_contract_verifier_package(&root, args.collect())?
        }
        "contract-verifier-package-verify" => {
            verify_contract_verifier_package(&root, args.collect())?
        }
        "milestone3-installer-channels" => {
            emit_milestone3_installer_channels(&root, args.collect())?
        }
        "milestone2-perf-gate" => run_milestone2_perf_gate(&root, args.collect())?,
        "milestone2-supply-chain-gate" => run_milestone2_supply_chain_gate(&root, args.collect())?,
        "ci" => {
            run_release_precheck(&root)?;
            cargo_cmd(&root, &["build", "--release", "-p", "clg-cli"])?;
            validate_samples(&root)?;
        }
        "help" | "-h" | "--help" => {
            print_help();
        }
        other => return Err(format!("unknown xtask command: {other}")),
    }

    Ok(())
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask in repo root/xtask")
        .to_path_buf()
}

fn cargo_cmd(root: &Path, args: &[&str]) -> Result<(), String> {
    run(Command::new("cargo").args(args).current_dir(root))
}

fn run_release_precheck(root: &Path) -> Result<(), String> {
    cargo_cmd(root, &["fmt", "--all", "--", "--check"])?;
    cargo_cmd(
        root,
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    cargo_cmd(root, &["test", "--workspace"])?;
    run_manifest_lock_drift_gate(root, Vec::new())?;
    run_std_arch_conformance_check(root, Vec::new())?;
    run_shared_std_distribution_check(root, Vec::new())?;
    run_clg_test_schema_gate(root)?;
    Ok(())
}

const PILOT_PRECHECK_KIND: &str = "clg.pilot-precheck-report.v1";
const PILOT_PRECHECK_SCOPE_FULL: &str = "full";
const PILOT_PRECHECK_GATES: [&str; 5] = [
    "release-precheck",
    "contract-release-fixtures",
    "verifier-distribution-fixtures",
    "caller-authorization-fixtures",
    "reference-pilot-offline-replay",
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct PilotPrecheckOpts {
    report: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct PilotPrecheckDigest {
    id: String,
    sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct PilotPrecheckTool {
    name: String,
    version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct PilotPrecheckGateReport {
    id: String,
    status: String,
    command: Vec<String>,
    input_digests: Vec<PilotPrecheckDigest>,
    evidence_digests: Vec<PilotPrecheckDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct PilotPrecheckReport {
    schema_version: u32,
    kind: String,
    scope: String,
    overall_status: String,
    toolchain: Vec<PilotPrecheckTool>,
    gate_order: Vec<String>,
    future_required_gates: Vec<String>,
    gates: Vec<PilotPrecheckGateReport>,
}

fn run_pilot_precheck(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_pilot_precheck_args(raw_args)?;
    let mut report = new_full_pilot_precheck_report(root)?;

    for index in 0..report.gates.len() {
        let gate_id = report.gates[index].id.clone();
        let outcome = match gate_id.as_str() {
            "release-precheck" => run_release_precheck(root),
            "contract-release-fixtures" => cargo_cmd(
                root,
                &[
                    "test",
                    "-p",
                    "clg-cli",
                    "--test",
                    "cli_it",
                    "basic::contract_release_",
                ],
            ),
            "verifier-distribution-fixtures" => {
                cargo_cmd(root, &["test", "--workspace", "verifier"])
            }
            "caller-authorization-fixtures" => cargo_cmd(
                root,
                &[
                    "test",
                    "-p",
                    "clg-cli",
                    "--test",
                    "cli_it",
                    "basic::caller_aware_",
                ],
            ),
            "reference-pilot-offline-replay" => cargo_cmd(
                root,
                &[
                    "test",
                    "-p",
                    "clg-cli",
                    "--test",
                    "cli_it",
                    "basic::pilot_evidence_replay_",
                ],
            ),
            _ => Err(format!("unregistered pilot-precheck gate `{gate_id}`")),
        };
        if let Err(error) = outcome {
            report.gates[index].status = "failed".to_string();
            report.gates[index].failure_code = Some("command_failed".to_string());
            report.overall_status = "failed".to_string();
            validate_pilot_precheck_report_against_root(root, &report)?;
            write_pilot_precheck_report(&opts.report, &report)?;
            return Err(format!(
                "pilot-precheck gate `{gate_id}` failed; report written: {error}"
            ));
        }
        report.gates[index].status = "passed".to_string();
    }

    validate_pilot_precheck_report_against_root(root, &report)?;
    write_pilot_precheck_report(&opts.report, &report)
}

fn parse_pilot_precheck_args(raw_args: Vec<String>) -> Result<PilotPrecheckOpts, String> {
    let mut report = None;
    let mut index = 0usize;
    while index < raw_args.len() {
        match raw_args[index].as_str() {
            "--report" => {
                index += 1;
                let value = raw_args
                    .get(index)
                    .ok_or_else(|| "missing value for `--report`".to_string())?;
                if report.replace(PathBuf::from(value)).is_some() {
                    return Err("`--report` may be supplied only once".to_string());
                }
            }
            other => {
                return Err(format!(
                    "unknown pilot-precheck arg `{other}` (supported: --report)"
                ));
            }
        }
        index += 1;
    }
    let report = report.ok_or_else(|| "missing required `--report`".to_string())?;
    if report.as_os_str().is_empty() {
        return Err("`--report` must not be empty".to_string());
    }
    Ok(PilotPrecheckOpts { report })
}

fn new_full_pilot_precheck_report(root: &Path) -> Result<PilotPrecheckReport, String> {
    Ok(PilotPrecheckReport {
        schema_version: 1,
        kind: PILOT_PRECHECK_KIND.to_string(),
        scope: PILOT_PRECHECK_SCOPE_FULL.to_string(),
        overall_status: "passed".to_string(),
        toolchain: vec![PilotPrecheckTool {
            name: "xtask".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }],
        gate_order: PILOT_PRECHECK_GATES
            .iter()
            .map(|gate| (*gate).to_string())
            .collect(),
        future_required_gates: Vec::new(),
        gates: vec![
            new_pilot_precheck_gate(
                root,
                "release-precheck",
                &["cargo", "run", "-p", "xtask", "--", "release-precheck"],
                &["Cargo.lock"],
            )?,
            new_pilot_precheck_gate(
                root,
                "contract-release-fixtures",
                &[
                    "cargo",
                    "test",
                    "-p",
                    "clg-cli",
                    "--test",
                    "cli_it",
                    "basic::contract_release_",
                ],
                &["crates/cli/tests/cli_it/basic/contract_test_campaign.rs"],
            )?,
            new_pilot_precheck_gate(
                root,
                "verifier-distribution-fixtures",
                &["cargo", "test", "--workspace", "verifier"],
                &[
                    "xtask/src/main/tests.rs",
                    "crates/cli/tests/cli_it/basic/contract_test_campaign.rs",
                ],
            )?,
            new_pilot_precheck_gate(
                root,
                "caller-authorization-fixtures",
                &[
                    "cargo",
                    "test",
                    "-p",
                    "clg-cli",
                    "--test",
                    "cli_it",
                    "basic::caller_aware_",
                ],
                &["crates/cli/tests/cli_it/basic/contract_test_campaign.rs"],
            )?,
            new_pilot_precheck_gate(
                root,
                "reference-pilot-offline-replay",
                &[
                    "cargo",
                    "test",
                    "-p",
                    "clg-cli",
                    "--test",
                    "cli_it",
                    "basic::pilot_evidence_replay_",
                ],
                &["crates/cli/tests/cli_it/basic/contract_target.rs"],
            )?,
        ],
    })
}

fn new_pilot_precheck_gate(
    root: &Path,
    id: &str,
    command: &[&str],
    checked_inputs: &[&str],
) -> Result<PilotPrecheckGateReport, String> {
    let digests = checked_inputs
        .iter()
        .map(|checked_input| {
            Ok(PilotPrecheckDigest {
                id: (*checked_input).to_string(),
                sha256: file_sha256_hex(&root.join(checked_input))?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(PilotPrecheckGateReport {
        id: id.to_string(),
        status: "not_run".to_string(),
        command: command.iter().map(|arg| (*arg).to_string()).collect(),
        input_digests: digests.clone(),
        evidence_digests: digests,
        failure_code: None,
    })
}

fn validate_pilot_precheck_report(report: &PilotPrecheckReport) -> Result<(), String> {
    if report.schema_version != 1 || report.kind != PILOT_PRECHECK_KIND {
        return Err("invalid pilot-precheck report schema".to_string());
    }
    if report.scope != PILOT_PRECHECK_SCOPE_FULL {
        return Err("pilot-precheck report must use the full scope".to_string());
    }
    let expected_order = PILOT_PRECHECK_GATES
        .iter()
        .map(|gate| (*gate).to_string())
        .collect::<Vec<_>>();
    if report.gate_order != expected_order {
        return Err("pilot-precheck gate order is missing, substituted, or reordered".to_string());
    }
    if !report.future_required_gates.is_empty() {
        return Err("full pilot-precheck report must not declare future required gates".to_string());
    }
    if report.gates.len() != expected_order.len()
        || report
            .gates
            .iter()
            .map(|gate| &gate.id)
            .ne(expected_order.iter())
    {
        return Err("pilot-precheck gate results are missing, substituted, or reordered".to_string());
    }
    if report.toolchain.is_empty()
        || report
            .toolchain
            .iter()
            .any(|tool| tool.name.is_empty() || tool.version.is_empty())
    {
        return Err("pilot-precheck report has invalid toolchain identity".to_string());
    }

    let has_failed = report.gates.iter().any(|gate| gate.status == "failed");
    let has_not_run = report.gates.iter().any(|gate| gate.status == "not_run");
    let expected_status = if has_failed || has_not_run { "failed" } else { "passed" };
    if report.overall_status != expected_status {
        return Err("pilot-precheck overall status does not match gate results".to_string());
    }
    for gate in &report.gates {
        if !matches!(gate.status.as_str(), "passed" | "failed" | "not_run") {
            return Err(format!("invalid pilot-precheck gate status `{}`", gate.status));
        }
        if gate.command.is_empty()
            || gate
                .command
                .iter()
                .any(|arg| arg.is_empty() || Path::new(arg).is_absolute())
        {
            return Err(format!("pilot-precheck gate `{}` has unsafe command", gate.id));
        }
        validate_pilot_precheck_digests(&gate.input_digests, &gate.id, "input")?;
        validate_pilot_precheck_digests(&gate.evidence_digests, &gate.id, "evidence")?;
        match gate.status.as_str() {
            "failed" if gate.failure_code.is_none() => {
                return Err(format!("failed gate `{}` is missing failure_code", gate.id));
            }
            "failed" => {}
            _ if gate.failure_code.is_some() => {
                return Err(format!("non-failed gate `{}` has failure_code", gate.id));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_pilot_precheck_report_against_root(
    root: &Path,
    report: &PilotPrecheckReport,
) -> Result<(), String> {
    validate_pilot_precheck_report(report)?;
    let expected = new_full_pilot_precheck_report(root)?;
    for (actual, expected) in report.gates.iter().zip(expected.gates.iter()) {
        if actual.input_digests != expected.input_digests
            || actual.evidence_digests != expected.evidence_digests
            || actual.command != expected.command
        {
            return Err(format!(
                "pilot-precheck gate `{}` has substituted command or evidence identity",
                actual.id
            ));
        }
    }
    Ok(())
}

fn validate_pilot_precheck_digests(
    digests: &[PilotPrecheckDigest],
    gate_id: &str,
    kind: &str,
) -> Result<(), String> {
    if digests.is_empty() {
        return Err(format!("pilot-precheck gate `{gate_id}` has no {kind} digests"));
    }
    for digest in digests {
        let path = Path::new(&digest.id);
        if digest.id.is_empty()
            || path.is_absolute()
            || digest.id.contains("..")
            || digest.id.contains('\\')
            || !digest
                .sha256
                .chars()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
            || digest.sha256.len() != 64
        {
            return Err(format!(
                "pilot-precheck gate `{gate_id}` has unsafe or malformed {kind} digest"
            ));
        }
    }
    Ok(())
}

fn write_pilot_precheck_report(path: &Path, report: &PilotPrecheckReport) -> Result<(), String> {
    let bytes = pretty_json_bytes(report)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create report directory `{}`: {error}", parent.display()))?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes)
        .map_err(|error| format!("write report `{}`: {error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| {
        format!(
            "atomically publish report `{}` from `{}`: {error}",
            path.display(),
            temporary.display()
        )
    })
}

fn run_manifest_lock_drift_gate(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_manifest_lock_drift_args(raw_args)?;
    if opts.paths.is_empty() {
        return Err("manifest-lock drift gate requires at least one check path".to_string());
    }
    for relative in &opts.paths {
        let dir = root.join(relative);
        check_manifest_lock_consistency_for_dir(&dir).map_err(|err| {
            format!(
                "manifest/lock drift at `{}`: {err}",
                dir.strip_prefix(root).unwrap_or(dir.as_path()).display()
            )
        })?;
    }
    Ok(())
}

fn run_binary_repro_witness(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_binary_repro_witness_args(raw_args)?;
    let out_path = opts.out.unwrap_or_else(|| {
        root.join("tmp")
            .join("binary-repro")
            .join(format!("{}.json", current_platform_id()))
    });
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create binary repro output dir `{}`: {e}", parent.display()))?;
    }

    let run1_dir = root.join("tmp").join("binary-repro").join("run1-target");
    let run2_dir = root.join("tmp").join("binary-repro").join("run2-target");
    if run1_dir.exists() {
        fs::remove_dir_all(&run1_dir)
            .map_err(|e| format!("cleanup `{}`: {e}", run1_dir.display()))?;
    }
    if run2_dir.exists() {
        fs::remove_dir_all(&run2_dir)
            .map_err(|e| format!("cleanup `{}`: {e}", run2_dir.display()))?;
    }

    run(Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("-p")
        .arg("clg-cli")
        .arg("--target-dir")
        .arg(&run1_dir)
        .current_dir(root))?;
    run(Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("-p")
        .arg("clg-cli")
        .arg("--target-dir")
        .arg(&run2_dir)
        .current_dir(root))?;

    let bin1 = clg_binary_path(&run1_dir);
    let bin2 = clg_binary_path(&run2_dir);
    let hash1 = file_sha256_hex(&bin1)?;
    let hash2 = file_sha256_hex(&bin2)?;
    let equal = hash1 == hash2;

    let artifact = serde_json::json!({
        "schema_version": 1,
        "platform": current_platform_id(),
        "binary_name": bin1.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| "clg".to_string()),
        "run1": {
            "path": bin1.display().to_string(),
            "sha256": hash1,
        },
        "run2": {
            "path": bin2.display().to_string(),
            "sha256": hash2,
        },
        "equal": equal,
    });
    write_json_pretty(&out_path, &artifact)?;
    if !equal {
        return Err(format!(
            "binary reproducibility mismatch on {}: `{}` vs `{}`",
            current_platform_id(),
            bin1.display(),
            bin2.display()
        ));
    }
    println!(
        "binary reproducibility witness written to {}",
        out_path.display()
    );
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Milestone3BundleArtifact {
    path: String,
    sha256: String,
    size_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Milestone3BundlePayload {
    schema_version: u32,
    kind: String,
    platform: String,
    key_id: String,
    binary: Milestone3BundleArtifact,
    checksums_manifest: Milestone3BundleArtifact,
    sbom_artifacts: Vec<Milestone3BundleArtifact>,
    license_artifacts: Vec<Milestone3BundleArtifact>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Milestone3BundleSignature {
    key_id: String,
    signature_format: String,
    payload_hash: String,
    signature: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Milestone3SignedBundleMetadata {
    schema_version: u32,
    payload: Milestone3BundlePayload,
    signature: Milestone3BundleSignature,
}

#[derive(Clone, Debug, Deserialize)]
struct Milestone3BundlePubkeyMetadata {
    schema_version: u32,
    key_id: String,
    scheme: String,
    public_key: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Milestone3InstallerChannelsManifest {
    schema_version: u32,
    version: String,
    release_tag: String,
    repository: String,
    release_notes_url: String,
    archives: Vec<Milestone3InstallerArchive>,
    homebrew: Milestone3HomebrewChannel,
    winget: Milestone3WingetChannel,
}

const CONTRACT_VERIFIER_MANIFEST_FORMAT: &str =
    "clg.contract-verifier-distribution-manifest.v1";
const CONTRACT_VERIFIER_SIGNATURE_FORMAT: &str =
    "clg.contract-verifier-distribution-signature.v1";
const CONTRACT_VERIFIER_RESULT_FORMAT: &str = "clg.contract-release-verifier-result.v1";
const CONTRACT_VERIFIER_EXECUTABLE: &str = "clg-contract-verifier";
const CONTRACT_VERIFIER_PLATFORMS: [(&str, &str); 3] = [
    ("linux-x86_64", "tar.gz"),
    ("macos-x86_64", "tar.gz"),
    ("windows-x86_64", "zip"),
];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ContractVerifierDistributionArtifact {
    archive: String,
    archive_sha256: String,
    executable: String,
    executable_sha256: String,
    platform: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ContractVerifierCapabilities {
    release_bundle_format: String,
    result_format: String,
    target_execution_profiles: Vec<String>,
    target_profiles: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ContractVerifierDistributionManifest {
    artifacts: Vec<ContractVerifierDistributionArtifact>,
    capabilities: ContractVerifierCapabilities,
    format: String,
    schema_version: u32,
    verifier_version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ContractVerifierDistributionSignature {
    format: String,
    key_id: String,
    manifest_sha256: String,
    schema_version: u32,
    signature: String,
    signature_format: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractVerifierSigningKeyFile {
    private_key: String,
    scheme: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractVerifierPublicKeyFile {
    public_key: String,
    scheme: String,
}

fn emit_contract_verifier_package(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_contract_verifier_package_args(raw_args)?;
    if opts.out_dir.exists() {
        return Err(format!(
            "refusing to overwrite existing verifier package directory `{}`",
            opts.out_dir.display()
        ));
    }
    let signing_key_path = resolve_from_root(root, &opts.signing_key);
    let signing_key = load_contract_verifier_signing_key(&signing_key_path)?;
    fs::create_dir_all(&opts.out_dir)
        .map_err(|e| format!("create package directory `{}`: {e}", opts.out_dir.display()))?;

    let mut artifacts = Vec::with_capacity(opts.artifacts.len());
    for (platform, supplied_path) in &opts.artifacts {
        let source = resolve_from_root(root, supplied_path);
        if !source.is_file() {
            return Err(format!(
                "verifier artifact for `{platform}` is not a file: `{}`",
                source.display()
            ));
        }
        let archive_name = contract_verifier_archive_name(&opts.version, platform)?;
        let executable = contract_verifier_executable_name(platform)?;
        let binary = fs::read(&source)
            .map_err(|e| format!("read verifier artifact `{}`: {e}", source.display()))?;
        let archive_path = opts.out_dir.join(&archive_name);
        if platform == "windows-x86_64" {
            create_contract_verifier_zip(&archive_path, executable, &binary)?;
        } else {
            create_contract_verifier_tar_gz(&archive_path, executable, &binary)?;
        }
        artifacts.push(ContractVerifierDistributionArtifact {
            archive: archive_name,
            archive_sha256: format!("sha256:{}", file_sha256_hex(&archive_path)?),
            executable: executable.to_string(),
            executable_sha256: format!("sha256:{}", hex::encode(Sha256::digest(&binary))),
            platform: platform.clone(),
        });
    }
    artifacts.sort_by(|left, right| left.platform.cmp(&right.platform));
    let manifest = ContractVerifierDistributionManifest {
        artifacts: artifacts.clone(),
        capabilities: ContractVerifierCapabilities {
            release_bundle_format: "clg.contract-release-bundle.v1".to_string(),
            result_format: CONTRACT_VERIFIER_RESULT_FORMAT.to_string(),
            target_execution_profiles: vec!["clg.evm-stateful-scalar.v1".to_string()],
            target_profiles: vec!["clg.evm-sepolia-pilot.v1".to_string()],
        },
        format: CONTRACT_VERIFIER_MANIFEST_FORMAT.to_string(),
        schema_version: 1,
        verifier_version: opts.version.clone(),
    };
    let manifest_bytes = canonical_distribution_json_bytes(&manifest)?;
    let manifest_name = format!("clg-contract-verifier-v{}-manifest.json", opts.version);
    let manifest_path = opts.out_dir.join(&manifest_name);
    fs::write(&manifest_path, &manifest_bytes)
        .map_err(|e| format!("write manifest `{}`: {e}", manifest_path.display()))?;

    let mut checksum_entries = artifacts
        .iter()
        .map(|artifact| {
            let digest = artifact
                .archive_sha256
                .strip_prefix("sha256:")
                .expect("package generator adds sha256 prefix");
            (artifact.archive.as_str(), digest)
        })
        .collect::<Vec<_>>();
    checksum_entries.sort_by(|left, right| left.0.cmp(right.0));
    let checksum_lines = checksum_entries
        .into_iter()
        .map(|(archive, digest)| format!("{digest}  {archive}"))
        .collect::<Vec<_>>();
    let checksums_name = format!("clg-contract-verifier-v{}-checksums.txt", opts.version);
    let checksums_path = opts.out_dir.join(&checksums_name);
    fs::write(&checksums_path, format!("{}\n", checksum_lines.join("\n")))
        .map_err(|e| format!("write checksums `{}`: {e}", checksums_path.display()))?;

    let signature = ContractVerifierDistributionSignature {
        format: CONTRACT_VERIFIER_SIGNATURE_FORMAT.to_string(),
        key_id: opts.key_id.clone(),
        manifest_sha256: format!("sha256:{}", hex::encode(Sha256::digest(&manifest_bytes))),
        schema_version: 1,
        signature: hex::encode(signing_key.sign(&manifest_bytes).to_bytes()),
        signature_format: "ed25519".to_string(),
    };
    let signature_name = format!("clg-contract-verifier-v{}-manifest.sig.json", opts.version);
    let signature_path = opts.out_dir.join(&signature_name);
    fs::write(&signature_path, canonical_distribution_json_bytes(&signature)?)
        .map_err(|e| format!("write manifest signature `{}`: {e}", signature_path.display()))?;

    verify_contract_verifier_package_dir(&opts.out_dir, &signing_key.verifying_key())?;
    println!(
        "contract verifier package verified: {} (version={})",
        opts.out_dir.display(),
        opts.version
    );
    Ok(())
}

fn verify_contract_verifier_package(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_contract_verifier_package_verify_args(raw_args)?;
    let package_dir = resolve_from_root(root, &opts.package_dir);
    let pubkey_path = resolve_from_root(root, &opts.pubkey);
    let public_key = load_contract_verifier_public_key(&pubkey_path)?;
    verify_contract_verifier_package_dir(&package_dir, &public_key)?;
    println!("contract verifier package verified: {}", package_dir.display());
    Ok(())
}

fn verify_contract_verifier_package_dir(
    package_dir: &Path,
    public_key: &VerifyingKey,
) -> Result<(), String> {
    let files = fs::read_dir(package_dir)
        .map_err(|e| format!("read package directory `{}`: {e}", package_dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("enumerate package directory `{}`: {e}", package_dir.display()))?;
    let actual_names = files
        .iter()
        .map(|entry| {
            if !entry
                .file_type()
                .map_err(|e| format!("read package entry type: {e}"))?
                .is_file()
            {
                return Err("verifier package must contain flat regular files only".to_string());
            }
            entry
                .file_name()
                .into_string()
                .map_err(|_| "verifier package file name is not UTF-8".to_string())
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let manifest_paths = files
        .iter()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("clg-contract-verifier-v")
                        && name.ends_with("-manifest.json")
                })
        })
        .collect::<Vec<_>>();
    if manifest_paths.len() != 1 {
        return Err(format!(
            "package directory `{}` must contain exactly one verifier manifest",
            package_dir.display()
        ));
    }
    let manifest_path = &manifest_paths[0];
    let manifest_bytes = fs::read(manifest_path)
        .map_err(|e| format!("read manifest `{}`: {e}", manifest_path.display()))?;
    let mut manifest_value: serde_json::Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| format!("parse manifest `{}`: {e}", manifest_path.display()))?;
    canonicalize_distribution_json(&mut manifest_value);
    let canonical_manifest = serde_json::to_vec(&manifest_value)
        .map_err(|e| format!("serialize canonical manifest: {e}"))?;
    if manifest_bytes != canonical_manifest {
        return Err("verifier manifest is not canonical JSON".to_string());
    }
    let manifest: ContractVerifierDistributionManifest = serde_json::from_value(manifest_value)
        .map_err(|e| format!("decode verifier manifest: {e}"))?;
    validate_contract_verifier_manifest(&manifest)?;
    let signature_path = package_dir.join(format!(
        "clg-contract-verifier-v{}-manifest.sig.json",
        manifest.verifier_version
    ));
    let checksums_path = package_dir.join(format!(
        "clg-contract-verifier-v{}-checksums.txt",
        manifest.verifier_version
    ));
    let mut expected_names = manifest
        .artifacts
        .iter()
        .map(|artifact| artifact.archive.clone())
        .collect::<BTreeSet<_>>();
    expected_names.insert(
        manifest_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "verifier manifest has no UTF-8 filename".to_string())?
            .to_string(),
    );
    expected_names.insert(
        signature_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "verifier manifest signature has no UTF-8 filename".to_string())?
            .to_string(),
    );
    expected_names.insert(
        checksums_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "verifier checksum listing has no UTF-8 filename".to_string())?
            .to_string(),
    );
    if actual_names != expected_names {
        return Err("verifier package contains missing or unbound files".to_string());
    }
    let signature_bytes = fs::read(&signature_path)
        .map_err(|e| format!("read manifest signature `{}`: {e}", signature_path.display()))?;
    let signature: ContractVerifierDistributionSignature = serde_json::from_slice(&signature_bytes)
        .map_err(|e| format!("parse manifest signature `{}`: {e}", signature_path.display()))?;
    if signature.format != CONTRACT_VERIFIER_SIGNATURE_FORMAT
        || signature.schema_version != 1
        || signature.signature_format != "ed25519"
        || signature.key_id.trim().is_empty()
        || signature.manifest_sha256
            != format!("sha256:{}", hex::encode(Sha256::digest(&manifest_bytes)))
    {
        return Err("verifier manifest signature has an unsupported format or digest".to_string());
    }
    let signature_bytes = hex::decode(&signature.signature)
        .map_err(|e| format!("decode verifier manifest signature: {e}"))?;
    let signature = Signature::try_from(signature_bytes.as_slice())
        .map_err(|e| format!("parse verifier manifest signature: {e}"))?;
    public_key
        .verify(&manifest_bytes, &signature)
        .map_err(|e| format!("verify verifier manifest signature: {e}"))?;

    let checksums = fs::read_to_string(&checksums_path)
        .map_err(|e| format!("read checksums `{}`: {e}", checksums_path.display()))?;
    let mut expected_checksum_entries = manifest
        .artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.archive.as_str(),
                artifact
                    .archive_sha256
                    .strip_prefix("sha256:")
                    .expect("manifest validator requires sha256 prefix"),
            )
        })
        .collect::<Vec<_>>();
    expected_checksum_entries.sort_by(|left, right| left.0.cmp(right.0));
    let expected_checksum_lines = expected_checksum_entries
        .into_iter()
        .map(|(archive, digest)| format!("{digest}  {archive}"))
        .collect::<Vec<_>>();
    if checksums
        != format!(
            "{}\n",
            expected_checksum_lines.join("\n")
        )
    {
        return Err("verifier checksum listing does not exactly bind the manifest archives".to_string());
    }
    for artifact in &manifest.artifacts {
        let archive_path = package_dir.join(&artifact.archive);
        if file_sha256_hex(&archive_path)? != artifact.archive_sha256[7..] {
            return Err(format!("archive digest mismatch for `{}`", artifact.archive));
        }
        verify_contract_verifier_archive(&archive_path, artifact)?;
    }
    Ok(())
}

fn validate_contract_verifier_manifest(
    manifest: &ContractVerifierDistributionManifest,
) -> Result<(), String> {
    if manifest.format != CONTRACT_VERIFIER_MANIFEST_FORMAT
        || manifest.schema_version != 1
        || !is_exact_semver(&manifest.verifier_version)
        || manifest.capabilities.release_bundle_format != "clg.contract-release-bundle.v1"
        || manifest.capabilities.result_format != CONTRACT_VERIFIER_RESULT_FORMAT
        || manifest.capabilities.target_profiles != ["clg.evm-sepolia-pilot.v1"]
        || manifest.capabilities.target_execution_profiles != ["clg.evm-stateful-scalar.v1"]
    {
        return Err("verifier manifest has an unsupported format or capability set".to_string());
    }
    let expected_platforms = CONTRACT_VERIFIER_PLATFORMS
        .iter()
        .map(|(platform, _)| *platform)
        .collect::<Vec<_>>();
    let actual_platforms = manifest
        .artifacts
        .iter()
        .map(|artifact| artifact.platform.as_str())
        .collect::<Vec<_>>();
    if actual_platforms != expected_platforms {
        return Err("verifier manifest platform records are missing, duplicated, or unsorted".to_string());
    }
    for artifact in &manifest.artifacts {
        let expected_archive = contract_verifier_archive_name(&manifest.verifier_version, &artifact.platform)?;
        let expected_executable = contract_verifier_executable_name(&artifact.platform)?;
        if artifact.archive != expected_archive
            || artifact.executable != expected_executable
            || !is_prefixed_sha256(&artifact.archive_sha256)
            || !is_prefixed_sha256(&artifact.executable_sha256)
        {
            return Err(format!(
                "verifier manifest artifact for `{}` has an invalid name or digest",
                artifact.platform
            ));
        }
    }
    Ok(())
}

fn verify_contract_verifier_archive(
    archive_path: &Path,
    artifact: &ContractVerifierDistributionArtifact,
) -> Result<(), String> {
    let archive = fs::read(archive_path)
        .map_err(|e| format!("read archive `{}`: {e}", archive_path.display()))?;
    let executable = if artifact.platform == "windows-x86_64" {
        let mut zip = zip::ZipArchive::new(Cursor::new(archive))
            .map_err(|e| format!("parse zip archive `{}`: {e}", archive_path.display()))?;
        if zip.len() != 1 {
            return Err(format!("archive `{}` must contain exactly one file", artifact.archive));
        }
        let mut entry = zip
            .by_index(0)
            .map_err(|e| format!("read zip entry `{}`: {e}", artifact.archive))?;
        if entry.name() != artifact.executable || entry.is_dir() {
            return Err(format!("archive `{}` has an unexpected executable entry", artifact.archive));
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|e| format!("read zip executable `{}`: {e}", artifact.archive))?;
        bytes
    } else {
        extract_contract_verifier_tar_gz(&archive, &artifact.executable)?
    };
    if format!("sha256:{}", hex::encode(Sha256::digest(&executable))) != artifact.executable_sha256 {
        return Err(format!("archive `{}` executable digest mismatch", artifact.archive));
    }
    Ok(())
}

fn contract_verifier_archive_name(version: &str, platform: &str) -> Result<String, String> {
    let extension = CONTRACT_VERIFIER_PLATFORMS
        .iter()
        .find_map(|(supported, extension)| (*supported == platform).then_some(*extension))
        .ok_or_else(|| format!("unsupported verifier platform `{platform}`"))?;
    Ok(format!("clg-contract-verifier-v{version}-{platform}.{extension}"))
}

fn contract_verifier_executable_name(platform: &str) -> Result<&'static str, String> {
    match platform {
        "windows-x86_64" => Ok("clg-contract-verifier.exe"),
        "linux-x86_64" | "macos-x86_64" => Ok(CONTRACT_VERIFIER_EXECUTABLE),
        _ => Err(format!("unsupported verifier platform `{platform}`")),
    }
}

fn create_contract_verifier_zip(
    archive_path: &Path,
    executable: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let file = fs::File::create(archive_path)
        .map_err(|e| format!("create archive `{}`: {e}", archive_path.display()))?;
    let mut zip = ZipWriter::new(file);
    let timestamp =
        DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).expect("valid deterministic zip time");
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(timestamp)
        .unix_permissions(0o755);
    use std::io::Write;
    zip.start_file(executable, options)
        .map_err(|e| format!("start archive entry `{executable}`: {e}"))?;
    zip.write_all(bytes)
        .map_err(|e| format!("write archive entry `{executable}`: {e}"))?;
    zip.finish()
        .map_err(|e| format!("finalize archive `{}`: {e}", archive_path.display()))?;
    Ok(())
}

fn create_contract_verifier_tar_gz(
    archive_path: &Path,
    executable: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let mut tar = Vec::new();
    append_contract_verifier_tar_file(&mut tar, executable, bytes)?;
    tar.extend_from_slice(&[0u8; 1024]);
    fs::write(archive_path, gzip_stored(&tar))
        .map_err(|e| format!("write archive `{}`: {e}", archive_path.display()))?;
    Ok(())
}

fn append_contract_verifier_tar_file(tar: &mut Vec<u8>, name: &str, bytes: &[u8]) -> Result<(), String> {
    if name.len() > 100 {
        return Err("verifier executable name exceeds USTAR name field".to_string());
    }
    let mut header = [0u8; 512];
    header[..name.len()].copy_from_slice(name.as_bytes());
    write_tar_octal(&mut header[100..108], 0o755)?;
    write_tar_octal(&mut header[108..116], 0)?;
    write_tar_octal(&mut header[116..124], 0)?;
    write_tar_octal(&mut header[124..136], bytes.len() as u64)?;
    write_tar_octal(&mut header[136..148], 0)?;
    header[148..156].fill(b' ');
    header[156] = b'0';
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");
    let checksum = header.iter().map(|byte| u32::from(*byte)).sum::<u32>();
    let rendered = format!("{checksum:06o}\0 ");
    header[148..156].copy_from_slice(rendered.as_bytes());
    tar.extend_from_slice(&header);
    tar.extend_from_slice(bytes);
    let padding = (512 - bytes.len() % 512) % 512;
    tar.resize(tar.len() + padding, 0);
    Ok(())
}

fn write_tar_octal(field: &mut [u8], value: u64) -> Result<(), String> {
    let content_len = field.len() - 1;
    let rendered = format!("{:0width$o}", value, width = content_len);
    if rendered.len() != content_len {
        return Err("tar numeric field overflow".to_string());
    }
    field[..content_len].copy_from_slice(rendered.as_bytes());
    field[content_len] = 0;
    Ok(())
}

fn gzip_stored(input: &[u8]) -> Vec<u8> {
    let mut output = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 255];
    if input.is_empty() {
        output.extend_from_slice(&[1, 0, 0, 255, 255]);
    } else {
        for (index, chunk) in input.chunks(65_535).enumerate() {
            let final_block = index + 1 == input.chunks(65_535).len();
            output.push(if final_block { 1 } else { 0 });
            let length = chunk.len() as u16;
            output.extend_from_slice(&length.to_le_bytes());
            output.extend_from_slice(&(!length).to_le_bytes());
            output.extend_from_slice(chunk);
        }
    }
    output.extend_from_slice(&crc32(input).to_le_bytes());
    output.extend_from_slice(&(input.len() as u32).to_le_bytes());
    output
}

fn extract_contract_verifier_tar_gz(archive: &[u8], expected_name: &str) -> Result<Vec<u8>, String> {
    let tar = gunzip_stored(archive)?;
    let mut offset = 0usize;
    let mut executable = None;
    while offset + 512 <= tar.len() {
        let header = &tar[offset..offset + 512];
        if header.iter().all(|byte| *byte == 0) {
            if tar[offset..].iter().all(|byte| *byte == 0) {
                break;
            }
            return Err("tar archive has an invalid zero block".to_string());
        }
        let name_end = header[..100].iter().position(|byte| *byte == 0).unwrap_or(100);
        let name = std::str::from_utf8(&header[..name_end])
            .map_err(|_| "tar archive entry name is not UTF-8".to_string())?;
        let size = parse_tar_octal(&header[124..136])? as usize;
        let data_start = offset + 512;
        let data_end = data_start
            .checked_add(size)
            .ok_or_else(|| "tar archive entry size overflows".to_string())?;
        if data_end > tar.len() || header[156] != b'0' {
            return Err("tar archive has an invalid regular file entry".to_string());
        }
        if name != expected_name || executable.is_some() {
            return Err("tar archive has an unexpected executable entry".to_string());
        }
        executable = Some(tar[data_start..data_end].to_vec());
        offset = data_end + (512 - size % 512) % 512;
    }
    executable.ok_or_else(|| "tar archive is missing its verifier executable".to_string())
}

fn parse_tar_octal(field: &[u8]) -> Result<u64, String> {
    let end = field.iter().position(|byte| *byte == 0).unwrap_or(field.len());
    let rendered = std::str::from_utf8(&field[..end])
        .map_err(|_| "tar numeric field is not UTF-8".to_string())?
        .trim();
    u64::from_str_radix(rendered, 8).map_err(|_| "tar numeric field is invalid".to_string())
}

fn gunzip_stored(archive: &[u8]) -> Result<Vec<u8>, String> {
    if archive.len() < 18 || archive[..10] != [0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 255] {
        return Err("archive is not the deterministic stored gzip form".to_string());
    }
    let trailer_start = archive.len() - 8;
    let mut offset = 10usize;
    let mut output = Vec::new();
    loop {
        if offset + 5 > trailer_start {
            return Err("stored gzip data is truncated".to_string());
        }
        let block = archive[offset];
        offset += 1;
        if block & !1 != 0 {
            return Err("gzip archive uses unsupported compression".to_string());
        }
        let length = u16::from_le_bytes([archive[offset], archive[offset + 1]]) as usize;
        let inverse = u16::from_le_bytes([archive[offset + 2], archive[offset + 3]]);
        offset += 4;
        if inverse != !(length as u16) || offset + length > trailer_start {
            return Err("stored gzip block length is invalid".to_string());
        }
        output.extend_from_slice(&archive[offset..offset + length]);
        offset += length;
        if block == 1 {
            break;
        }
    }
    if offset != trailer_start {
        return Err("stored gzip archive has trailing compressed data".to_string());
    }
    let expected_crc = u32::from_le_bytes(archive[trailer_start..trailer_start + 4].try_into().expect("four-byte gzip crc"));
    let expected_size = u32::from_le_bytes(archive[trailer_start + 4..].try_into().expect("four-byte gzip size"));
    if crc32(&output) != expected_crc || output.len() as u32 != expected_size {
        return Err("stored gzip trailer does not match decompressed bytes".to_string());
    }
    Ok(output)
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn canonical_distribution_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut value = serde_json::to_value(value)
        .map_err(|e| format!("convert JSON value for canonical encoding: {e}"))?;
    canonicalize_distribution_json(&mut value);
    serde_json::to_vec(&value).map_err(|e| format!("serialize canonical JSON: {e}"))
}

fn canonicalize_distribution_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                canonicalize_distribution_json(item);
            }
        }
        serde_json::Value::Object(entries) => {
            let mut sorted = BTreeMap::new();
            for (key, mut nested) in std::mem::take(entries) {
                canonicalize_distribution_json(&mut nested);
                sorted.insert(key, nested);
            }
            *entries = sorted.into_iter().collect();
        }
        _ => {}
    }
}

fn load_contract_verifier_signing_key(path: &Path) -> Result<SigningKey, String> {
    let bytes = fs::read(path).map_err(|e| format!("read signing key `{}`: {e}", path.display()))?;
    let key: ContractVerifierSigningKeyFile = serde_json::from_slice(&bytes)
        .map_err(|e| format!("parse signing key `{}`: {e}", path.display()))?;
    if key.scheme != "ed25519" {
        return Err("verifier distribution signing key must use ed25519".to_string());
    }
    let key_bytes = decode_fixed_hex32(&key.private_key)
        .ok_or_else(|| "verifier distribution private key must be 32 lowercase hexadecimal bytes".to_string())?;
    Ok(SigningKey::from_bytes(&key_bytes))
}

fn load_contract_verifier_public_key(path: &Path) -> Result<VerifyingKey, String> {
    let bytes = fs::read(path).map_err(|e| format!("read public key `{}`: {e}", path.display()))?;
    let key: ContractVerifierPublicKeyFile = serde_json::from_slice(&bytes)
        .map_err(|e| format!("parse public key `{}`: {e}", path.display()))?;
    if key.scheme != "ed25519" {
        return Err("verifier distribution public key must use ed25519".to_string());
    }
    let key_bytes = decode_fixed_hex32(&key.public_key)
        .ok_or_else(|| "verifier distribution public key must be 32 lowercase hexadecimal bytes".to_string())?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| format!("parse verifier distribution public key: {e}"))
}

fn is_prefixed_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|digest| digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_digit() || (byte.is_ascii_lowercase() && byte.is_ascii_hexdigit())))
}

fn is_exact_semver(value: &str) -> bool {
    let pieces = value.split('.').collect::<Vec<_>>();
    pieces.len() == 3
        && pieces.iter().all(|piece| {
            !piece.is_empty()
                && piece.bytes().all(|byte| byte.is_ascii_digit())
                && (piece == &"0" || !piece.starts_with('0'))
        })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Milestone3InstallerArchive {
    platform: String,
    asset_name: String,
    url: String,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Milestone3HomebrewChannel {
    formula_path: String,
    linux_url: String,
    linux_sha256: String,
    macos_url: String,
    macos_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Milestone3WingetChannel {
    package_identifier: String,
    manifest_dir: String,
    installer_url: String,
    installer_sha256: String,
    nested_installer_relative_path: String,
}

fn emit_milestone3_binary_bundle(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_milestone3_binary_bundle_args(raw_args)?;
    let out_dir = opts.out_dir.clone().unwrap_or_else(|| {
        root.join("tmp")
            .join("milestone3-binary")
            .join(opts.platform.as_str())
    });
    if out_dir.exists() {
        fs::remove_dir_all(&out_dir).map_err(|e| {
            format!(
                "cleanup milestone3 binary bundle dir `{}`: {e}",
                out_dir.display()
            )
        })?;
    }
    fs::create_dir_all(&out_dir).map_err(|e| {
        format!(
            "create milestone3 binary bundle dir `{}`: {e}",
            out_dir.display()
        )
    })?;

    let binary_source =
        resolve_milestone3_binary_source(root, opts.binary.as_ref(), opts.platform.as_str())?;
    let binary_name = binary_source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("clg")
        .to_string();
    let bin_dir = out_dir.join("bin").join(opts.platform.as_str());
    let checksums_dir = out_dir.join("checksums");
    let metadata_dir = out_dir.join("metadata");
    let sbom_dir = out_dir.join("sbom");
    let licenses_dir = out_dir.join("licenses");
    for dir in [
        &bin_dir,
        &checksums_dir,
        &metadata_dir,
        &sbom_dir,
        &licenses_dir,
    ] {
        fs::create_dir_all(dir).map_err(|e| format!("create `{}`: {e}", dir.display()))?;
    }

    let binary_dest = bin_dir.join(&binary_name);
    fs::copy(&binary_source, &binary_dest).map_err(|e| {
        format!(
            "copy binary from `{}` to `{}`: {e}",
            binary_source.display(),
            binary_dest.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_dest)
            .map_err(|e| format!("read `{}` metadata: {e}", binary_dest.display()))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_dest, perms)
            .map_err(|e| format!("set `{}` executable bit: {e}", binary_dest.display()))?;
    }

    run_milestone2_supply_chain_gate(
        root,
        vec![
            "--out-dir".to_string(),
            sbom_dir.to_string_lossy().to_string(),
        ],
    )?;

    let root_license = root.join("LICENSE");
    if !root_license.is_file() {
        return Err(format!(
            "missing repository LICENSE at `{}`",
            root_license.display()
        ));
    }
    let project_license_path = licenses_dir.join("clearlang-LICENSE.txt");
    fs::copy(&root_license, &project_license_path).map_err(|e| {
        format!(
            "copy repository LICENSE from `{}` to `{}`: {e}",
            root_license.display(),
            project_license_path.display()
        )
    })?;

    let third_party_summary = build_third_party_license_summary(sbom_dir.as_path())?;
    let third_party_summary_path = licenses_dir.join("third-party-licenses.json");
    write_json_pretty(&third_party_summary_path, &third_party_summary)?;

    let release_notes_src = root.join("release_notes").join("milestone_3.md");
    if release_notes_src.is_file() {
        let release_notes_dir = out_dir.join("release_notes");
        fs::create_dir_all(&release_notes_dir)
            .map_err(|e| format!("create `{}`: {e}", release_notes_dir.display()))?;
        fs::copy(&release_notes_src, release_notes_dir.join("milestone_3.md")).map_err(|e| {
            format!(
                "copy release notes from `{}`: {e}",
                release_notes_src.display()
            )
        })?;
    }

    let mut checksum_entries = Vec::<(String, String)>::new();
    let mut collect_entry = |path: &Path| -> Result<(), String> {
        let rel = normalize_rel_path(&out_dir, path);
        let sha = file_sha256_hex(path)?;
        checksum_entries.push((rel, sha));
        Ok(())
    };
    collect_entry(&binary_dest)?;
    collect_entry(&project_license_path)?;
    collect_entry(&third_party_summary_path)?;
    let sbom_files = list_json_files(sbom_dir.as_path())?;
    for sbom_file in &sbom_files {
        collect_entry(sbom_file.as_path())?;
    }

    checksum_entries.sort_by(|a, b| a.0.cmp(&b.0));
    let checksums_path = checksums_dir.join("SHA256SUMS");
    let checksums_text = checksum_entries
        .iter()
        .map(|(path, sha)| format!("{sha}  {path}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&checksums_path, format!("{checksums_text}\n"))
        .map_err(|e| format!("write `{}`: {e}", checksums_path.display()))?;

    let binary_artifact = manifest_artifact(&out_dir, binary_dest.as_path())?;
    let checksums_artifact = manifest_artifact(&out_dir, checksums_path.as_path())?;
    let sbom_artifacts = sbom_files
        .iter()
        .map(|path| manifest_artifact(&out_dir, path.as_path()))
        .collect::<Result<Vec<_>, _>>()?;
    let license_artifacts = vec![
        manifest_artifact(&out_dir, project_license_path.as_path())?,
        manifest_artifact(&out_dir, third_party_summary_path.as_path())?,
    ];
    let payload = Milestone3BundlePayload {
        schema_version: 1,
        kind: "clearlang.milestone3_binary_bundle".to_string(),
        platform: opts.platform.clone(),
        key_id: opts.key_id.clone(),
        binary: binary_artifact,
        checksums_manifest: checksums_artifact,
        sbom_artifacts,
        license_artifacts,
    };

    let signing_key = load_binary_release_signing_key()?;
    let payload_canonical =
        serde_json::to_string(&payload).map_err(|e| format!("serialize bundle payload: {e}"))?;
    let payload_hash = hex::encode(Sha256::digest(payload_canonical.as_bytes()));
    let signature = hex::encode(signing_key.sign(payload_canonical.as_bytes()).to_bytes());
    let signed = Milestone3SignedBundleMetadata {
        schema_version: 1,
        payload,
        signature: Milestone3BundleSignature {
            key_id: opts.key_id,
            signature_format: "ed25519".to_string(),
            payload_hash,
            signature,
        },
    };
    let signed_path = metadata_dir.join("milestone3-binary-bundle.signed.json");
    write_json_pretty(&signed_path, &signed)?;

    let pubkey_path = metadata_dir.join("milestone3-binary-bundle.pubkey.json");
    let pubkey_payload = serde_json::json!({
        "schema_version": 1,
        "key_id": signed.signature.key_id,
        "scheme": "ed25519",
        "public_key": hex::encode(signing_key.verifying_key().to_bytes()),
    });
    write_json_pretty(&pubkey_path, &pubkey_payload)?;

    println!(
        "milestone3 binary bundle emitted: {} (platform={}, binary={})",
        out_dir.display(),
        opts.platform,
        binary_name
    );
    Ok(())
}

fn verify_milestone3_binary_bundle(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_milestone3_binary_bundle_verify_args(raw_args)?;
    let bundle_dir = if opts.bundle_dir.is_absolute() {
        opts.bundle_dir
    } else {
        root.join(opts.bundle_dir)
    };
    let metadata_dir = bundle_dir.join("metadata");
    let signed_path = metadata_dir.join("milestone3-binary-bundle.signed.json");
    let pubkey_path = metadata_dir.join("milestone3-binary-bundle.pubkey.json");
    let checksums_path = bundle_dir.join("checksums").join("SHA256SUMS");

    let signed: Milestone3SignedBundleMetadata = serde_json::from_slice(
        &fs::read(&signed_path).map_err(|e| format!("read `{}`: {e}", signed_path.display()))?,
    )
    .map_err(|e| format!("parse `{}`: {e}", signed_path.display()))?;
    if signed.schema_version != 1 {
        return Err(format!(
            "unsupported bundle metadata schema_version {} in `{}`",
            signed.schema_version,
            signed_path.display()
        ));
    }
    if signed.payload.schema_version != 1 {
        return Err(format!(
            "unsupported bundle payload schema_version {} in `{}`",
            signed.payload.schema_version,
            signed_path.display()
        ));
    }

    let pubkey: Milestone3BundlePubkeyMetadata = serde_json::from_slice(
        &fs::read(&pubkey_path).map_err(|e| format!("read `{}`: {e}", pubkey_path.display()))?,
    )
    .map_err(|e| format!("parse `{}`: {e}", pubkey_path.display()))?;
    if pubkey.schema_version != 1 {
        return Err(format!(
            "unsupported pubkey schema_version {} in `{}`",
            pubkey.schema_version,
            pubkey_path.display()
        ));
    }
    if pubkey.scheme != "ed25519" {
        return Err(format!(
            "unsupported pubkey scheme `{}` in `{}`",
            pubkey.scheme,
            pubkey_path.display()
        ));
    }
    if pubkey.key_id != signed.signature.key_id {
        return Err(format!(
            "pubkey key_id `{}` does not match signature key_id `{}`",
            pubkey.key_id, signed.signature.key_id
        ));
    }

    let payload_canonical = serde_json::to_string(&signed.payload)
        .map_err(|e| format!("serialize bundle payload for verification: {e}"))?;
    let payload_hash = hex::encode(Sha256::digest(payload_canonical.as_bytes()));
    if payload_hash != signed.signature.payload_hash {
        return Err(format!(
            "payload hash mismatch in `{}`: expected {}, got {}",
            signed_path.display(),
            signed.signature.payload_hash,
            payload_hash
        ));
    }

    let pubkey_bytes_vec =
        hex::decode(pubkey.public_key.as_str()).map_err(|e| format!("decode public_key hex: {e}"))?;
    let pubkey_bytes: [u8; 32] = pubkey_bytes_vec
        .as_slice()
        .try_into()
        .map_err(|_| "public_key must decode to 32 bytes".to_string())?;
    let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
        .map_err(|e| format!("parse public key bytes: {e}"))?;
    let signature_bytes_vec =
        hex::decode(signed.signature.signature.as_str()).map_err(|e| format!("decode signature hex: {e}"))?;
    let signature = Signature::try_from(signature_bytes_vec.as_slice())
        .map_err(|e| format!("parse signature bytes: {e}"))?;
    verifying_key
        .verify(payload_canonical.as_bytes(), &signature)
        .map_err(|e| format!("verify signature: {e}"))?;

    let checksums_raw = fs::read_to_string(&checksums_path)
        .map_err(|e| format!("read `{}`: {e}", checksums_path.display()))?;
    let lines: Vec<&str> = checksums_raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return Err(format!(
            "checksum manifest `{}` must contain at least one checksum line",
            checksums_path.display()
        ));
    }
    let mut entries = Vec::<(&str, &str)>::new();
    for line in &lines {
        let (expected_hash, rel_path) = line.split_once("  ").ok_or_else(|| {
            format!(
                "invalid checksum manifest line `{line}` in `{}`",
                checksums_path.display()
            )
        })?;
        entries.push((expected_hash, rel_path));
    }
    let mut sorted_paths = entries.iter().map(|(_, path)| *path).collect::<Vec<_>>();
    sorted_paths.sort();
    let listed_paths = entries.iter().map(|(_, path)| *path).collect::<Vec<_>>();
    if listed_paths != sorted_paths {
        return Err(format!(
            "checksum manifest `{}` paths are not lexicographically sorted",
            checksums_path.display()
        ));
    }
    for (expected_hash, rel_path) in entries {
        let absolute = bundle_dir.join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !absolute.is_file() {
            return Err(format!(
                "checksum manifest member `{}` missing from `{}`",
                rel_path,
                bundle_dir.display()
            ));
        }
        let actual_hash = file_sha256_hex(absolute.as_path())?;
        if actual_hash != expected_hash {
            return Err(format!(
                "checksum mismatch for `{}`: expected {}, got {}",
                rel_path, expected_hash, actual_hash
            ));
        }
    }

    println!(
        "milestone3 binary bundle verified: {}",
        bundle_dir.display()
    );
    Ok(())
}

fn emit_milestone3_installer_channels(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_milestone3_installer_channels_args(raw_args)?;
    let out_dir = opts.out_dir.clone().unwrap_or_else(|| {
        root.join("tmp")
            .join("milestone3-installer-channels")
            .join(opts.version.as_str())
    });
    if out_dir.exists() {
        fs::remove_dir_all(&out_dir)
            .map_err(|e| format!("cleanup installer channels dir `{}`: {e}", out_dir.display()))?;
    }
    fs::create_dir_all(&out_dir)
        .map_err(|e| format!("create installer channels dir `{}`: {e}", out_dir.display()))?;

    let bundle_specs = [
        ("windows", resolve_from_root(root, &opts.windows_bundle_dir)),
        ("linux", resolve_from_root(root, &opts.linux_bundle_dir)),
        ("macos", resolve_from_root(root, &opts.macos_bundle_dir)),
    ];

    let archives_dir = out_dir.join("archives");
    let homebrew_dir = out_dir.join("homebrew").join("Formula");
    let winget_dir = out_dir
        .join("winget")
        .join("manifests")
        .join("c")
        .join("ClearLang")
        .join("ClearLang")
        .join(opts.version.as_str());
    let metadata_dir = out_dir.join("metadata");
    for dir in [&archives_dir, &homebrew_dir, &winget_dir, &metadata_dir] {
        fs::create_dir_all(dir).map_err(|e| format!("create `{}`: {e}", dir.display()))?;
    }

    let mut archives = Vec::new();
    for (platform, bundle_dir) in bundle_specs {
        verify_milestone3_binary_bundle(
            root,
            vec![
                "--bundle-dir".to_string(),
                bundle_dir.to_string_lossy().to_string(),
            ],
        )?;
        let signed = read_milestone3_signed_bundle(bundle_dir.as_path())?;
        if signed.payload.platform != platform {
            return Err(format!(
                "bundle `{}` declares platform `{}` but `{platform}` was required",
                bundle_dir.display(),
                signed.payload.platform
            ));
        }
        let asset_name = format!("clg-milestone3-{platform}-x64-bundle.zip");
        let archive_path = archives_dir.join(asset_name.as_str());
        create_deterministic_zip(bundle_dir.as_path(), archive_path.as_path())?;
        let sha256 = file_sha256_hex(archive_path.as_path())?;
        archives.push(Milestone3InstallerArchive {
            platform: platform.to_string(),
            url: format!(
                "https://github.com/{}/releases/download/{}/{}",
                opts.repo, opts.release_tag, asset_name
            ),
            asset_name,
            sha256,
        });
    }
    archives.sort_by(|a, b| a.platform.cmp(&b.platform));

    let release_notes_url = format!(
        "https://github.com/{}/releases/tag/{}",
        opts.repo, opts.release_tag
    );
    let linux_archive = archive_for_platform(&archives, "linux")?.clone();
    let macos_archive = archive_for_platform(&archives, "macos")?.clone();
    let windows_archive = archive_for_platform(&archives, "windows")?.clone();

    let formula_path = homebrew_dir.join("clg.rb");
    fs::write(
        &formula_path,
        render_homebrew_formula(
            &opts.version,
            &release_notes_url,
            &linux_archive,
            &macos_archive,
        ),
    )
    .map_err(|e| format!("write `{}`: {e}", formula_path.display()))?;

    let winget_version_path = winget_dir.join("ClearLang.ClearLang.yaml");
    let winget_locale_path = winget_dir.join("ClearLang.ClearLang.locale.en-US.yaml");
    let winget_installer_path = winget_dir.join("ClearLang.ClearLang.installer.yaml");
    fs::write(
        &winget_version_path,
        render_winget_version_manifest(&opts.version),
    )
    .map_err(|e| format!("write `{}`: {e}", winget_version_path.display()))?;
    fs::write(
        &winget_locale_path,
        render_winget_locale_manifest(&opts.version, &release_notes_url),
    )
    .map_err(|e| format!("write `{}`: {e}", winget_locale_path.display()))?;
    fs::write(
        &winget_installer_path,
        render_winget_installer_manifest(&opts.version, &release_notes_url, &windows_archive),
    )
    .map_err(|e| format!("write `{}`: {e}", winget_installer_path.display()))?;

    let manifest = Milestone3InstallerChannelsManifest {
        schema_version: 1,
        version: opts.version.clone(),
        release_tag: opts.release_tag.clone(),
        repository: opts.repo.clone(),
        release_notes_url: release_notes_url.clone(),
        archives,
        homebrew: Milestone3HomebrewChannel {
            formula_path: normalize_rel_path(&out_dir, formula_path.as_path()),
            linux_url: linux_archive.url.clone(),
            linux_sha256: linux_archive.sha256.clone(),
            macos_url: macos_archive.url.clone(),
            macos_sha256: macos_archive.sha256.clone(),
        },
        winget: Milestone3WingetChannel {
            package_identifier: "ClearLang.ClearLang".to_string(),
            manifest_dir: normalize_rel_path(&out_dir, winget_dir.as_path()),
            installer_url: windows_archive.url.clone(),
            installer_sha256: windows_archive.sha256.clone(),
            nested_installer_relative_path: "bin/windows/clg.exe".to_string(),
        },
    };
    write_json_pretty(
        &metadata_dir.join("milestone3-installer-channels.json"),
        &manifest,
    )?;

    println!(
        "milestone3 installer channels emitted: {} (version={}, tag={})",
        out_dir.display(),
        opts.version,
        opts.release_tag
    );
    Ok(())
}

fn parse_binary_repro_witness_args(
    raw_args: Vec<String>,
) -> Result<BinaryReproWitnessOpts, String> {
    let mut out: Option<PathBuf> = None;
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--out" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--out`".to_string())?;
                out = Some(PathBuf::from(value));
            }
            other => {
                return Err(format!(
                    "unknown binary-repro-witness arg `{other}` (supported: --out)"
                ));
            }
        }
        idx += 1;
    }
    Ok(BinaryReproWitnessOpts { out })
}

fn parse_milestone3_binary_bundle_args(
    raw_args: Vec<String>,
) -> Result<Milestone3BinaryBundleOpts, String> {
    let mut platform = current_platform_id().to_string();
    let mut binary: Option<PathBuf> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut key_id = DEFAULT_BINARY_RELEASE_KEY_ID.to_string();
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--platform" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--platform`".to_string())?;
                platform = value.trim().to_ascii_lowercase();
            }
            "--binary" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--binary`".to_string())?;
                binary = Some(PathBuf::from(value));
            }
            "--out-dir" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--out-dir`".to_string())?;
                out_dir = Some(PathBuf::from(value));
            }
            "--key-id" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--key-id`".to_string())?;
                if value.trim().is_empty() {
                    return Err("`--key-id` cannot be empty".to_string());
                }
                key_id = value.trim().to_string();
            }
            other => {
                return Err(format!(
                    "unknown milestone3-binary-bundle arg `{other}` (supported: --platform, --binary, --out-dir, --key-id)"
                ));
            }
        }
        idx += 1;
    }
    if !matches!(platform.as_str(), "windows" | "linux" | "macos") {
        return Err(format!(
            "unsupported `--platform` value `{platform}` (expected windows|linux|macos)"
        ));
    }
    Ok(Milestone3BinaryBundleOpts {
        platform,
        binary,
        out_dir,
        key_id,
    })
}

fn parse_milestone3_binary_bundle_verify_args(
    raw_args: Vec<String>,
) -> Result<Milestone3BinaryBundleVerifyOpts, String> {
    let mut bundle_dir: Option<PathBuf> = None;
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--bundle-dir" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--bundle-dir`".to_string())?;
                bundle_dir = Some(PathBuf::from(value));
            }
            other => {
                return Err(format!(
                    "unknown milestone3-binary-bundle-verify arg `{other}` (supported: --bundle-dir)"
                ));
            }
        }
        idx += 1;
    }
    let bundle_dir =
        bundle_dir.ok_or_else(|| "missing required `--bundle-dir`".to_string())?;
    Ok(Milestone3BinaryBundleVerifyOpts { bundle_dir })
}

fn parse_contract_verifier_package_args(
    raw_args: Vec<String>,
) -> Result<ContractVerifierPackageOpts, String> {
    let mut version = None;
    let mut artifacts = Vec::new();
    let mut signing_key = None;
    let mut key_id = None;
    let mut out_dir = None;
    let mut index = 0usize;
    while index < raw_args.len() {
        match raw_args[index].as_str() {
            "--version" => {
                index += 1;
                version = Some(
                    raw_args
                        .get(index)
                        .ok_or_else(|| "missing value for `--version`".to_string())?
                        .trim()
                        .to_string(),
                );
            }
            "--artifact" => {
                index += 1;
                let value = raw_args
                    .get(index)
                    .ok_or_else(|| "missing value for `--artifact`".to_string())?;
                let (platform, path) = value.split_once('=').ok_or_else(|| {
                    "`--artifact` must have PLATFORM=FILE form".to_string()
                })?;
                if path.trim().is_empty() {
                    return Err("`--artifact` file path cannot be empty".to_string());
                }
                artifacts.push((platform.to_string(), PathBuf::from(path)));
            }
            "--signing-key" => {
                index += 1;
                signing_key = Some(PathBuf::from(
                    raw_args
                        .get(index)
                        .ok_or_else(|| "missing value for `--signing-key`".to_string())?,
                ));
            }
            "--key-id" => {
                index += 1;
                key_id = Some(
                    raw_args
                        .get(index)
                        .ok_or_else(|| "missing value for `--key-id`".to_string())?
                        .trim()
                        .to_string(),
                );
            }
            "--out-dir" => {
                index += 1;
                out_dir = Some(PathBuf::from(
                    raw_args
                        .get(index)
                        .ok_or_else(|| "missing value for `--out-dir`".to_string())?,
                ));
            }
            other => return Err(format!("unknown contract-verifier-package arg `{other}` (supported: --version, --artifact, --signing-key, --key-id, --out-dir)")),
        }
        index += 1;
    }
    let version = version.ok_or_else(|| "missing required `--version`".to_string())?;
    if !is_exact_semver(&version) {
        return Err(format!("invalid verifier package version `{version}`"));
    }
    let key_id = key_id.ok_or_else(|| "missing required `--key-id`".to_string())?;
    if key_id.is_empty() {
        return Err("`--key-id` cannot be empty".to_string());
    }
    artifacts.sort_by(|left, right| left.0.cmp(&right.0));
    let expected_platforms = CONTRACT_VERIFIER_PLATFORMS
        .iter()
        .map(|(platform, _)| *platform)
        .collect::<Vec<_>>();
    let actual_platforms = artifacts
        .iter()
        .map(|(platform, _)| platform.as_str())
        .collect::<Vec<_>>();
    if actual_platforms != expected_platforms {
        return Err("`--artifact` must provide exactly linux-x86_64, macos-x86_64, and windows-x86_64 once each".to_string());
    }
    Ok(ContractVerifierPackageOpts {
        version,
        artifacts,
        signing_key: signing_key
            .ok_or_else(|| "missing required `--signing-key`".to_string())?,
        key_id,
        out_dir: out_dir.ok_or_else(|| "missing required `--out-dir`".to_string())?,
    })
}

fn parse_contract_verifier_package_verify_args(
    raw_args: Vec<String>,
) -> Result<ContractVerifierPackageVerifyOpts, String> {
    let mut package_dir = None;
    let mut pubkey = None;
    let mut index = 0usize;
    while index < raw_args.len() {
        match raw_args[index].as_str() {
            "--package-dir" => {
                index += 1;
                package_dir = Some(PathBuf::from(
                    raw_args
                        .get(index)
                        .ok_or_else(|| "missing value for `--package-dir`".to_string())?,
                ));
            }
            "--pubkey" => {
                index += 1;
                pubkey = Some(PathBuf::from(
                    raw_args
                        .get(index)
                        .ok_or_else(|| "missing value for `--pubkey`".to_string())?,
                ));
            }
            other => return Err(format!("unknown contract-verifier-package-verify arg `{other}` (supported: --package-dir, --pubkey)")),
        }
        index += 1;
    }
    Ok(ContractVerifierPackageVerifyOpts {
        package_dir: package_dir
            .ok_or_else(|| "missing required `--package-dir`".to_string())?,
        pubkey: pubkey.ok_or_else(|| "missing required `--pubkey`".to_string())?,
    })
}

fn parse_milestone3_installer_channels_args(
    raw_args: Vec<String>,
) -> Result<Milestone3InstallerChannelsOpts, String> {
    let mut version: Option<String> = None;
    let mut release_tag: Option<String> = None;
    let mut repo = "clearlang/clearlang".to_string();
    let mut windows_bundle_dir: Option<PathBuf> = None;
    let mut linux_bundle_dir: Option<PathBuf> = None;
    let mut macos_bundle_dir: Option<PathBuf> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--version" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--version`".to_string())?;
                validate_semver(value)?;
                version = Some(value.trim().to_string());
            }
            "--release-tag" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--release-tag`".to_string())?;
                if value.trim().is_empty() {
                    return Err("`--release-tag` cannot be empty".to_string());
                }
                release_tag = Some(value.trim().to_string());
            }
            "--repo" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--repo`".to_string())?;
                let normalized = value.trim();
                if normalized.is_empty() || normalized.split('/').count() != 2 {
                    return Err(format!(
                        "invalid `--repo` value `{normalized}`: expected `owner/name`"
                    ));
                }
                repo = normalized.to_string();
            }
            "--windows-bundle-dir" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--windows-bundle-dir`".to_string())?;
                windows_bundle_dir = Some(PathBuf::from(value));
            }
            "--linux-bundle-dir" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--linux-bundle-dir`".to_string())?;
                linux_bundle_dir = Some(PathBuf::from(value));
            }
            "--macos-bundle-dir" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--macos-bundle-dir`".to_string())?;
                macos_bundle_dir = Some(PathBuf::from(value));
            }
            "--out-dir" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--out-dir`".to_string())?;
                out_dir = Some(PathBuf::from(value));
            }
            other => {
                return Err(format!(
                    "unknown milestone3-installer-channels arg `{other}` (supported: --version, --release-tag, --repo, --windows-bundle-dir, --linux-bundle-dir, --macos-bundle-dir, --out-dir)"
                ));
            }
        }
        idx += 1;
    }
    Ok(Milestone3InstallerChannelsOpts {
        version: version.ok_or_else(|| "missing required `--version`".to_string())?,
        release_tag: release_tag.ok_or_else(|| "missing required `--release-tag`".to_string())?,
        repo,
        windows_bundle_dir: windows_bundle_dir
            .ok_or_else(|| "missing required `--windows-bundle-dir`".to_string())?,
        linux_bundle_dir: linux_bundle_dir
            .ok_or_else(|| "missing required `--linux-bundle-dir`".to_string())?,
        macos_bundle_dir: macos_bundle_dir
            .ok_or_else(|| "missing required `--macos-bundle-dir`".to_string())?,
        out_dir,
    })
}

fn clg_binary_path(target_dir: &Path) -> PathBuf {
    let file = if cfg!(windows) { "clg.exe" } else { "clg" };
    target_dir.join("release").join(file)
}

fn file_sha256_hex(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("read `{}`: {e}", path.display()))?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

fn current_platform_id() -> &'static str {
    match env::consts::OS {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "macos",
        _ => "unknown",
    }
}

fn read_milestone3_signed_bundle(
    bundle_dir: &Path,
) -> Result<Milestone3SignedBundleMetadata, String> {
    let signed_path = bundle_dir
        .join("metadata")
        .join("milestone3-binary-bundle.signed.json");
    serde_json::from_slice(
        &fs::read(&signed_path).map_err(|e| format!("read `{}`: {e}", signed_path.display()))?,
    )
    .map_err(|e| format!("parse `{}`: {e}", signed_path.display()))
}

fn resolve_from_root(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn archive_for_platform<'a>(
    archives: &'a [Milestone3InstallerArchive],
    platform: &str,
) -> Result<&'a Milestone3InstallerArchive, String> {
    archives
        .iter()
        .find(|archive| archive.platform == platform)
        .ok_or_else(|| format!("missing generated installer archive for `{platform}`"))
}

fn render_homebrew_formula(
    version: &str,
    release_notes_url: &str,
    linux_archive: &Milestone3InstallerArchive,
    macos_archive: &Milestone3InstallerArchive,
) -> String {
    format!(
        "class Clg < Formula\n  desc \"ClearLang command-line interface\"\n  homepage \"https://clearlang.net\"\n  url \"{macos_url}\"\n  version \"{version}\"\n  license \"MIT\"\n\n  on_macos do\n    sha256 \"{macos_sha}\"\n  end\n\n  on_linux do\n    url \"{linux_url}\"\n    sha256 \"{linux_sha}\"\n  end\n\n  def install\n    if OS.mac?\n      bin.install \"bin/macos/clg\" => \"clg\"\n    elsif OS.linux?\n      bin.install \"bin/linux/clg\" => \"clg\"\n    else\n      odie \"ClearLang milestone_3 supports Homebrew only on macOS and Linux\"\n    end\n  end\n\n  test do\n    system \"#{{bin}}/clg\", \"--help\"\n  end\nend\n\n# Release notes: {release_notes_url}\n",
        macos_url = macos_archive.url,
        macos_sha = macos_archive.sha256,
        linux_url = linux_archive.url,
        linux_sha = linux_archive.sha256,
    )
}

fn render_winget_version_manifest(version: &str) -> String {
    format!(
        "PackageIdentifier: ClearLang.ClearLang\nPackageVersion: {version}\nDefaultLocale: en-US\nManifestType: version\nManifestVersion: 1.6.0\n"
    )
}

fn render_winget_locale_manifest(version: &str, release_notes_url: &str) -> String {
    format!(
        "PackageIdentifier: ClearLang.ClearLang\nPackageVersion: {version}\nPackageLocale: en-US\nPublisher: ClearLang\nPublisherUrl: https://clearlang.net\nPublisherSupportUrl: https://github.com/clearlang/clearlang/issues\nPackageName: ClearLang CLI\nPackageUrl: https://clearlang.net\nShortDescription: ClearLang command-line interface\nLicense: MIT\nLicenseUrl: https://github.com/clearlang/clearlang/blob/main/LICENSE\nReleaseNotesUrl: {release_notes_url}\nManifestType: defaultLocale\nManifestVersion: 1.6.0\n"
    )
}

fn render_winget_installer_manifest(
    version: &str,
    release_notes_url: &str,
    windows_archive: &Milestone3InstallerArchive,
) -> String {
    format!(
        "PackageIdentifier: ClearLang.ClearLang\nPackageVersion: {version}\nInstallerLocale: en-US\nInstallerType: zip\nNestedInstallerType: portable\nNestedInstallerFiles:\n  - RelativeFilePath: bin/windows/clg.exe\n    PortableCommandAlias: clg\nReleaseNotesUrl: {release_notes_url}\nInstallers:\n  - Architecture: x64\n    InstallerUrl: {url}\n    InstallerSha256: {sha}\nManifestType: installer\nManifestVersion: 1.6.0\n",
        url = windows_archive.url,
        sha = windows_archive.sha256,
    )
}

fn create_deterministic_zip(source_dir: &Path, archive_path: &Path) -> Result<(), String> {
    let file = fs::File::create(archive_path)
        .map_err(|e| format!("create `{}`: {e}", archive_path.display()))?;
    let mut zip = ZipWriter::new(file);
    let timestamp =
        DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).expect("valid deterministic zip time");
    let files = collect_files_recursive_sorted(source_dir)?;
    for path in files {
        let rel = normalize_rel_path(source_dir, path.as_path());
        let permissions = if rel.starts_with("bin/") { 0o755 } else { 0o644 };
        let options = FileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(timestamp)
            .unix_permissions(permissions);
        zip.start_file(rel, options)
            .map_err(|e| format!("start zip entry `{}`: {e}", archive_path.display()))?;
        let bytes = fs::read(&path).map_err(|e| format!("read `{}`: {e}", path.display()))?;
        use std::io::Write;
        zip.write_all(&bytes)
            .map_err(|e| format!("write zip entry `{}`: {e}", archive_path.display()))?;
    }
    zip.finish()
        .map_err(|e| format!("finalize `{}`: {e}", archive_path.display()))?;
    Ok(())
}

fn collect_files_recursive_sorted(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_files_recursive(dir, &mut files)?;
    files.sort_by_key(|a| normalize_rel_path(dir, a.as_path()));
    Ok(files)
}

fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries = fs::read_dir(dir)
        .map_err(|e| format!("read directory `{}`: {e}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("enumerate directory `{}`: {e}", dir.display()))?;
    entries.sort_by_key(|a| a.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(path.as_path(), files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn resolve_milestone3_binary_source(
    root: &Path,
    binary: Option<&PathBuf>,
    platform: &str,
) -> Result<PathBuf, String> {
    let binary_path = match binary {
        Some(path) => path.clone(),
        None => {
            let file = if platform == "windows" {
                "clg.exe"
            } else {
                "clg"
            };
            root.join("target").join("release").join(file)
        }
    };
    if !binary_path.is_file() {
        return Err(format!(
            "missing binary `{}`; run `cargo build --release -p clg-cli` first or pass `--binary <FILE>`",
            binary_path.display()
        ));
    }
    Ok(binary_path)
}

fn manifest_artifact(root: &Path, path: &Path) -> Result<Milestone3BundleArtifact, String> {
    let sha256 = file_sha256_hex(path)?;
    let size_bytes = fs::metadata(path)
        .map_err(|e| format!("read `{}` metadata: {e}", path.display()))?
        .len();
    Ok(Milestone3BundleArtifact {
        path: normalize_rel_path(root, path),
        sha256,
        size_bytes,
    })
}

fn list_json_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("read dir `{}`: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read dir entry `{}`: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("json"))
                .unwrap_or(false)
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn build_third_party_license_summary(sbom_dir: &Path) -> Result<serde_json::Value, String> {
    let sbom_path = sbom_dir.join("milestone2-sbom.json");
    let sbom_bytes =
        fs::read(&sbom_path).map_err(|e| format!("read `{}`: {e}", sbom_path.display()))?;
    let sbom: serde_json::Value = serde_json::from_slice(&sbom_bytes)
        .map_err(|e| format!("parse `{}`: {e}", sbom_path.display()))?;
    let packages = sbom
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("SBOM `{}` missing `packages[]` array", sbom_path.display()))?;
    let mut entries = packages
        .iter()
        .map(|pkg| {
            serde_json::json!({
                "id": pkg.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "name": pkg.get("name").cloned().unwrap_or(serde_json::Value::Null),
                "version": pkg.get("version").cloned().unwrap_or(serde_json::Value::Null),
                "license": pkg.get("license").cloned().unwrap_or(serde_json::Value::Null),
                "license_file": pkg.get("license_file").cloned().unwrap_or(serde_json::Value::Null),
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        let ak = (
            a.get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
            a.get("version")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
            a.get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
        );
        let bk = (
            b.get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
            b.get("version")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
            b.get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
        );
        ak.cmp(&bk)
    });
    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "cargo_third_party_license_summary",
        "source_sbom": "sbom/milestone2-sbom.json",
        "entries": entries
    }))
}

fn parse_manifest_lock_drift_args(raw_args: Vec<String>) -> Result<ManifestLockDriftOpts, String> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--path" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--path`".to_string())?;
                paths.push(PathBuf::from(value));
            }
            other => {
                return Err(format!(
                    "unknown manifest-lock-drift-check arg `{other}` (supported: --path)"
                ));
            }
        }
        idx += 1;
    }
    if paths.is_empty() {
        paths.push(PathBuf::from(
            "docs/fixtures/phase-25.4/manifest-lock-consistency",
        ));
    }
    Ok(ManifestLockDriftOpts { paths })
}

fn check_manifest_lock_consistency_for_dir(dir: &Path) -> Result<(), String> {
    let manifest_path = dir.join("clg.project.json");
    let lock_path = dir.join("clg.lock.json");
    let graph_path = dir.join("clg.resolved-graph.json");
    let graph_hash_path = dir.join("clg.resolved-graph.sha256");
    if !manifest_path.is_file() {
        return Err(format!("missing manifest `{}`", manifest_path.display()));
    }
    if !lock_path.is_file() {
        return Err(format!("missing lockfile `{}`", lock_path.display()));
    }
    if !graph_path.is_file() {
        return Err(format!("missing resolved graph `{}`", graph_path.display()));
    }
    if !graph_hash_path.is_file() {
        return Err(format!(
            "missing resolved graph hash `{}`",
            graph_hash_path.display()
        ));
    }

    let manifest_raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("read manifest `{}`: {e}", manifest_path.display()))?;
    let lock_raw_bytes = fs::read(&lock_path)
        .map_err(|e| format!("read lockfile `{}`: {e}", lock_path.display()))?;
    let lock_raw = std::str::from_utf8(&lock_raw_bytes)
        .map_err(|e| format!("lockfile `{}` is not valid utf-8: {e}", lock_path.display()))?;
    let graph_raw_bytes = fs::read(&graph_path)
        .map_err(|e| format!("read resolved graph `{}`: {e}", graph_path.display()))?;
    let graph_raw = std::str::from_utf8(&graph_raw_bytes).map_err(|e| {
        format!(
            "resolved graph `{}` is not valid utf-8: {e}",
            graph_path.display()
        )
    })?;
    let graph_hash_raw = fs::read_to_string(&graph_hash_path).map_err(|e| {
        format!(
            "read resolved graph hash `{}`: {e}",
            graph_hash_path.display()
        )
    })?;

    let manifest: DriftManifestFile = serde_json::from_str(&manifest_raw)
        .map_err(|e| format!("parse manifest `{}`: {e}", manifest_path.display()))?;
    let lock: DriftLockFile = serde_json::from_str(lock_raw)
        .map_err(|e| format!("parse lockfile `{}`: {e}", lock_path.display()))?;
    let graph: DriftLockFile = serde_json::from_str(graph_raw)
        .map_err(|e| format!("parse resolved graph `{}`: {e}", graph_path.display()))?;

    if manifest.schema_version != 1 {
        return Err(format!(
            "manifest `{}` must use schema_version 1 for drift checks",
            manifest_path.display()
        ));
    }
    if lock.schema_version != 1 || lock.resolver_version != 1 {
        return Err(format!(
            "lockfile `{}` must use schema_version=1 and resolver_version=1",
            lock_path.display()
        ));
    }
    if graph.schema_version != 1 || graph.resolver_version != 1 {
        return Err(format!(
            "resolved graph `{}` must use schema_version=1 and resolver_version=1",
            graph_path.display()
        ));
    }

    let expected_dependencies = normalize_manifest_dependencies(&manifest.dependencies)?;
    let expected_roots = vec![DriftLockRoot {
        name: manifest.project.name.trim().to_string(),
        dependencies: expected_dependencies,
    }];
    let actual_roots = normalize_lock_roots(lock.roots)?;
    let lock_packages = normalize_lock_packages(lock.packages)?;
    let canonical_lock = DriftLockFile {
        schema_version: 1,
        resolver_version: 1,
        roots: actual_roots.clone(),
        packages: lock_packages.clone(),
    };
    let canonical_lock_bytes = pretty_json_bytes(&canonical_lock)?;
    if lock_raw_bytes != canonical_lock_bytes {
        return Err(format!(
            "lockfile `{}` must match canonical tool-owned format; regenerate with `clg pkg lock --update --root {}`",
            lock_path.display(),
            dir.display()
        ));
    }

    if actual_roots != expected_roots {
        let expected_json = serde_json::to_string(&expected_roots)
            .map_err(|e| format!("serialize expected roots: {e}"))?;
        let actual_json = serde_json::to_string(&actual_roots)
            .map_err(|e| format!("serialize lock roots: {e}"))?;
        return Err(format!(
            "manifest/lock inconsistency (`{}` vs `{}`): expected roots {}, found {}",
            manifest_path.display(),
            lock_path.display(),
            expected_json,
            actual_json
        ));
    }

    let graph_roots = normalize_lock_roots(graph.roots)?;
    let graph_packages = normalize_lock_packages(graph.packages)?;
    if graph_roots != actual_roots || graph_packages != lock_packages {
        return Err(format!(
            "lock/resolved-graph inconsistency (`{}` vs `{}`): roots/packages must match exactly",
            lock_path.display(),
            graph_path.display()
        ));
    }
    let graph_canonical_bytes = if graph_raw_bytes.last() == Some(&b'\n') {
        &graph_raw_bytes[..graph_raw_bytes.len() - 1]
    } else {
        graph_raw_bytes.as_slice()
    };
    let expected_graph_hash = hex::encode(Sha256::digest(graph_canonical_bytes));
    let actual_graph_hash = graph_hash_raw.trim();
    if actual_graph_hash != expected_graph_hash {
        return Err(format!(
            "resolved graph hash mismatch (`{}`): expected {}, found {}",
            graph_hash_path.display(),
            expected_graph_hash,
            actual_graph_hash
        ));
    }

    Ok(())
}

fn normalize_manifest_dependencies(
    dependencies: &[DriftManifestDependency],
) -> Result<Vec<DriftLockRootDependency>, String> {
    let mut out = Vec::with_capacity(dependencies.len());
    let mut seen = BTreeSet::new();
    for dep in dependencies {
        let name = dep.name.trim();
        let requirement = dep.requirement.trim();
        if name.is_empty() {
            return Err("manifest dependency name must be non-empty".to_string());
        }
        if requirement.is_empty() {
            return Err(format!(
                "manifest dependency `{name}` requirement must be non-empty"
            ));
        }
        if !seen.insert(name.to_string()) {
            return Err(format!("manifest has duplicate dependency `{name}`"));
        }
        out.push(DriftLockRootDependency {
            name: name.to_string(),
            requirement: requirement.to_string(),
        });
    }
    out.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.requirement.cmp(&b.requirement))
    });
    Ok(out)
}

fn normalize_lock_roots(roots: Vec<DriftLockRoot>) -> Result<Vec<DriftLockRoot>, String> {
    if roots.len() != 1 {
        return Err(format!(
            "lockfile roots must contain exactly one root for manifest v1 (found {})",
            roots.len()
        ));
    }
    let mut out = Vec::with_capacity(roots.len());
    let mut seen_root_names = BTreeSet::new();
    for mut root in roots {
        let root_name = root.name.trim();
        if root_name.is_empty() {
            return Err("lockfile root name must be non-empty".to_string());
        }
        if !seen_root_names.insert(root_name.to_string()) {
            return Err(format!("lockfile has duplicate root `{root_name}`"));
        }
        root.name = root_name.to_string();

        let mut seen_dep_names = BTreeSet::new();
        for dep in &mut root.dependencies {
            let name = dep.name.trim();
            let requirement = dep.requirement.trim();
            if name.is_empty() {
                return Err(format!(
                    "lockfile root `{}` has dependency with empty name",
                    root.name
                ));
            }
            if requirement.is_empty() {
                return Err(format!(
                    "lockfile root `{}` dependency `{}` has empty requirement",
                    root.name, name
                ));
            }
            if !seen_dep_names.insert(name.to_string()) {
                return Err(format!(
                    "lockfile root `{}` has duplicate dependency `{}`",
                    root.name, name
                ));
            }
            dep.name = name.to_string();
            dep.requirement = requirement.to_string();
        }
        root.dependencies.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.requirement.cmp(&b.requirement))
        });
        out.push(root);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn normalize_lock_packages(
    packages: Vec<DriftLockPackage>,
) -> Result<Vec<DriftLockPackage>, String> {
    let mut out = Vec::with_capacity(packages.len());
    let mut seen_ids = BTreeSet::new();
    for mut pkg in packages {
        let id = pkg.id.trim().to_string();
        let name = pkg.name.trim().to_string();
        let version = pkg.version.trim().to_string();
        let digest = pkg.digest.trim().to_string();
        let abi_id = pkg.abi_id.trim().to_string();
        if id.is_empty()
            || name.is_empty()
            || version.is_empty()
            || digest.is_empty()
            || abi_id.is_empty()
        {
            return Err("lockfile package identity fields must be non-empty".to_string());
        }
        if id != format!("{name}@{version}") {
            return Err(format!(
                "lockfile package id `{}` must match `{}@{}`",
                id, name, version
            ));
        }
        if !seen_ids.insert(id.clone()) {
            return Err(format!("lockfile has duplicate package id `{id}`"));
        }

        let mut seen_deps = BTreeSet::new();
        let mut normalized_deps = Vec::with_capacity(pkg.dependencies.len());
        for dep in pkg.dependencies {
            let dep_id = dep.trim().to_string();
            if dep_id.is_empty() {
                return Err(format!("lockfile package `{}` has empty dependency id", id));
            }
            if !seen_deps.insert(dep_id.clone()) {
                return Err(format!(
                    "lockfile package `{}` has duplicate dependency id `{}`",
                    id, dep_id
                ));
            }
            normalized_deps.push(dep_id);
        }
        normalized_deps.sort();

        pkg.id = id;
        pkg.name = name;
        pkg.version = version;
        pkg.digest = digest;
        pkg.abi_id = abi_id;
        pkg.dependencies = normalized_deps;
        out.push(pkg);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn run_clg_test_schema_gate(root: &Path) -> Result<(), String> {
    let project_root = root.join("examples").join("projects").join("testing");
    let output = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("clg-cli")
        .arg("--bin")
        .arg("clg")
        .arg("--")
        .arg("test")
        .arg(&project_root)
        .arg("--report")
        .arg("json")
        .current_dir(root)
        .output()
        .map_err(|e| format!("failed to run `clg test` schema gate: {e}"))?;

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("`clg test` schema gate stdout is not utf-8: {e}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "`clg test` schema gate failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            stdout,
            stderr
        ));
    }

    validate_clg_test_report_schema(stdout.trim())
}

fn validate_clg_test_report_schema(raw: &str) -> Result<(), String> {
    if raw.is_empty() {
        return Err("`clg test` schema gate emitted empty stdout".to_string());
    }
    let payload: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("parsing `clg test` report json: {e}"))?;
    let obj = payload
        .as_object()
        .ok_or_else(|| "`clg test` report must be a JSON object".to_string())?;

    let schema_version = obj
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "`clg test` report missing numeric `schema_version`".to_string())?;
    if schema_version != 1 {
        return Err(format!(
            "`clg test` report schema_version must be 1, found {schema_version}"
        ));
    }

    let report = obj
        .get("report")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "`clg test` report missing string `report`".to_string())?;
    if report != "json" {
        return Err(format!(
            "`clg test` report kind must be `json`, found `{report}`"
        ));
    }

    let status = obj
        .get("status")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "`clg test` report missing string `status`".to_string())?;
    if status != "ok" {
        return Err(format!(
            "`clg test` schema gate expects `status=ok` for examples/projects/testing, found `{status}`"
        ));
    }

    let discovered = obj
        .get("discovered")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "`clg test` report missing numeric `discovered`".to_string())?;
    let selected = obj
        .get("selected")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "`clg test` report missing numeric `selected`".to_string())?;
    let executed = obj
        .get("executed")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "`clg test` report missing numeric `executed`".to_string())?;
    let passed = obj
        .get("passed")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "`clg test` report missing numeric `passed`".to_string())?;
    let failed = obj
        .get("failed")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "`clg test` report missing numeric `failed`".to_string())?;

    if discovered == 0 {
        return Err("`clg test` schema gate requires at least one discovered test".to_string());
    }
    if selected == 0 || executed == 0 {
        return Err(
            "`clg test` schema gate requires selected/executed counts to be non-zero".to_string(),
        );
    }
    if selected != executed {
        return Err(format!(
            "`clg test` schema gate requires selected ({selected}) == executed ({executed})"
        ));
    }
    if failed != 0 {
        return Err(format!(
            "`clg test` schema gate requires zero failed tests, found {failed}"
        ));
    }
    if passed != executed {
        return Err(format!(
            "`clg test` schema gate requires passed ({passed}) == executed ({executed})"
        ));
    }

    let tests = obj
        .get("tests")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "`clg test` report missing array `tests`".to_string())?;
    if tests.len() as u64 != executed {
        return Err(format!(
            "`clg test` schema gate requires tests.len ({}) == executed ({executed})",
            tests.len()
        ));
    }

    let mut prev_id: Option<String> = None;
    let mut mocked_cases = 0usize;
    let mut non_mocked_cases = 0usize;
    for case in tests {
        let case_obj = case
            .as_object()
            .ok_or_else(|| "each `tests[]` entry must be an object".to_string())?;
        let id = case_obj
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "each `tests[]` entry must include string `id`".to_string())?;
        let _file = case_obj
            .get("file")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("test `{id}` missing string `file`"))?;
        let _function = case_obj
            .get("function")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("test `{id}` missing string `function`"))?;
        let _timeout_ms = case_obj
            .get("timeout_ms")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("test `{id}` missing numeric `timeout_ms`"))?;
        let mock_sets = case_obj
            .get("mock_sets")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("test `{id}` missing array `mock_sets`"))?;
        if mock_sets.is_empty() {
            non_mocked_cases += 1;
        } else {
            mocked_cases += 1;
        }
        let status = case_obj
            .get("status")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("test `{id}` missing string `status`"))?;
        if status != "passed" {
            return Err(format!(
                "`clg test` schema gate expected test `{id}` status `passed`, found `{status}`"
            ));
        }
        let _captured_stdout = case_obj
            .get("captured_stdout")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("test `{id}` missing string `captured_stdout`"))?;
        let _captured_stderr = case_obj
            .get("captured_stderr")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("test `{id}` missing string `captured_stderr`"))?;
        let replay_argv = case_obj
            .get("replay")
            .and_then(serde_json::Value::as_object)
            .and_then(|replay| replay.get("argv"))
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("test `{id}` missing replay.argv[]"))?;
        if replay_argv.len() < 3 {
            return Err(format!(
                "test `{id}` replay.argv[] must include at least command tokens"
            ));
        }
        if replay_argv.first().and_then(serde_json::Value::as_str) != Some("clg")
            || replay_argv.get(1).and_then(serde_json::Value::as_str) != Some("test")
        {
            return Err(format!(
                "test `{id}` replay.argv[] must start with [`clg`, `test`]"
            ));
        }

        if let Some(prev) = prev_id.as_ref() {
            if prev.as_str() > id {
                return Err(format!(
                    "`clg test` report test ids must be sorted deterministically (`{prev}` > `{id}`)"
                ));
            }
        }
        prev_id = Some(id.to_string());
    }
    if mocked_cases == 0 || non_mocked_cases == 0 {
        return Err(format!(
            "`clg test` schema gate requires balanced critical-path coverage with mocked and non-mocked cases (mocked={mocked_cases}, non_mocked={non_mocked_cases})"
        ));
    }

    Ok(())
}
