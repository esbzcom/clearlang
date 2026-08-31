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
    let abi_path = directory.join(&bundle.artifacts.contract_abi.path);
    let wire_path = directory.join(&bundle.artifacts.evm_wire_abi.path);
    let schema_path = directory.join(&bundle.artifacts.contract_state_schema.path);
    let proof_path = directory.join(&bundle.artifacts.proof_artifact.path);
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
    let signature = signing::verify_signature_details(
        &artifact_path,
        &directory.join(&bundle.artifacts.signature.path),
        &pubkey,
    )
    .map_err(|err| anyhow!("verify contract release signature: {err}"))?;
    let assurance = signing::verify_assurance_manifest(
        &directory.join(&bundle.artifacts.signed_assurance_manifest.path),
        &pubkey,
    )
    .map_err(|err| anyhow!("verify contract release assurance manifest: {err}"))?;
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
}
