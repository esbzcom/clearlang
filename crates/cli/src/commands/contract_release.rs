//! One-command, fail-closed release preparation for the supported contract target profile.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{build, contract_test};
use crate::commands::helpers::{canonical_json_bytes, sha256_hex};
use crate::logging::{Logger, StageTimings};
use crate::signing::{self, SignScope};

const BUNDLE_FORMAT: &str = "clg.contract-release-bundle.v1";
const CAMPAIGN_EVIDENCE_FORMAT: &str = "clg.contract-campaign-evidence-index.v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    format: String,
    schema_version: u32,
    key_id: String,
    target_profile: String,
    target_execution_profile: String,
    artifacts: BundleArtifacts,
    orchestration: Vec<Stage>,
    source: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleArtifacts {
    contract_abi: Artifact,
    contract_state_schema: Artifact,
    contract_test_evidence: Artifact,
    contract_test_report: Artifact,
    evm_artifact: Artifact,
    evm_wire_abi: Artifact,
    proof_artifact: Artifact,
    signature: Artifact,
    signed_assurance_manifest: Artifact,
    vcs: Artifact,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stage {
    stage: String,
    status: String,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct CampaignEvidenceIndex {
    format: String,
    schema_version: u32,
    report_sha256: String,
    trace_directory: String,
    cases: Vec<CampaignEvidenceCase>,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct CampaignEvidenceCase {
    case: u32,
    args: Value,
    replay: Value,
    files: Vec<Artifact>,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    source: PathBuf,
    plan: PathBuf,
    key: PathBuf,
    pubkey: PathBuf,
    key_id: String,
    out_dir: PathBuf,
    prior_state_schema: Option<PathBuf>,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    if key_id.trim().is_empty() {
        return Err(anyhow!("contract release requires a non-empty --key-id"));
    }
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("contract release source must have a file stem"))?;
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("create contract release directory `{}`", out_dir.display()))?;
    let paths = ReleasePaths {
        artifact: out_dir.join(format!("{stem}.evm.json")),
        schema: out_dir.join(format!("{stem}.schema.json")),
        abi: out_dir.join(format!("{stem}.abi.json")),
        wire_abi: out_dir.join(format!("{stem}.evm-wire-abi.json")),
        vcs: out_dir.join(format!("{stem}.vcs.json")),
        proof: out_dir.join(format!("{stem}.proof.json")),
        signature: out_dir.join(format!("{stem}.sig.json")),
        assurance: out_dir.join(format!("{stem}.assurance.json")),
        campaign: out_dir.join(format!("{stem}.campaign.json")),
        campaign_evidence: out_dir.join(format!("{stem}.campaign-evidence.json")),
        traces: out_dir.join(format!("{stem}.campaign-traces")),
        bundle: out_dir.join(format!("{stem}.contract-release-bundle.json")),
    };
    let mut timings = StageTimings::new();
    {
        let _stage = timings.start(logger, "contract_release_build_prove_sign");
        build::run(
            source.clone(),
            paths.artifact.clone(),
            false,
            false,
            false,
            Some(paths.schema.clone()),
            Some(paths.abi.clone()),
            Some(paths.wire_abi.clone()),
            Some(paths.artifact.clone()),
            true,
            prior_state_schema,
            Some(paths.vcs.clone()),
            Some(paths.proof.clone()),
            build::CompilerMode::Strict,
            build::ReleaseProfile::Production,
            build::StdCoreLinkMode::Intrinsic,
            None,
            true,
            Some(key),
            Some(key_id.clone()),
            SignScope::Both,
            Some(paths.signature.clone()),
            Some(paths.assurance.clone()),
            None,
            None,
            json_errors,
            logger,
        )?;
    }
    {
        let _stage = timings.start(logger, "contract_release_verify_signature");
        signing::verify_signature_details(&paths.artifact, &paths.signature, &pubkey)
            .map_err(|err| anyhow!("verify contract release signature: {err}"))?;
        signing::verify_assurance_manifest(&paths.assurance, &pubkey)
            .map_err(|err| anyhow!("verify contract release assurance manifest: {err}"))?;
    }
    {
        let _stage = timings.start(logger, "contract_release_campaign");
        contract_test::run(
            source.clone(),
            plan,
            paths.campaign.clone(),
            paths.traces.clone(),
            logger,
        )?;
    }
    {
        let _stage = timings.start(logger, "contract_release_campaign_evidence");
        write_campaign_evidence_index(&paths.campaign, &paths.traces, &paths.campaign_evidence)?;
    }
    let bundle = {
        let _stage = timings.start(logger, "contract_release_bundle");
        let manifest = json!({
            "artifacts": {
                "contract_abi": artifact_file(&paths.abi)?,
                "contract_state_schema": artifact_file(&paths.schema)?,
                "contract_test_evidence": artifact_file(&paths.campaign_evidence)?,
                "contract_test_report": artifact_file(&paths.campaign)?,
                "evm_artifact": artifact_file(&paths.artifact)?,
                "evm_wire_abi": artifact_file(&paths.wire_abi)?,
                "proof_artifact": artifact_file(&paths.proof)?,
                "signature": artifact_file(&paths.signature)?,
                "signed_assurance_manifest": artifact_file(&paths.assurance)?,
                "vcs": artifact_file(&paths.vcs)?,
            },
            "format": BUNDLE_FORMAT,
            "key_id": key_id,
            "orchestration": [
                { "stage": "build_prove_sign", "status": "ok" },
                { "stage": "verify_signature", "status": "ok" },
                { "stage": "contract_test_simulate", "status": "ok" },
                { "stage": "package", "status": "ok" }
            ],
            "schema_version": 1,
            "source": source.display().to_string(),
            "target_execution_profile": "clg.evm-stateful-scalar.v1",
            "target_profile": "clg.evm-compatible.v1",
        });
        fs::write(&paths.bundle, canonical_json_bytes(&manifest)).with_context(|| {
            format!("write contract release bundle `{}`", paths.bundle.display())
        })?;
        manifest
    };
    logger.summary(&timings);
    println!(
        "{}",
        serde_json::to_string_pretty(&bundle).expect("serialize contract release bundle")
    );
    Ok(())
}

pub fn verify(
    bundle_path: PathBuf,
    pubkey: PathBuf,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    let failure = |message: String| -> Result<()> {
        if json_errors {
            return Err(crate::commands::helpers::CommandError::json(
                crate::commands::helpers::make_single_json_error(
                    "C140",
                    "verify",
                    message,
                    &bundle_path,
                    0,
                    0,
                    None,
                ),
            )
            .into());
        }
        Err(anyhow!(message))
    };
    let bytes = fs::read(&bundle_path)
        .map_err(|err| anyhow!(err).context("read contract release bundle"))?;
    let bundle: Bundle = serde_json::from_slice(&bytes)
        .map_err(|err| anyhow!(err).context("parse contract release bundle"))?;
    if bundle.format != BUNDLE_FORMAT
        || bundle.schema_version != 1
        || bundle.target_profile != "clg.evm-compatible.v1"
        || bundle.target_execution_profile != "clg.evm-stateful-scalar.v1"
        || bundle.key_id.trim().is_empty()
        || bundle.source.trim().is_empty()
    {
        return failure("unsupported or malformed contract release bundle".to_string());
    }
    if bundle.orchestration.len() != 4
        || bundle
            .orchestration
            .iter()
            .any(|stage| stage.status != "ok")
    {
        return failure(
            "contract release bundle has incomplete orchestration evidence".to_string(),
        );
    }
    let known_stages = [
        "build_prove_sign",
        "verify_signature",
        "contract_test_simulate",
        "package",
    ];
    if bundle
        .orchestration
        .iter()
        .map(|stage| stage.stage.as_str())
        .collect::<Vec<_>>()
        != known_stages
    {
        return failure("contract release bundle has unexpected orchestration stages".to_string());
    }
    let directory = bundle_path.parent().unwrap_or_else(|| Path::new("."));
    let artifacts = [
        &bundle.artifacts.contract_abi,
        &bundle.artifacts.contract_state_schema,
        &bundle.artifacts.contract_test_evidence,
        &bundle.artifacts.contract_test_report,
        &bundle.artifacts.evm_artifact,
        &bundle.artifacts.evm_wire_abi,
        &bundle.artifacts.proof_artifact,
        &bundle.artifacts.signature,
        &bundle.artifacts.signed_assurance_manifest,
        &bundle.artifacts.vcs,
    ];
    for artifact in artifacts {
        if Path::new(&artifact.path).components().count() != 1
            || !artifact.sha256.starts_with("sha256:")
        {
            return failure(
                "contract release bundle has unsafe artifact path or digest".to_string(),
            );
        }
        let path = directory.join(&artifact.path);
        let actual = fs::read(&path)
            .map_err(|err| anyhow!(err).context("read contract release artifact"))?;
        if format!("sha256:{}", sha256_hex(&actual)) != artifact.sha256 {
            return failure(format!(
                "contract release artifact digest mismatch: {}",
                artifact.path
            ));
        }
    }
    let artifact_path = directory.join(&bundle.artifacts.evm_artifact.path);
    let abi_path = directory.join(&bundle.artifacts.contract_abi.path);
    let wire_path = directory.join(&bundle.artifacts.evm_wire_abi.path);
    let schema_path = directory.join(&bundle.artifacts.contract_state_schema.path);
    let proof_path = directory.join(&bundle.artifacts.proof_artifact.path);
    verify_campaign_evidence_index(
        directory,
        &bundle.artifacts.contract_test_report,
        &directory.join(&bundle.artifacts.contract_test_evidence.path),
    )
    .or_else(|err| failure(format!("contract release campaign evidence failed: {err}")))?;
    let artifact: Value = serde_json::from_slice(&fs::read(&artifact_path)?)?;
    let abi: Value = serde_json::from_slice(&fs::read(&abi_path)?)?;
    let wire: Value = serde_json::from_slice(&fs::read(&wire_path)?)?;
    let schema: Value = serde_json::from_slice(&fs::read(&schema_path)?)?;
    let proof: Value = serde_json::from_slice(&fs::read(&proof_path)?)?;
    if artifact["target"]["profile"] != bundle.target_profile
        || artifact["execution_profile"] != bundle.target_execution_profile
        || artifact["wire_abi"]["digest"] != wire["wire_abi"]["digest"]
        || artifact["state_schema"]["digest"] != schema["schema"]["digest"]
        || proof["format"] != "clg.proof_artifact.v1"
    {
        return failure(
            "contract release artifacts have incompatible target, ABI, schema, or proof evidence"
                .to_string(),
        );
    }
    let signature = match signing::verify_signature_details(
        &artifact_path,
        &directory.join(&bundle.artifacts.signature.path),
        &pubkey,
    ) {
        Ok(signature) => signature,
        Err(err) => return failure(format!("verify contract release signature: {err}")),
    };
    let assurance = match signing::verify_assurance_manifest(
        &directory.join(&bundle.artifacts.signed_assurance_manifest.path),
        &pubkey,
    ) {
        Ok(assurance) => assurance,
        Err(err) => return failure(format!("verify contract release assurance manifest: {err}")),
    };
    verify_evidence_binding(
        &bundle.key_id,
        &artifact,
        &abi,
        &wire,
        &schema,
        &proof,
        &signature,
        &assurance,
    )
    .or_else(|err| failure(format!("contract release evidence binding failed: {err}")))?;
    logger.event(
        crate::logging::LogLevel::Info,
        "verified",
        "contract_release",
        &[],
    );
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"schema_version":1,"status":"verified","bundle":bundle_path})
        )
        .expect("serialize verification")
    );
    Ok(())
}

