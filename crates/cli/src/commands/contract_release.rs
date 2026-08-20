//! One-command, fail-closed release preparation for the supported contract target profile.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde_json::json;

use super::{build, contract_test};
use crate::commands::helpers::{canonical_json_bytes, sha256_hex};
use crate::logging::{Logger, StageTimings};
use crate::signing::{self, SignScope};

const BUNDLE_FORMAT: &str = "clg.contract-release-bundle.v1";

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
