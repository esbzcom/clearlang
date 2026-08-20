//! One-command, fail-closed release preparation for the supported contract target profile.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::json;

use super::{build, contract_test};
use crate::commands::helpers::{canonical_json_bytes, sha256_hex};
use crate::logging::{Logger, StageTimings};
use crate::signing::{self, SignScope};

const BUNDLE_FORMAT: &str = "clg.contract-release-bundle.v1";

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
    contract_test_report: Artifact,
    evm_artifact: Artifact,
    evm_wire_abi: Artifact,
    proof_artifact: Artifact,
    signature: Artifact,
    signed_assurance_manifest: Artifact,
    vcs: Artifact,
}

#[derive(Deserialize)]
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
    let bundle = {
        let _stage = timings.start(logger, "contract_release_bundle");
        let manifest = json!({
            "artifacts": {
                "contract_abi": artifact_file(&paths.abi)?,
                "contract_state_schema": artifact_file(&paths.schema)?,
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
    let wire_path = directory.join(&bundle.artifacts.evm_wire_abi.path);
    let schema_path = directory.join(&bundle.artifacts.contract_state_schema.path);
    let proof_path = directory.join(&bundle.artifacts.proof_artifact.path);
    let artifact: serde_json::Value = serde_json::from_slice(&fs::read(&artifact_path)?)?;
    let wire: serde_json::Value = serde_json::from_slice(&fs::read(&wire_path)?)?;
    let schema: serde_json::Value = serde_json::from_slice(&fs::read(&schema_path)?)?;
    let proof: serde_json::Value = serde_json::from_slice(&fs::read(&proof_path)?)?;
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
    signing::verify_signature_details(
        &artifact_path,
        &directory.join(&bundle.artifacts.signature.path),
        &pubkey,
    )
    .map_err(|err| anyhow!("verify contract release signature: {err}"))?;
    signing::verify_assurance_manifest(
        &directory.join(&bundle.artifacts.signed_assurance_manifest.path),
        &pubkey,
    )
    .map_err(|err| anyhow!("verify contract release assurance manifest: {err}"))?;
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