struct ReleasePaths {
    artifact: PathBuf,
    schema: PathBuf,
    abi: PathBuf,
    wire_abi: PathBuf,
    vcs: PathBuf,
    proof: PathBuf,
    signature: PathBuf,
    assurance: PathBuf,
    campaign: PathBuf,
    campaign_evidence: PathBuf,
    traces: PathBuf,
    bundle: PathBuf,
}

fn artifact_file(path: &Path) -> Result<serde_json::Value> {
    let bytes =
        fs::read(path).with_context(|| format!("read release artifact `{}`", path.display()))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            anyhow!(
                "release artifact path has no filename: `{}`",
                path.display()
            )
        })?;
    Ok(json!({ "path": name, "sha256": format!("sha256:{}", sha256_hex(&bytes)) }))
}

fn write_campaign_evidence_index(
    report_path: &Path,
    trace_dir: &Path,
    index_path: &Path,
) -> Result<()> {
    let report_bytes = fs::read(report_path)
        .with_context(|| format!("read contract campaign report `{}`", report_path.display()))?;
    let report: Value = serde_json::from_slice(&report_bytes)
        .with_context(|| format!("parse contract campaign report `{}`", report_path.display()))?;
    if report["format"] != "clg.contract-test-report.v1" || report["schema_version"] != 1 {
        bail!("contract campaign report has an unsupported format");
    }
    let cases = report["cases"]
        .as_array()
        .ok_or_else(|| anyhow!("contract campaign report is missing cases"))?;
    let trace_directory = single_path_component(trace_dir, "contract campaign trace directory")?;
    let mut indexed_cases = Vec::with_capacity(cases.len());
    for (expected_case, case) in cases.iter().enumerate() {
        let case_number = case["case"]
            .as_u64()
            .filter(|number| *number == expected_case as u64)
            .ok_or_else(|| {
                anyhow!("contract campaign report cases must be ordered and numbered")
            })?;
        if case["status"] != "success" {
            bail!("contract campaign report contains an unsuccessful case");
        }
        let args = case
            .get("args")
            .cloned()
            .filter(Value::is_array)
            .ok_or_else(|| anyhow!("contract campaign report case is missing arguments"))?;
        let replay = case
            .get("replay")
            .cloned()
            .filter(|value| value["argv"].as_array().is_some())
            .ok_or_else(|| anyhow!("contract campaign report case is missing replay metadata"))?;
        let stem = format!("case-{case_number:04}");
        let files = [
            format!("{stem}.args.json"),
            format!("{stem}.state.json"),
            format!("{stem}.state-out.json"),
            format!("{stem}.trace.json"),
        ]
        .into_iter()
        .map(|name| campaign_trace_file(trace_dir, &name))
        .collect::<Result<Vec<_>>>()?;
        indexed_cases.push(CampaignEvidenceCase {
            case: case_number as u32,
            args,
            replay,
            files,
        });
    }
    let index = CampaignEvidenceIndex {
        format: CAMPAIGN_EVIDENCE_FORMAT.to_string(),
        schema_version: 1,
        report_sha256: format!("sha256:{}", sha256_hex(&report_bytes)),
        trace_directory,
        cases: indexed_cases,
    };
    fs::write(
        index_path,
        canonical_json_bytes(&serde_json::to_value(index)?),
    )
    .with_context(|| format!("write campaign evidence index `{}`", index_path.display()))
}

