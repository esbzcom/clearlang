use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::commands::build::{self, CompilerMode, ReleaseProfile, StdCoreLinkMode};
use crate::commands::helpers::{make_single_json_error, sha256_hex, CommandError};
use crate::commands::pkg;
use crate::commands::verify::{self, VerifyMode};
use crate::logging::{Logger, StageTimings};
use crate::signing::SignScope;

#[derive(Debug, Clone)]
struct ReleasePaths {
    module: PathBuf,
    vcs: PathBuf,
    proof: PathBuf,
    signature: PathBuf,
    assurance_manifest: PathBuf,
    bundle_manifest: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseBundleManifestV1 {
    schema_version: u32,
    policy_version: &'static str,
    proof_policy: &'static str,
    primary_commands: [&'static str; 3],
    entry: String,
    root: String,
    advisory_as_of: String,
    key_id: String,
    trust_policy: String,
    artifacts: ReleaseArtifacts,
    orchestration: Vec<ReleaseStageStatus>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseArtifacts {
    module: ReleaseArtifactFile,
    vcs: ReleaseArtifactFile,
    proof: ReleaseArtifactFile,
    signature: ReleaseArtifactFile,
    assurance_manifest: ReleaseArtifactFile,
    bundle_manifest: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseArtifactFile {
    path: String,
    sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseStageStatus {
    stage: &'static str,
    status: &'static str,
}

#[derive(Debug, Clone, Deserialize)]
struct VerifyTrustPolicyV1 {
    schema_version: u32,
    trust_anchors: VerifyTrustAnchors,
}

#[derive(Debug, Clone, Deserialize)]
struct VerifyTrustAnchors {
    lean_checker: String,
    coq_checker: String,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    file: PathBuf,
    advisory_as_of: String,
    key: PathBuf,
    key_id: String,
    pubkey: PathBuf,
    root: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    trust_policy: Option<PathBuf>,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    let root = root.unwrap_or_else(|| {
        file.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    });
    let verify_trust_policy = trust_policy.unwrap_or_else(|| root.join("trust-policy.json"));
    let stem = file_stem_or_error(file.as_path(), json_errors)?;
    let paths = release_paths(&root, out_dir, stem.as_str());
    let trust_anchors =
        load_verify_trust_policy_v1(verify_trust_policy.as_path(), file.as_path(), json_errors)?;

    let mut timings = StageTimings::new();
    {
        let _stage = timings.start(logger, "release_lock");
        let lockfile_path = root.join("clg.lock.json");
        let generate = !lockfile_path.exists();
        let update = !generate;
        pkg::run_lock(
            generate,
            update,
            CompilerMode::Strict,
            Some(advisory_as_of.clone()),
            root.clone(),
            json_errors,
            logger,
        )?;
    }
    {
        let _stage = timings.start(logger, "release_build_prove_sign");
        build::run(
            file.clone(),
            paths.module.clone(),
            false,
            false,
            false,
            Some(paths.vcs.clone()),
            Some(paths.proof.clone()),
            CompilerMode::Strict,
            ReleaseProfile::Production,
            StdCoreLinkMode::Intrinsic,
            None,
            true,
            Some(key),
            Some(key_id.clone()),
            SignScope::Both,
            Some(paths.signature.clone()),
            Some(paths.assurance_manifest.clone()),
            Some(trust_anchors.lean_checker.clone()),
            Some(trust_anchors.coq_checker.clone()),
            json_errors,
            logger,
        )?;
    }
    {
        let _stage = timings.start(logger, "release_verify");
        verify::run(
            paths.module.clone(),
            paths.signature.clone(),
            pubkey,
            VerifyMode::CompileTime,
            Some(verify_trust_policy.clone()),
            Some(paths.assurance_manifest.clone()),
            Some(paths.proof.clone()),
            None,
            Some("proved_all".to_string()),
            false,
            json_errors,
            logger,
        )?;
    }
    let bundle_manifest = {
        let _stage = timings.start(logger, "release_bundle");
        write_release_bundle_manifest(
            paths.bundle_manifest.as_path(),
            &file,
            &root,
            advisory_as_of.as_str(),
            key_id.as_str(),
            verify_trust_policy.as_path(),
            &paths,
        )?
    };

    logger.summary(&timings);
    println!(
        "{}",
        serde_json::to_string_pretty(&bundle_manifest).expect("serialize release bundle manifest")
    );
    Ok(())
}

fn release_paths(root: &Path, out_dir: Option<PathBuf>, stem: &str) -> ReleasePaths {
    let out_dir = out_dir.unwrap_or_else(|| root.join("out").join("release"));
    ReleasePaths {
        module: out_dir.join(format!("{stem}.wasm")),
        vcs: out_dir.join(format!("{stem}.vc.json")),
        proof: out_dir.join(format!("{stem}.proof.json")),
        signature: out_dir.join(format!("{stem}.sig.json")),
        assurance_manifest: out_dir.join(format!("{stem}.assurance.json")),
        bundle_manifest: out_dir.join(format!("{stem}.release-bundle.json")),
    }
}

fn load_verify_trust_policy_v1(
    path: &Path,
    file: &Path,
    json_errors: bool,
) -> Result<VerifyTrustAnchors> {
    let bytes = fs::read(path).map_err(|err| {
        release_error(
            "C130",
            format!("reading trust policy {}: {err}", path.display()),
            file,
            json_errors,
        )
    })?;
    let policy: VerifyTrustPolicyV1 = serde_json::from_slice(bytes.as_slice()).map_err(|err| {
        release_error(
            "C130",
            format!("parsing trust policy {}: {err}", path.display()),
            file,
            json_errors,
        )
    })?;
    if policy.schema_version != 1 {
        return Err(release_error(
            "C130",
            format!(
                "unsupported trust policy schema_version {} (expected 1) in {}",
                policy.schema_version,
                path.display()
            ),
            file,
            json_errors,
        ));
    }
    if policy.trust_anchors.lean_checker.trim().is_empty()
        || policy.trust_anchors.coq_checker.trim().is_empty()
    {
        return Err(release_error(
            "C130",
            format!(
                "trust policy {} must provide non-empty trust_anchors.lean_checker and trust_anchors.coq_checker",
                path.display()
            ),
            file,
            json_errors,
        ));
    }
    Ok(policy.trust_anchors)
}

fn write_release_bundle_manifest(
    path: &Path,
    entry: &Path,
    root: &Path,
    advisory_as_of: &str,
    key_id: &str,
    trust_policy: &Path,
    paths: &ReleasePaths,
) -> Result<ReleaseBundleManifestV1> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }

    let manifest = ReleaseBundleManifestV1 {
        schema_version: 1,
        policy_version: "25.2.3",
        proof_policy: "release == proved (require_assurance=proved_all)",
        primary_commands: ["check", "test", "release"],
        entry: entry.display().to_string(),
        root: root.display().to_string(),
        advisory_as_of: advisory_as_of.to_string(),
        key_id: key_id.to_string(),
        trust_policy: trust_policy.display().to_string(),
        artifacts: ReleaseArtifacts {
            module: artifact_file(paths.module.as_path())?,
            vcs: artifact_file(paths.vcs.as_path())?,
            proof: artifact_file(paths.proof.as_path())?,
            signature: artifact_file(paths.signature.as_path())?,
            assurance_manifest: artifact_file(paths.assurance_manifest.as_path())?,
            bundle_manifest: path.display().to_string(),
        },
        orchestration: vec![
            ReleaseStageStatus {
                stage: "lock",
                status: "ok",
            },
            ReleaseStageStatus {
                stage: "build/prove",
                status: "ok",
            },
            ReleaseStageStatus {
                stage: "sign",
                status: "ok",
            },
            ReleaseStageStatus {
                stage: "verify(require-assurance=proved_all)",
                status: "ok",
            },
            ReleaseStageStatus {
                stage: "bundle",
                status: "ok",
            },
        ],
    };

    let bytes =
        serde_json::to_vec_pretty(&manifest).context("serializing release bundle manifest JSON")?;
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(manifest)
}

fn artifact_file(path: &Path) -> Result<ReleaseArtifactFile> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(ReleaseArtifactFile {
        path: path.display().to_string(),
        sha256: sha256_hex(bytes.as_slice()),
    })
}

fn file_stem_or_error(file: &Path, json_errors: bool) -> Result<String> {
    let stem = file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::trim)
        .filter(|stem| !stem.is_empty())
        .map(ToOwned::to_owned);
    match stem {
        Some(value) => Ok(value),
        None => Err(release_error(
            "C130",
            format!(
                "could not derive release artifact stem from entry path `{}`",
                file.display()
            ),
            file,
            json_errors,
        )),
    }
}

fn release_error(
    code: &'static str,
    message: String,
    file: &Path,
    json_errors: bool,
) -> anyhow::Error {
    if json_errors {
        let json = make_single_json_error(code, "release", message, file, 0, 0, None);
        CommandError::json(json).into()
    } else {
        anyhow::anyhow!(message)
    }
}