fn verify_campaign_evidence_index(
    bundle_directory: &Path,
    report_artifact: &Artifact,
    index_path: &Path,
) -> Result<()> {
    let index_bytes = fs::read(index_path)
        .with_context(|| format!("read campaign evidence index `{}`", index_path.display()))?;
    let index: CampaignEvidenceIndex = serde_json::from_slice(&index_bytes)
        .with_context(|| format!("parse campaign evidence index `{}`", index_path.display()))?;
    if index.format != CAMPAIGN_EVIDENCE_FORMAT || index.schema_version != 1 {
        bail!("campaign evidence index has an unsupported format");
    }
    if index.report_sha256 != report_artifact.sha256 {
        bail!("campaign evidence index report digest does not match the release bundle");
    }
    let report_path = bundle_directory.join(&report_artifact.path);
    let report: Value =
        serde_json::from_slice(&fs::read(&report_path).with_context(|| {
            format!("read contract campaign report `{}`", report_path.display())
        })?)
        .with_context(|| format!("parse contract campaign report `{}`", report_path.display()))?;
    if report["format"] != "clg.contract-test-report.v1"
        || report["schema_version"] != 1
        || report["status"] != "success"
    {
        bail!("contract campaign report has an unsupported or unsuccessful status");
    }
    let report_cases = report["cases"]
        .as_array()
        .ok_or_else(|| anyhow!("contract campaign report is missing cases"))?;
    if report_cases.len() != index.cases.len() {
        bail!("campaign evidence index case count does not match the campaign report");
    }
    let trace_directory = safe_relative_name(&index.trace_directory, "campaign trace directory")?;
    let trace_root = bundle_directory.join(trace_directory);
    let bundle_root = fs::canonicalize(bundle_directory).with_context(|| {
        format!(
            "resolve release bundle directory `{}`",
            bundle_directory.display()
        )
    })?;
    let trace_metadata = fs::symlink_metadata(&trace_root).with_context(|| {
        format!(
            "inspect campaign trace directory `{}`",
            trace_root.display()
        )
    })?;
    if trace_metadata.file_type().is_symlink() || !trace_metadata.is_dir() {
        bail!("campaign trace directory must be a bundle-local directory");
    }
    let trace_root =
        fs::canonicalize(&trace_root).with_context(|| "resolve campaign trace directory")?;
    if !trace_root.starts_with(&bundle_root) {
        bail!("campaign trace directory resolves outside the release bundle");
    }
    for (expected_case, (report_case, evidence_case)) in
        report_cases.iter().zip(index.cases.iter()).enumerate()
    {
        if evidence_case.case != expected_case as u32
            || report_case["case"].as_u64() != Some(expected_case as u64)
            || report_case["status"] != "success"
            || report_case["args"] != evidence_case.args
            || report_case["replay"] != evidence_case.replay
        {
            bail!("campaign evidence index case does not match the campaign report");
        }
        let stem = format!("case-{expected_case:04}");
        let expected_files = [
            format!("{stem}.args.json"),
            format!("{stem}.state.json"),
            format!("{stem}.state-out.json"),
            format!("{stem}.trace.json"),
        ];
        if evidence_case.files.len() != expected_files.len() {
            bail!("campaign evidence index case has incomplete trace/input evidence");
        }
        for (artifact, expected_name) in evidence_case.files.iter().zip(expected_files) {
            if artifact.path != expected_name {
                bail!("campaign evidence index has an unexpected trace/input path");
            }
            let name = safe_relative_name(&artifact.path, "campaign trace/input")?;
            let path = trace_root.join(name);
            let metadata = fs::symlink_metadata(&path).with_context(|| {
                format!("inspect indexed campaign trace/input `{}`", path.display())
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                bail!("campaign trace/input must be a regular bundle-local file");
            }
            let bytes =
                fs::read(&path).with_context(|| "read indexed campaign trace/input evidence")?;
            if format!("sha256:{}", sha256_hex(&bytes)) != artifact.sha256 {
                bail!("campaign evidence index trace/input digest does not match");
            }
        }
        let args_bytes = fs::read(trace_root.join(format!("{stem}.args.json")))
            .with_context(|| "read indexed campaign arguments")?;
        let args: Value = serde_json::from_slice(&args_bytes)
            .with_context(|| "parse indexed campaign arguments")?;
        if args != evidence_case.args {
            bail!("campaign evidence index arguments do not match the indexed input file");
        }
    }
    Ok(())
}

fn campaign_trace_file(trace_dir: &Path, name: &str) -> Result<Artifact> {
    let path = trace_dir.join(name);
    let bytes = fs::read(&path)
        .with_context(|| format!("read campaign trace/input `{}`", path.display()))?;
    Ok(Artifact {
        path: name.to_string(),
        sha256: format!("sha256:{}", sha256_hex(&bytes)),
    })
}

fn single_path_component(path: &Path, description: &str) -> Result<String> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("{description} must have a file name"))?;
    Ok(safe_relative_name(name, description)?.to_string())
}

fn safe_relative_name<'a>(name: &'a str, description: &str) -> Result<&'a str> {
    let path = Path::new(name);
    if name.is_empty()
        || name == "."
        || name == ".."
        || path.is_absolute()
        || path.components().count() != 1
    {
        bail!("{description} has an unsafe path");
    }
    Ok(name)
}

#[allow(clippy::too_many_arguments)]
fn verify_evidence_binding(
    bundle_key_id: &str,
    artifact: &Value,
    abi: &Value,
    wire: &Value,
    schema: &Value,
    proof: &Value,
    signature: &signing::SignatureFile,
    assurance: &signing::AssuranceManifestFile,
) -> Result<()> {
    let compiler = identity_object(artifact, "compiler", "EVM artifact")?;
    let contract = identity_object(artifact, "contract", "EVM artifact")?;
    let source_graph = source_graph_identity(artifact, "EVM artifact")?;
    for (name, evidence) in [
        ("contract ABI", abi),
        ("EVM wire ABI", wire),
        ("contract-state schema", schema),
        ("proof artifact", proof),
    ] {
        ensure_same_identity(
            "EVM artifact compiler",
            &compiler,
            &format!("{name} compiler"),
            &identity_object(evidence, "compiler", name)?,
        )?;
        ensure_same_identity(
            "EVM artifact source graph",
            &source_graph,
            &format!("{name} source graph"),
            &source_graph_identity(evidence, name)?,
        )?;
    }
    for (name, evidence) in [
        ("contract ABI", abi),
        ("EVM wire ABI", wire),
        ("contract-state schema", schema),
    ] {
        ensure_same_identity(
            "EVM artifact contract",
            &contract,
            &format!("{name} contract"),
            &identity_object(evidence, "contract", name)?,
        )?;
    }

    if signature.key_id != bundle_key_id {
        bail!("signature key ID does not match the release bundle");
    }
    if assurance.signature.key_id != bundle_key_id {
        bail!("assurance-manifest key ID does not match the release bundle");
    }
    let payload = signature
        .payload
        .as_object()
        .ok_or_else(|| anyhow!("signature payload must be an object"))?;
    ensure_same_identity(
        "EVM artifact compiler",
        &compiler,
        "signature payload compiler",
        &required_object(payload, "compiler", "signature payload")?,
    )?;
    ensure_same_identity(
        "EVM artifact source graph",
        &source_graph,
        "signature payload source graph",
        &source_graph_identity(&Value::Object(payload.clone()), "signature payload")?,
    )?;

    let proof_hash = format!("sha256:{}", sha256_hex(&canonical_json_bytes(proof)));
    ensure_same_digest(
        &required_string(payload, "proof_artifact_hash", "signature payload")?,
        &proof_hash,
        "signature payload proof-artifact hash",
    )?;
    let assurance_payload = assurance
        .payload
        .as_object()
        .ok_or_else(|| anyhow!("assurance manifest payload must be an object"))?;
    let assurance_artifacts =
        required_object(assurance_payload, "artifacts", "assurance manifest payload")?;
    ensure_same_digest(
        &required_string(
            assurance_artifacts
                .as_object()
                .expect("artifacts is an object"),
            "proof_artifact_hash",
            "assurance manifest",
        )?,
        &proof_hash,
        "assurance manifest proof-artifact hash",
    )?;
    let module_hash = required_string(payload, "module_hash", "signature payload")?;
    ensure_same_digest(
        &required_string(
            assurance_artifacts
                .as_object()
                .expect("artifacts is an object"),
            "module_hash",
            "assurance manifest",
        )?,
        &module_hash,
        "assurance manifest module hash",
    )?;
    let proofs_hash = required_string(payload, "proofs_hash", "signature payload")?;
    ensure_same_digest(
        &required_string(
            assurance_artifacts
                .as_object()
                .expect("artifacts is an object"),
            "proofs_hash",
            "assurance manifest",
        )?,
        &proofs_hash,
        "assurance manifest proofs hash",
    )?;

    let compiler_name = required_string(
        compiler.as_object().expect("compiler is an object"),
        "name",
        "EVM artifact compiler",
    )?;
    let compiler_version = required_string(
        compiler.as_object().expect("compiler is an object"),
        "version",
        "EVM artifact compiler",
    )?;
    let toolchain = required_object(assurance_payload, "toolchain", "assurance manifest payload")?;
    let expected_toolchain = format!("{compiler_name}/{compiler_version}");
    if required_string(
        toolchain.as_object().expect("toolchain is an object"),
        "name",
        "assurance manifest toolchain",
    )? != expected_toolchain
    {
        bail!("assurance manifest toolchain does not match the EVM artifact compiler");
    }
    ensure_same_digest(
        &required_string(
            toolchain.as_object().expect("toolchain is an object"),
            "fingerprint_sha256",
            "assurance manifest toolchain",
        )?,
        &sha256_hex(expected_toolchain.as_bytes()),
        "assurance manifest toolchain fingerprint",
    )?;
    Ok(())
}

fn identity_object(value: &Value, key: &str, description: &str) -> Result<Value> {
    let identity = value[key].clone();
    if !identity.is_object() {
        bail!("{description} is missing its `{key}` identity object");
    }
    Ok(identity)
}

fn source_graph_identity(value: &Value, description: &str) -> Result<Value> {
    let source_graph = identity_object(value, "source_graph", description)?;
    let source_graph_object = source_graph
        .as_object()
        .expect("source graph identity is an object");
    if required_string(source_graph_object, "algorithm", description)?
        != "clg.loaded-source-graph.v1"
    {
        bail!("{description} has an unsupported loaded-source graph");
    }
    let files = source_graph_object
        .get("files")
        .filter(|value| value.is_array())
        .ok_or_else(|| anyhow!("{description} loaded-source graph is missing files"))?;
    let expected_digest = format!(
        "sha256:{}",
        sha256_hex(&canonical_json_bytes(&json!({
            "algorithm": source_graph_object["algorithm"],
            "files": files,
        })))
    );
    ensure_same_digest(
        &required_string(source_graph_object, "digest", description)?,
        &expected_digest,
        &format!("{description} loaded-source graph digest"),
    )?;
    Ok(source_graph)
}

fn required_object(
    object: &serde_json::Map<String, Value>,
    key: &str,
    description: &str,
) -> Result<Value> {
    let value = object
        .get(key)
        .cloned()
        .filter(Value::is_object)
        .ok_or_else(|| anyhow!("{description} is missing its `{key}` object"))?;
    Ok(value)
}

fn required_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    description: &str,
) -> Result<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("{description} is missing its `{key}` string"))
}

fn ensure_same_digest(actual: &str, expected: &str, description: &str) -> Result<()> {
    if actual != expected {
        bail!("{description} does not match");
    }
    Ok(())
}

fn ensure_same_identity(
    left_name: &str,
    left: &Value,
    right_name: &str,
    right: &Value,
) -> Result<()> {
    if canonical_json_bytes(left) != canonical_json_bytes(right) {
        bail!("cross-artifact identity drift: {left_name} does not match {right_name}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matching_evidence() -> (
        Value,
        Value,
        Value,
        Value,
        Value,
        signing::SignatureFile,
        signing::AssuranceManifestFile,
    ) {
        let files = json!([{"path":"counter.clear","sha256":"sha256:source"}]);
        let source_graph = json!({
            "algorithm": "clg.loaded-source-graph.v1",
            "digest": format!("sha256:{}", sha256_hex(&canonical_json_bytes(&json!({
                "algorithm": "clg.loaded-source-graph.v1",
                "files": files,
            })))),
            "files": files,
        });
        let compiler = json!({"name":"clg-cli","version":"0.1.0"});
        let contract = json!({"name":"Counter","version":1});
        let artifact = json!({
            "compiler": compiler,
            "contract": contract,
            "source_graph": source_graph,
        });
        let abi = artifact.clone();
        let wire = artifact.clone();
        let schema = artifact.clone();
        let proof = json!({
            "compiler": artifact["compiler"],
            "source_graph": artifact["source_graph"],
        });
        let proof_hash = format!("sha256:{}", sha256_hex(&canonical_json_bytes(&proof)));
        let module_hash = "a".repeat(64);
        let proofs_hash = "b".repeat(64);
        let signature = signing::SignatureFile {
            key_id: "release-key".to_string(),
            scope: SignScope::Both,
            signature_format: "ed25519".to_string(),
            signature: String::new(),
            payload: json!({
                "compiler": artifact["compiler"],
                "module_hash": module_hash,
                "proof_artifact_hash": proof_hash,
                "proofs_hash": proofs_hash,
                "source_graph": artifact["source_graph"],
            }),
        };
        let toolchain = "clg-cli/0.1.0";
        let assurance = signing::AssuranceManifestFile {
            schema_version: 1,
            payload: json!({
                "artifacts": {
                    "module_hash": module_hash,
                    "proof_artifact_hash": proof_hash,
                    "proofs_hash": proofs_hash,
                },
                "toolchain": {
                    "name": toolchain,
                    "fingerprint_sha256": sha256_hex(toolchain.as_bytes()),
                },
            }),
            signature: signing::AssuranceManifestSignature {
                key_id: "release-key".to_string(),
                signature_format: "ed25519".to_string(),
                payload_hash: String::new(),
                signature: String::new(),
            },
        };
        (artifact, abi, wire, schema, proof, signature, assurance)
    }

    fn verify_matching_evidence(
        artifact: &Value,
        abi: &Value,
        wire: &Value,
        schema: &Value,
        proof: &Value,
        signature: &signing::SignatureFile,
        assurance: &signing::AssuranceManifestFile,
    ) -> Result<()> {
        verify_evidence_binding(
            "release-key",
            artifact,
            abi,
            wire,
            schema,
            proof,
            signature,
            assurance,
        )
    }

    #[test]
    fn evidence_binding_accepts_matching_release_evidence() {
        let (artifact, abi, wire, schema, proof, signature, assurance) = matching_evidence();
        verify_matching_evidence(
            &artifact, &abi, &wire, &schema, &proof, &signature, &assurance,
        )
        .expect("matching evidence must verify");
    }

    #[test]
    fn evidence_binding_rejects_signature_proof_hash_drift() {
        let (artifact, abi, wire, schema, proof, mut signature, assurance) = matching_evidence();
        signature.payload["proof_artifact_hash"] = json!("sha256:tampered");
        let err = verify_matching_evidence(
            &artifact, &abi, &wire, &schema, &proof, &signature, &assurance,
        )
        .expect_err("proof-hash drift must fail closed");
        assert!(err
            .to_string()
            .contains("signature payload proof-artifact hash does not match"));
    }

    #[test]
    fn evidence_binding_rejects_source_graph_and_key_id_drift() {
        let (artifact, mut abi, wire, schema, proof, signature, assurance) = matching_evidence();
        abi["source_graph"]["files"][0]["path"] = json!("other.clear");
        let err = verify_matching_evidence(
            &artifact, &abi, &wire, &schema, &proof, &signature, &assurance,
        )
        .expect_err("source-graph drift must fail closed");
        assert!(err.to_string().contains("loaded-source graph digest"));

        let (artifact, abi, wire, schema, proof, signature, mut assurance) = matching_evidence();
        assurance.signature.key_id = "other-key".to_string();
        let err = verify_matching_evidence(
            &artifact, &abi, &wire, &schema, &proof, &signature, &assurance,
        )
        .expect_err("assurance key drift must fail closed");
        assert!(err
            .to_string()
            .contains("assurance-manifest key ID does not match"));

        let (artifact, abi, wire, schema, proof, mut signature, assurance) = matching_evidence();
        signature.key_id = "other-key".to_string();
        let err = verify_matching_evidence(
            &artifact, &abi, &wire, &schema, &proof, &signature, &assurance,
        )
        .expect_err("signature key drift must fail closed");
        assert!(err.to_string().contains("signature key ID does not match"));
    }

    fn campaign_evidence_fixture() -> (tempfile::TempDir, Artifact, PathBuf) {
        let directory = tempfile::tempdir().expect("tempdir");
        let traces = directory.path().join("counter.campaign-traces");
        fs::create_dir(&traces).expect("create trace directory");
        let stem = "case-0000";
        fs::write(traces.join(format!("{stem}.args.json")), br#"[7]"#).expect("write args");
        fs::write(traces.join(format!("{stem}.state.json")), br#"{"total":0}"#)
            .expect("write state");
        fs::write(
            traces.join(format!("{stem}.state-out.json")),
            br#"{"total":7}"#,
        )
        .expect("write state output");
        fs::write(
            traces.join(format!("{stem}.trace.json")),
            br#"{"result":7,"state_after":{"total":7}}"#,
        )
        .expect("write trace");
        let report_path = directory.path().join("counter.campaign.json");
        fs::write(
            &report_path,
            canonical_json_bytes(&json!({
                "cases": [{
                    "args": [7],
                    "case": 0,
                    "replay": {"argv": ["clg", "simulate", "counter.clear"]},
                    "status": "success",
                }],
                "format": "clg.contract-test-report.v1",
                "schema_version": 1,
                "status": "success",
            })),
        )
        .expect("write report");
        let index_path = directory.path().join("counter.campaign-evidence.json");
        write_campaign_evidence_index(&report_path, &traces, &index_path)
            .expect("write evidence index");
        let report_bytes = fs::read(&report_path).expect("read report");
        (
            directory,
            Artifact {
                path: "counter.campaign.json".to_string(),
                sha256: format!("sha256:{}", sha256_hex(&report_bytes)),
            },
            index_path,
        )
    }

    #[test]
    fn campaign_evidence_index_verifies_all_bundle_local_trace_inputs() {
        let (directory, report, index_path) = campaign_evidence_fixture();
        verify_campaign_evidence_index(directory.path(), &report, &index_path)
            .expect("complete campaign evidence must verify");
    }

    #[test]
    fn campaign_evidence_index_rejects_tampered_or_unsafe_trace_evidence() {
        let (directory, report, index_path) = campaign_evidence_fixture();
        fs::write(
            directory
                .path()
                .join("counter.campaign-traces")
                .join("case-0000.trace.json"),
            br#"{"result":8,"state_after":{"total":8}}"#,
        )
        .expect("tamper trace");
        let err = verify_campaign_evidence_index(directory.path(), &report, &index_path)
            .expect_err("tampered trace must fail closed");
        assert!(err
            .to_string()
            .contains("trace/input digest does not match"));

        let (directory, report, index_path) = campaign_evidence_fixture();
        let mut index: Value = serde_json::from_slice(&fs::read(&index_path).expect("read index"))
            .expect("parse index");
        index["trace_directory"] = json!("../outside");
        fs::write(&index_path, canonical_json_bytes(&index)).expect("write unsafe index");
        let err = verify_campaign_evidence_index(directory.path(), &report, &index_path)
            .expect_err("unsafe trace path must fail closed");
        assert!(err
            .to_string()
            .contains("campaign trace directory has an unsafe path"));
    }
}
