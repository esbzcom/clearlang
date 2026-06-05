use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::commands::build::{self, CompilerMode, ReleaseProfile, StdCoreLinkMode};
use crate::commands::helpers::{make_single_json_error, sha256_hex, CommandError};
use crate::commands::pkg;
use crate::commands::release_defaults::{
    enforce_project_clg_version_compatibility, is_release_defaults_placeholder,
    load_project_manifest_v1, load_required_release_defaults_v0, load_verify_trust_policy_v1,
    ProjectManifestV1, STRICT_PROJECT_FILE,
};
use crate::commands::shared_std_lock::{
    load_validated_shared_std_lock_section, project_shared_std_evidence,
};
use crate::commands::verify::{self, VerifyMode};
use crate::logging::{Logger, StageTimings};
use crate::signing::{self, SignScope};

type ReleaseSharedStdPackageEvidence = crate::commands::shared_std_lock::SharedStdEvidence;
#[cfg(test)]
type ReleaseSharedStdAbiClaim = crate::commands::shared_std_lock::SharedStdAbiClaim;

#[derive(Debug, Clone)]
struct ReleasePaths {
    module: PathBuf,
    strict_import_map: PathBuf,
    vcs: PathBuf,
    proof: PathBuf,
    signature: PathBuf,
    assurance_manifest: PathBuf,
    provenance: PathBuf,
    bundle_manifest: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseBundleManifestV1 {
    schema_version: u32,
    policy_version: String,
    proof_policy: String,
    primary_commands: Vec<String>,
    entry: String,
    root: String,
    advisory_as_of: String,
    key_id: String,
    trust_policy: String,
    #[serde(default)]
    shared_std: Vec<ReleaseSharedStdPackageEvidence>,
    artifacts: ReleaseArtifacts,
    orchestration: Vec<ReleaseStageStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseArtifacts {
    module: ReleaseArtifactFile,
    strict_import_map: ReleaseArtifactFile,
    vcs: ReleaseArtifactFile,
    proof: ReleaseArtifactFile,
    signature: ReleaseArtifactFile,
    assurance_manifest: ReleaseArtifactFile,
    #[serde(skip_serializing_if = "Option::is_none")]
    provenance: Option<ReleaseArtifactFile>,
    bundle_manifest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseArtifactFile {
    path: String,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseStageStatus {
    stage: String,
    status: String,
}

#[derive(Debug, Clone, Deserialize)]
struct VerifyBundleKeyringV1 {
    schema_version: u32,
    keys: Vec<VerifyBundleKeyringEntry>,
    revoked_key_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct VerifyBundleKeyringEntry {
    key_id: String,
    pubkey: String,
}

pub fn run(
    key: Option<PathBuf>,
    pubkey: Option<PathBuf>,
    check_only: bool,
    root: Option<PathBuf>,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    if check_only && (key.is_some() || pubkey.is_some()) {
        return Err(release_error(
            "C130",
            "`--check-only` cannot be combined with `--key/--pubkey`; run readiness gate without signing flags".to_string(),
            Path::new(STRICT_PROJECT_FILE),
            json_errors,
        ));
    }

    let root = resolve_release_root(root, json_errors)?;
    let key_for_release = key.clone();
    let manifest_path = root.join(STRICT_PROJECT_FILE);

    let release_defaults = load_required_release_defaults_v0(root.as_path()).map_err(|err| {
        release_error(
            "C130",
            format!("loading `{}`: {}", STRICT_PROJECT_FILE, err.message()),
            manifest_path.as_path(),
            json_errors,
        )
    })?;

    let project_manifest = load_project_manifest_v1(root.as_path()).map_err(|err| {
        release_error(
            "C130",
            err.message().to_string(),
            manifest_path.as_path(),
            json_errors,
        )
    })?;
    let project_manifest = project_manifest.ok_or_else(|| {
        release_error(
            "C130",
            format!(
                "release requires schema v1/v2 `{}` with `project.entry` configured",
                STRICT_PROJECT_FILE
            ),
            manifest_path.as_path(),
            json_errors,
        )
    })?;
    enforce_project_clg_version_compatibility(&project_manifest).map_err(|err| {
        release_error(
            "C130",
            err.message().to_string(),
            manifest_path.as_path(),
            json_errors,
        )
    })?;
    let file = root.join(project_manifest.project.entry.as_str());
    if !file.exists() || !file.is_file() {
        return Err(release_error(
            "C130",
            format!(
                "release entrypoint `project.entry = {}` resolved to `{}` which is missing or not a file",
                project_manifest.project.entry,
                file.display()
            ),
            manifest_path.as_path(),
            json_errors,
        ));
    }

    let advisory_as_of = resolve_required_manifest_release_value(
        release_defaults.advisory_as_of.as_str(),
        "release_defaults.advisory_as_of",
        file.as_path(),
        json_errors,
    )?;

    let key_id = resolve_required_manifest_release_value(
        release_defaults.key_id.as_str(),
        "release_defaults.key_id",
        file.as_path(),
        json_errors,
    )?;

    let verify_trust_policy = root.join(release_defaults.trust_policy.as_str());
    let stem = file_stem_or_error(file.as_path(), json_errors)?;
    let paths = release_paths(root.join(release_defaults.out_dir.as_str()), stem.as_str());
    let trust_anchors =
        load_verify_trust_policy_v1(verify_trust_policy.as_path()).map_err(|err| {
            release_error(
                "C130",
                err.message().to_string(),
                file.as_path(),
                json_errors,
            )
        })?;

    let mut timings = StageTimings::new();
    {
        let _stage = timings.start(logger, "release_lock");
        let lockfile_path = root.join("clg.lock.json");
        let generate = !lockfile_path.exists();
        let update = !generate;
        pkg::run_lock(
            pkg::RunLockArgs {
                generate,
                update,
                compiler_mode: CompilerMode::Strict,
                advisory_as_of: Some(advisory_as_of.clone()),
                root: root.clone(),
                json_errors,
                emit_stdout_summary: false,
            },
            logger,
        )?;
    }
    {
        let _stage = timings.start(logger, "release_build_prove_sign");
        let (sign, key_path, key_id_for_sign, lean_checker, coq_checker) = if check_only {
            (false, None, None, None, None)
        } else {
            let key = key.ok_or_else(|| {
                release_error(
                    "C130",
                    "missing `--key <FILE>` (required unless `--check-only`)".to_string(),
                    file.as_path(),
                    json_errors,
                )
            })?;
            pubkey.as_ref().ok_or_else(|| {
                release_error(
                    "C130",
                    "missing `--pubkey <FILE>` (required unless `--check-only`)".to_string(),
                    file.as_path(),
                    json_errors,
                )
            })?;
            (
                true,
                Some(key),
                Some(key_id.clone()),
                Some(trust_anchors.lean_checker.clone()),
                Some(trust_anchors.coq_checker.clone()),
            )
        };
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
            sign,
            key_path,
            key_id_for_sign,
            SignScope::Both,
            if sign {
                Some(paths.signature.clone())
            } else {
                None
            },
            if sign {
                Some(paths.assurance_manifest.clone())
            } else {
                None
            },
            lean_checker,
            coq_checker,
            json_errors,
            logger,
        )?;
    }
    {
        let _stage = timings.start(logger, "release_artifact_scan");
        validate_release_import_map_has_no_test_paths(&paths, file.as_path(), json_errors)?;
    }
    if check_only {
        logger.summary(&timings);
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1,
                "mode": "check_only",
                "status": "ready",
                "entry": file.display().to_string(),
                "root": root.display().to_string(),
                "advisory_as_of": advisory_as_of,
                "orchestration": [
                    { "stage": "lock", "status": "ok" },
                    { "stage": "build/prove", "status": "ok" },
                    { "stage": "artifact_scan", "status": "ok" }
                ]
            }))
            .expect("serialize release check-only summary")
        );
        return Ok(());
    }

    let pubkey = pubkey.expect("pubkey should be present when not check-only");
    let release_signing_key =
        key_for_release.expect("signing key should be present when not check-only");
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
    let shared_std =
        collect_release_shared_std_package_evidence(root.as_path()).map_err(|err| {
            release_error(
                "C130",
                format!("loading shared std release evidence: {err:#}"),
                file.as_path(),
                json_errors,
            )
        })?;
    validate_release_shared_std_mode(&project_manifest, shared_std.as_slice())
        .map_err(|message| release_error("C130", message, file.as_path(), json_errors))?;
    {
        let _stage = timings.start(logger, "release_provenance");
        write_release_provenance(
            paths.provenance.as_path(),
            &release_signing_key,
            key_id.as_str(),
            advisory_as_of.as_str(),
            file.as_path(),
            root.as_path(),
            &paths,
            shared_std.as_slice(),
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
            shared_std.as_slice(),
        )?
    };

    logger.summary(&timings);
    println!(
        "{}",
        serde_json::to_string_pretty(&bundle_manifest).expect("serialize release bundle manifest")
    );
    Ok(())
}

pub fn run_verify_bundle(
    bundle: PathBuf,
    pubkey: Option<PathBuf>,
    keyring: Option<PathBuf>,
    require_provenance: bool,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    const VERIFY_BUNDLE_ERROR_CODE: &str = "C140";

    let mut timings = StageTimings::new();
    let (manifest, bundle_dir) = {
        let _stage = timings.start(logger, "verify_bundle_manifest");
        let bytes = fs::read(&bundle).map_err(|err| {
            release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                format!(
                    "reading release bundle manifest `{}`: {err}",
                    bundle.display()
                ),
                bundle.as_path(),
                json_errors,
            )
        })?;
        let manifest: ReleaseBundleManifestV1 = serde_json::from_slice(&bytes).map_err(|err| {
            release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                format!(
                    "parsing release bundle manifest `{}`: {err}",
                    bundle.display()
                ),
                bundle.as_path(),
                json_errors,
            )
        })?;
        if manifest.schema_version != 1 {
            return Err(release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                format!(
                    "unsupported release bundle schema_version {} (expected 1)",
                    manifest.schema_version
                ),
                bundle.as_path(),
                json_errors,
            ));
        }
        let bundle_dir = bundle
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        (manifest, bundle_dir)
    };

    let resolved = {
        let _stage = timings.start(logger, "verify_bundle_artifacts");
        resolve_release_artifact_paths(&manifest, bundle_dir.as_path())
    };
    let resolved = resolved.map_err(|message| {
        release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            message,
            bundle.as_path(),
            json_errors,
        )
    })?;
    {
        let _stage = timings.start(logger, "verify_bundle_hashes");
        verify_release_artifact_hashes(resolved.as_slice(), bundle.as_path(), json_errors)?;
    }

    let trust_policy = resolve_manifest_path(&manifest.trust_policy, bundle_dir.as_path());
    let module = resolved_path_for("module", resolved.as_slice()).expect("module path");
    let signature = resolved_path_for("signature", resolved.as_slice()).expect("signature path");
    let proof = resolved_path_for("proof", resolved.as_slice()).expect("proof path");
    let assurance_manifest =
        resolved_path_for("assurance_manifest", resolved.as_slice()).expect("assurance path");
    let selected_pubkey = {
        let _stage = timings.start(logger, "verify_bundle_key_select");
        resolve_verify_bundle_pubkey(
            pubkey,
            keyring,
            signature.as_path(),
            bundle.as_path(),
            json_errors,
        )?
    };
    {
        let _stage = timings.start(logger, "verify_bundle_provenance");
        verify_bundle_provenance(
            &manifest,
            bundle_dir.as_path(),
            selected_pubkey.as_path(),
            signature.as_path(),
            require_provenance,
            bundle.as_path(),
            json_errors,
        )?;
    }
    {
        let _stage = timings.start(logger, "verify_bundle_shared_std");
        verify_bundle_strict_import_map_shared_std_parity(
            &manifest,
            bundle_dir.as_path(),
            bundle.as_path(),
            json_errors,
        )?;
    }

    {
        let _stage = timings.start(logger, "verify_bundle_signature");
        verify::run(
            module,
            signature,
            selected_pubkey,
            VerifyMode::CompileTime,
            Some(trust_policy),
            Some(assurance_manifest),
            Some(proof),
            None,
            Some("proved_all".to_string()),
            false,
            json_errors,
            logger,
        )?;
    }

    logger.summary(&timings);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 1,
            "status": "verified",
            "bundle": bundle.display().to_string(),
        }))
        .expect("serialize verify-bundle summary")
    );
    Ok(())
}

fn resolve_release_root(root: Option<PathBuf>, json_errors: bool) -> Result<PathBuf> {
    if let Some(explicit_root) = root {
        return Ok(explicit_root);
    }
    let cwd = std::env::current_dir()
        .map_err(|err| anyhow::anyhow!("resolving current directory for release: {err}"))?;
    let mut candidates = discover_manifest_roots(cwd.as_path())?;
    candidates.sort();
    candidates.dedup();
    match candidates.len() {
        0 => Err(release_error(
            "C130",
            format!(
                "could not find `{}` under current directory `{}`; pass `--root <DIR>` or run from a project directory",
                STRICT_PROJECT_FILE,
                cwd.display()
            ),
            cwd.as_path(),
            json_errors,
        )),
        1 => Ok(candidates.remove(0)),
        _ => {
            let shown = candidates
                .iter()
                .take(5)
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let suffix = if candidates.len() > 5 {
                format!(" (showing 5 of {})", candidates.len())
            } else {
                String::new()
            };
            Err(release_error(
                "C130",
                format!(
                    "multiple `{}` files found under current directory `{}`: {}{}; pass `--root <DIR>`",
                    STRICT_PROJECT_FILE,
                    cwd.display(),
                    shown,
                    suffix
                ),
                cwd.as_path(),
                json_errors,
            ))
        }
    }
}

fn discover_manifest_roots(search_root: &Path) -> Result<Vec<PathBuf>> {
    let mut stack = vec![search_root.to_path_buf()];
    let mut roots = Vec::new();
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).with_context(|| {
            format!(
                "reading directory `{}` during release root discovery",
                dir.display()
            )
        })?;
        let mut children = entries
            .map(|entry| entry.map_err(anyhow::Error::from))
            .collect::<Result<Vec<_>>>()?;
        children.sort_by(|a, b| a.path().cmp(&b.path()));
        for entry in children {
            let path = entry.path();
            let file_type = entry
                .file_type()
                .with_context(|| format!("reading file type for `{}`", path.display()))?;
            let name = entry.file_name();
            if file_type.is_file() && name == OsStr::new(STRICT_PROJECT_FILE) {
                roots.push(dir.clone());
                continue;
            }
            if file_type.is_dir() && !is_release_discovery_ignored_dir(name.as_os_str()) {
                stack.push(path);
            }
        }
    }
    Ok(roots)
}

fn is_release_discovery_ignored_dir(name: &OsStr) -> bool {
    matches!(
        name.to_str(),
        Some(".git") | Some(".hg") | Some(".svn") | Some("target")
    )
}

fn resolve_required_manifest_release_value(
    configured_value: &str,
    project_field: &str,
    file: &Path,
    json_errors: bool,
) -> Result<String> {
    let trimmed = configured_value.trim();
    if trimmed.is_empty() || is_release_defaults_placeholder(trimmed) {
        return Err(release_error(
            "C130",
            format!(
                "missing required release value: configure `{}` in `{}` with a non-placeholder value",
                project_field, STRICT_PROJECT_FILE
            ),
            file,
            json_errors,
        ));
    }
    Ok(trimmed.to_string())
}

fn release_paths(out_dir: PathBuf, stem: &str) -> ReleasePaths {
    ReleasePaths {
        module: out_dir.join(format!("{stem}.wasm")),
        strict_import_map: out_dir.join(format!("{stem}.strict-import-map.json")),
        vcs: out_dir.join(format!("{stem}.vc.json")),
        proof: out_dir.join(format!("{stem}.proof.json")),
        signature: out_dir.join(format!("{stem}.sig.json")),
        assurance_manifest: out_dir.join(format!("{stem}.assurance.json")),
        provenance: out_dir.join(format!("{stem}.provenance.json")),
        bundle_manifest: out_dir.join(format!("{stem}.release-bundle.json")),
    }
}

fn write_release_bundle_manifest(
    path: &Path,
    entry: &Path,
    root: &Path,
    advisory_as_of: &str,
    key_id: &str,
    trust_policy: &Path,
    paths: &ReleasePaths,
    shared_std: &[ReleaseSharedStdPackageEvidence],
) -> Result<ReleaseBundleManifestV1> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }

    let manifest = ReleaseBundleManifestV1 {
        schema_version: 1,
        policy_version: "25.2.3".to_string(),
        proof_policy: "release == proved (require_assurance=proved_all)".to_string(),
        primary_commands: vec![
            "check".to_string(),
            "test".to_string(),
            "release".to_string(),
        ],
        entry: entry.display().to_string(),
        root: root.display().to_string(),
        advisory_as_of: advisory_as_of.to_string(),
        key_id: key_id.to_string(),
        trust_policy: trust_policy.display().to_string(),
        shared_std: shared_std.to_vec(),
        artifacts: ReleaseArtifacts {
            module: artifact_file(paths.module.as_path())?,
            strict_import_map: artifact_file(paths.strict_import_map.as_path())?,
            vcs: artifact_file(paths.vcs.as_path())?,
            proof: artifact_file(paths.proof.as_path())?,
            signature: artifact_file(paths.signature.as_path())?,
            assurance_manifest: artifact_file(paths.assurance_manifest.as_path())?,
            provenance: if paths.provenance.exists() {
                Some(artifact_file(paths.provenance.as_path())?)
            } else {
                None
            },
            bundle_manifest: path.display().to_string(),
        },
        orchestration: vec![
            ReleaseStageStatus {
                stage: "lock".to_string(),
                status: "ok".to_string(),
            },
            ReleaseStageStatus {
                stage: "build/prove".to_string(),
                status: "ok".to_string(),
            },
            ReleaseStageStatus {
                stage: "sign".to_string(),
                status: "ok".to_string(),
            },
            ReleaseStageStatus {
                stage: "verify(require-assurance=proved_all)".to_string(),
                status: "ok".to_string(),
            },
            ReleaseStageStatus {
                stage: "bundle".to_string(),
                status: "ok".to_string(),
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

fn write_release_provenance(
    path: &Path,
    signing_key: &Path,
    key_id: &str,
    advisory_as_of: &str,
    entry: &Path,
    root: &Path,
    paths: &ReleasePaths,
    shared_std: &[ReleaseSharedStdPackageEvidence],
) -> Result<()> {
    let payload = serde_json::json!({
        "kind": "clearlang.release_provenance",
        "schema_version": 1,
        "clg_version": env!("CARGO_PKG_VERSION"),
        "advisory_as_of": advisory_as_of,
        "entry": entry.display().to_string(),
        "root": root.display().to_string(),
        "release_signature_key_id": key_id,
        "artifacts": {
            "module_sha256": sha256_hex(&fs::read(paths.module.as_path()).with_context(|| format!("reading {}", paths.module.display()))?),
            "strict_import_map_sha256": sha256_hex(&fs::read(paths.strict_import_map.as_path()).with_context(|| format!("reading {}", paths.strict_import_map.display()))?),
            "vcs_sha256": sha256_hex(&fs::read(paths.vcs.as_path()).with_context(|| format!("reading {}", paths.vcs.display()))?),
            "proof_sha256": sha256_hex(&fs::read(paths.proof.as_path()).with_context(|| format!("reading {}", paths.proof.display()))?),
            "signature_sha256": sha256_hex(&fs::read(paths.signature.as_path()).with_context(|| format!("reading {}", paths.signature.display()))?),
            "assurance_manifest_sha256": sha256_hex(&fs::read(paths.assurance_manifest.as_path()).with_context(|| format!("reading {}", paths.assurance_manifest.display()))?),
        },
        "shared_std": shared_std,
    });
    signing::sign_assurance_manifest(payload, signing_key, key_id, path)
}

fn collect_release_shared_std_package_evidence(
    root: &Path,
) -> Result<Vec<ReleaseSharedStdPackageEvidence>> {
    let Some(shared_std) =
        load_validated_shared_std_lock_section(root).map_err(anyhow::Error::msg)?
    else {
        return Ok(Vec::new());
    };
    Ok(project_shared_std_evidence(&shared_std))
}

fn validate_release_shared_std_mode(
    manifest: &ProjectManifestV1,
    shared_std: &[ReleaseSharedStdPackageEvidence],
) -> std::result::Result<(), String> {
    let Some(std) = manifest.std.as_ref() else {
        if !shared_std.is_empty() {
            return Err(format!(
                "`{}` does not declare shared std, but release lock evidence contains {} shared std package(s)",
                STRICT_PROJECT_FILE,
                shared_std.len()
            ));
        }
        return Ok(());
    };
    if std.delivery == "embedded" {
        if !shared_std.is_empty() {
            return Err(format!(
                "`{}` declares `std.delivery = embedded`, but release lock evidence contains {} shared std package(s)",
                STRICT_PROJECT_FILE,
                shared_std.len()
            ));
        }
        return Ok(());
    }
    if shared_std.is_empty() {
        return Err(format!(
            "release requires non-empty shared std evidence because `{}` declares `std.delivery = shared`; `clg.lock.json` must be schema v2 with populated `std.packages[]`",
            STRICT_PROJECT_FILE
        ));
    }
    let expected = std
        .packages
        .iter()
        .map(|package| package.package_id.as_str())
        .collect::<BTreeSet<_>>();
    let actual = shared_std
        .iter()
        .map(|package| package.package_id.as_str())
        .collect::<BTreeSet<_>>();
    if expected != actual {
        return Err(format!(
            "release shared std evidence does not match `{}` intent; manifest packages = {:?}, lock evidence packages = {:?}",
            STRICT_PROJECT_FILE,
            expected,
            actual
        ));
    }
    Ok(())
}

fn parse_release_shared_std_package_evidence_value(
    value: Option<&JsonValue>,
) -> std::result::Result<Vec<ReleaseSharedStdPackageEvidence>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut entries: Vec<ReleaseSharedStdPackageEvidence> =
        serde_json::from_value(value.clone()).map_err(|err| err.to_string())?;
    entries.sort();
    Ok(entries)
}

fn release_shared_std_parity_violation(
    expected: &[ReleaseSharedStdPackageEvidence],
    actual: &[ReleaseSharedStdPackageEvidence],
) -> Option<String> {
    (expected != actual).then(|| {
        format!(
            "shared std provenance mismatch: bundle manifest records {} entries but provenance payload records {} entries",
            expected.len(),
            actual.len()
        )
    })
}

fn resolve_release_artifact_paths(
    manifest: &ReleaseBundleManifestV1,
    bundle_dir: &Path,
) -> std::result::Result<Vec<(String, PathBuf, String)>, String> {
    let artifacts = &manifest.artifacts;
    let entries = vec![
        (
            "module".to_string(),
            artifacts.module.path.clone(),
            artifacts.module.sha256.clone(),
        ),
        (
            "strict_import_map".to_string(),
            artifacts.strict_import_map.path.clone(),
            artifacts.strict_import_map.sha256.clone(),
        ),
        (
            "vcs".to_string(),
            artifacts.vcs.path.clone(),
            artifacts.vcs.sha256.clone(),
        ),
        (
            "proof".to_string(),
            artifacts.proof.path.clone(),
            artifacts.proof.sha256.clone(),
        ),
        (
            "signature".to_string(),
            artifacts.signature.path.clone(),
            artifacts.signature.sha256.clone(),
        ),
        (
            "assurance_manifest".to_string(),
            artifacts.assurance_manifest.path.clone(),
            artifacts.assurance_manifest.sha256.clone(),
        ),
    ];
    let mut entries = entries;
    if let Some(provenance) = artifacts.provenance.as_ref() {
        entries.push((
            "provenance".to_string(),
            provenance.path.clone(),
            provenance.sha256.clone(),
        ));
    }

    let mut out = Vec::with_capacity(entries.len());
    for (name, raw_path, hash) in entries {
        if raw_path.trim().is_empty() {
            return Err(format!("release bundle artifact `{name}` has empty path"));
        }
        if hash.trim().len() != 64 {
            return Err(format!(
                "release bundle artifact `{name}` has invalid sha256 `{hash}` (expected 64 lowercase hex chars)"
            ));
        }
        let path = resolve_manifest_path(raw_path.as_str(), bundle_dir);
        out.push((name, path, hash));
    }
    Ok(out)
}

fn resolve_manifest_path(raw_path: &str, base_dir: &Path) -> PathBuf {
    let path = PathBuf::from(raw_path);
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

fn verify_release_artifact_hashes(
    artifacts: &[(String, PathBuf, String)],
    bundle_path: &Path,
    json_errors: bool,
) -> Result<()> {
    for (name, path, expected_hash) in artifacts {
        let bytes = fs::read(path).map_err(|err| {
            release_error(
                "C140",
                format!(
                    "release bundle artifact `{name}` is missing/unreadable at `{}`: {err}",
                    path.display()
                ),
                bundle_path,
                json_errors,
            )
        })?;
        let actual_hash = sha256_hex(bytes.as_slice());
        if &actual_hash != expected_hash {
            return Err(release_error(
                "C140",
                format!(
                    "release bundle artifact `{name}` hash mismatch for `{}`: expected {expected_hash}, got {actual_hash}",
                    path.display()
                ),
                bundle_path,
                json_errors,
            ));
        }
    }
    Ok(())
}

fn resolved_path_for(name: &str, artifacts: &[(String, PathBuf, String)]) -> Option<PathBuf> {
    artifacts
        .iter()
        .find_map(|(artifact_name, artifact_path, _)| {
            (artifact_name == name).then(|| artifact_path.clone())
        })
}

fn verify_bundle_provenance(
    manifest: &ReleaseBundleManifestV1,
    bundle_dir: &Path,
    selected_pubkey: &Path,
    signature_path: &Path,
    require_provenance: bool,
    bundle_path: &Path,
    json_errors: bool,
) -> Result<()> {
    const VERIFY_BUNDLE_ERROR_CODE: &str = "C140";
    let Some(provenance_entry) = manifest.artifacts.provenance.as_ref() else {
        if require_provenance {
            return Err(release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                "required provenance artifact is missing from release bundle".to_string(),
                bundle_path,
                json_errors,
            ));
        }
        return Ok(());
    };
    let provenance_path = resolve_manifest_path(provenance_entry.path.as_str(), bundle_dir);
    let provenance = signing::verify_assurance_manifest(provenance_path.as_path(), selected_pubkey)
        .map_err(|err| {
            release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                format!(
                    "provenance signature verification failed for `{}`: {err}",
                    provenance_path.display()
                ),
                bundle_path,
                json_errors,
            )
        })?;
    let payload = provenance.payload.as_object().ok_or_else(|| {
        release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "provenance payload in `{}` must be a JSON object",
                provenance_path.display()
            ),
            bundle_path,
            json_errors,
        )
    })?;
    let kind = payload
        .get("kind")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| {
            release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                format!(
                    "provenance payload in `{}` missing `kind`",
                    provenance_path.display()
                ),
                bundle_path,
                json_errors,
            )
        })?;
    if kind != "clearlang.release_provenance" {
        return Err(release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "provenance payload in `{}` has unsupported kind `{}`",
                provenance_path.display(),
                kind
            ),
            bundle_path,
            json_errors,
        ));
    }
    let schema_version = payload
        .get("schema_version")
        .and_then(JsonValue::as_u64)
        .ok_or_else(|| {
            release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                format!(
                    "provenance payload in `{}` missing `schema_version`",
                    provenance_path.display()
                ),
                bundle_path,
                json_errors,
            )
        })?;
    if schema_version != 1 {
        return Err(release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "provenance payload in `{}` has unsupported schema_version {}",
                provenance_path.display(),
                schema_version
            ),
            bundle_path,
            json_errors,
        ));
    }
    let signature_key_id = payload
        .get("release_signature_key_id")
        .and_then(JsonValue::as_str)
        .unwrap_or("");
    if signature_key_id.trim().is_empty() {
        return Err(release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "provenance payload in `{}` missing non-empty `release_signature_key_id`",
                provenance_path.display()
            ),
            bundle_path,
            json_errors,
        ));
    }
    let release_signature: signing::SignatureFile =
        serde_json::from_slice(&fs::read(signature_path).map_err(|err| {
            release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                format!(
                    "reading signature file `{}`: {err}",
                    signature_path.display()
                ),
                bundle_path,
                json_errors,
            )
        })?)
        .map_err(|err| {
            release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                format!(
                    "parsing signature file `{}`: {err}",
                    signature_path.display()
                ),
                bundle_path,
                json_errors,
            )
        })?;
    if release_signature.key_id != signature_key_id {
        return Err(release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "provenance `release_signature_key_id` ({}) does not match release signature key_id ({})",
                signature_key_id, release_signature.key_id
            ),
            bundle_path,
            json_errors,
        ));
    }
    let provenance_shared_std = parse_release_shared_std_package_evidence_value(
        payload.get("shared_std"),
    )
    .map_err(|err| {
        release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "provenance payload in `{}` has invalid `shared_std` evidence: {err}",
                provenance_path.display()
            ),
            bundle_path,
            json_errors,
        )
    })?;
    if let Some(message) =
        release_shared_std_parity_violation(manifest.shared_std.as_slice(), &provenance_shared_std)
    {
        return Err(release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            message,
            bundle_path,
            json_errors,
        ));
    }
    Ok(())
}

fn verify_bundle_strict_import_map_shared_std_parity(
    manifest: &ReleaseBundleManifestV1,
    bundle_dir: &Path,
    bundle_path: &Path,
    json_errors: bool,
) -> Result<()> {
    const VERIFY_BUNDLE_ERROR_CODE: &str = "C140";
    let strict_import_map_path = resolve_manifest_path(
        manifest.artifacts.strict_import_map.path.as_str(),
        bundle_dir,
    );
    let bytes = fs::read(strict_import_map_path.as_path()).map_err(|err| {
        release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "reading strict import-map `{}` for shared std parity: {err}",
                strict_import_map_path.display()
            ),
            bundle_path,
            json_errors,
        )
    })?;
    let import_map: JsonValue = serde_json::from_slice(bytes.as_slice()).map_err(|err| {
        release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "parsing strict import-map `{}` for shared std parity: {err}",
                strict_import_map_path.display()
            ),
            bundle_path,
            json_errors,
        )
    })?;
    let import_map_shared_std = parse_release_shared_std_package_evidence_value(
        import_map.get("shared_std"),
    )
    .map_err(|err| {
        release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "strict import-map `{}` has invalid `shared_std` evidence: {err}",
                strict_import_map_path.display()
            ),
            bundle_path,
            json_errors,
        )
    })?;
    if let Some(message) =
        release_shared_std_parity_violation(manifest.shared_std.as_slice(), &import_map_shared_std)
    {
        return Err(release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "strict import-map shared std evidence mismatch for `{}`: {message}",
                strict_import_map_path.display()
            ),
            bundle_path,
            json_errors,
        ));
    }
    Ok(())
}

fn resolve_verify_bundle_pubkey(
    pubkey: Option<PathBuf>,
    keyring: Option<PathBuf>,
    signature_path: &Path,
    bundle_path: &Path,
    json_errors: bool,
) -> Result<PathBuf> {
    const VERIFY_BUNDLE_ERROR_CODE: &str = "C140";
    if let Some(path) = pubkey {
        return Ok(path);
    }

    let keyring = keyring.ok_or_else(|| {
        release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            "verify-bundle requires `--pubkey <FILE>` or `--keyring <FILE>`".to_string(),
            bundle_path,
            json_errors,
        )
    })?;
    let keyring_bytes = fs::read(&keyring).map_err(|err| {
        release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!("reading keyring `{}`: {err}", keyring.display()),
            bundle_path,
            json_errors,
        )
    })?;
    let keyring_doc: VerifyBundleKeyringV1 =
        serde_json::from_slice(&keyring_bytes).map_err(|err| {
            release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                format!("parsing keyring `{}`: {err}", keyring.display()),
                bundle_path,
                json_errors,
            )
        })?;
    if keyring_doc.schema_version != 1 {
        return Err(release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "unsupported keyring schema_version {} (expected 1)",
                keyring_doc.schema_version
            ),
            bundle_path,
            json_errors,
        ));
    }

    let mut keys = BTreeMap::<String, String>::new();
    for entry in keyring_doc.keys {
        let key_id = entry.key_id.trim();
        let pubkey_path = entry.pubkey.trim();
        if key_id.is_empty() {
            return Err(release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                "keyring entry has empty key_id".to_string(),
                bundle_path,
                json_errors,
            ));
        }
        if pubkey_path.is_empty() {
            return Err(release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                format!("keyring entry `{key_id}` has empty `pubkey` path"),
                bundle_path,
                json_errors,
            ));
        }
        if keys
            .insert(key_id.to_string(), pubkey_path.to_string())
            .is_some()
        {
            return Err(release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                format!("keyring has duplicate key_id `{key_id}`"),
                bundle_path,
                json_errors,
            ));
        }
    }
    if keys.is_empty() {
        return Err(release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            "keyring must contain at least one key entry".to_string(),
            bundle_path,
            json_errors,
        ));
    }

    let mut revoked = BTreeSet::<String>::new();
    for key_id in keyring_doc.revoked_key_ids {
        let key_id = key_id.trim();
        if key_id.is_empty() {
            return Err(release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                "keyring `revoked_key_ids` contains an empty key_id".to_string(),
                bundle_path,
                json_errors,
            ));
        }
        if !keys.contains_key(key_id) {
            return Err(release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                format!("keyring revoked key_id `{key_id}` is not present in `keys[]`"),
                bundle_path,
                json_errors,
            ));
        }
        if !revoked.insert(key_id.to_string()) {
            return Err(release_error(
                VERIFY_BUNDLE_ERROR_CODE,
                format!("keyring `revoked_key_ids` contains duplicate `{key_id}`"),
                bundle_path,
                json_errors,
            ));
        }
    }

    let sig_bytes = fs::read(signature_path).map_err(|err| {
        release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "reading signature file `{}` for key_id selection: {err}",
                signature_path.display()
            ),
            bundle_path,
            json_errors,
        )
    })?;
    let sig_file: signing::SignatureFile = serde_json::from_slice(&sig_bytes).map_err(|err| {
        release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "parsing signature file `{}` for key_id selection: {err}",
                signature_path.display()
            ),
            bundle_path,
            json_errors,
        )
    })?;
    let signature_key_id = sig_file.key_id.trim();
    if signature_key_id.is_empty() {
        return Err(release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "signature file `{}` has empty key_id",
                signature_path.display()
            ),
            bundle_path,
            json_errors,
        ));
    }
    if revoked.contains(signature_key_id) {
        return Err(release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "signature key_id `{signature_key_id}` is revoked in keyring `{}`",
                keyring.display()
            ),
            bundle_path,
            json_errors,
        ));
    }
    let Some(pubkey_rel) = keys.get(signature_key_id) else {
        return Err(release_error(
            VERIFY_BUNDLE_ERROR_CODE,
            format!(
                "signature key_id `{signature_key_id}` not found in keyring `{}`",
                keyring.display()
            ),
            bundle_path,
            json_errors,
        ));
    };
    let keyring_dir = keyring
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    Ok(resolve_manifest_path(pubkey_rel, keyring_dir.as_path()))
}

fn validate_release_import_map_has_no_test_paths(
    paths: &ReleasePaths,
    file: &Path,
    json_errors: bool,
) -> Result<()> {
    let bytes = fs::read(paths.strict_import_map.as_path()).map_err(|err| {
        release_error(
            "C129",
            format!(
                "release artifact scan failed: reading strict import-map `{}`: {err}",
                paths.strict_import_map.display()
            ),
            file,
            json_errors,
        )
    })?;
    let import_map: JsonValue = serde_json::from_slice(bytes.as_slice()).map_err(|err| {
        release_error(
            "C129",
            format!(
                "release artifact scan failed: parsing strict import-map `{}`: {err}",
                paths.strict_import_map.display()
            ),
            file,
            json_errors,
        )
    })?;
    let source_files = import_map
        .get("source_files")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| {
            release_error(
                "C129",
                format!(
                    "release artifact scan failed: strict import-map `{}` is missing `source_files[]`",
                    paths.strict_import_map.display()
                ),
                file,
                json_errors,
            )
        })?;
    let mut violations = source_files
        .iter()
        .filter_map(JsonValue::as_str)
        .filter(|path| path_has_tests_or_mocks(path))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    violations.sort();
    violations.dedup();
    if violations.is_empty() {
        return Ok(());
    }
    Err(release_error(
        "C129",
        format!(
            "release artifact scan failed: strict import-map references test/mock source paths [{}]",
            violations.join(", ")
        ),
        file,
        json_errors,
    ))
}

fn path_has_tests_or_mocks(path: &str) -> bool {
    let segments = path
        .split(['/', '\\'])
        .filter(|segment| !segment.trim().is_empty())
        .map(|segment| segment.to_ascii_lowercase())
        .collect::<Vec<_>>();
    segments.iter().any(|segment| segment == "tests")
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

#[cfg(test)]
mod tests {
    use super::{
        collect_release_shared_std_package_evidence, discover_manifest_roots,
        is_release_discovery_ignored_dir, parse_release_shared_std_package_evidence_value,
        path_has_tests_or_mocks, release_shared_std_parity_violation,
        validate_release_import_map_has_no_test_paths, ReleasePaths, ReleaseSharedStdAbiClaim,
        ReleaseSharedStdPackageEvidence,
    };
    use crate::commands::release_defaults::STRICT_PROJECT_FILE;
    use serde_json::json;
    use std::ffi::OsStr;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    use crate::commands::shared_std_lock::tests::malformed_shared_std_lockfile_cases;

    fn release_paths_for_import_map(import_map: PathBuf) -> ReleasePaths {
        ReleasePaths {
            module: PathBuf::from("ignored.wasm"),
            strict_import_map: import_map,
            vcs: PathBuf::from("ignored.vc.json"),
            proof: PathBuf::from("ignored.proof.json"),
            signature: PathBuf::from("ignored.sig.json"),
            assurance_manifest: PathBuf::from("ignored.assurance.json"),
            provenance: PathBuf::from("ignored.provenance.json"),
            bundle_manifest: PathBuf::from("ignored.release-bundle.json"),
        }
    }

    #[test]
    fn path_has_tests_or_mocks_detects_tests_segments_cross_platform() {
        assert!(path_has_tests_or_mocks("tests/unit/a.clear"));
        assert!(path_has_tests_or_mocks("src\\tests\\mocks\\a.clear"));
        assert!(path_has_tests_or_mocks("a/TESTS/b.clear"));
        assert!(!path_has_tests_or_mocks("src/domain/a.clear"));
    }

    #[test]
    fn release_artifact_scan_rejects_tests_paths_with_c129() {
        let tmp = tempdir().expect("tempdir");
        let map_path = tmp.path().join("main.strict-import-map.json");
        let payload = json!({
            "schema_version": 1,
            "source_files": [
                "src/main.clear",
                "tests/unit/helper.clear"
            ]
        });
        fs::write(
            &map_path,
            serde_json::to_vec_pretty(&payload).expect("serialize import map"),
        )
        .expect("write import map");

        let paths = release_paths_for_import_map(map_path);
        let err = validate_release_import_map_has_no_test_paths(&paths, tmp.path(), true)
            .expect_err("expected C129 rejection");
        let text = err.to_string();
        assert!(
            text.contains("\"code\": \"C129\""),
            "expected C129, got: {text}"
        );
        assert!(
            text.contains("tests/unit/helper.clear"),
            "expected offending test path evidence, got: {text}"
        );
    }

    #[test]
    fn release_artifact_scan_accepts_production_only_source_files() {
        let tmp = tempdir().expect("tempdir");
        let map_path = tmp.path().join("main.strict-import-map.json");
        let payload = json!({
            "schema_version": 1,
            "source_files": [
                "src/main.clear",
                "src/domain/pricing.clear"
            ]
        });
        fs::write(
            &map_path,
            serde_json::to_vec_pretty(&payload).expect("serialize import map"),
        )
        .expect("write import map");

        let paths = release_paths_for_import_map(map_path);
        validate_release_import_map_has_no_test_paths(&paths, tmp.path(), true)
            .expect("production-only import map should pass C129 scan");
    }

    #[test]
    fn release_discovery_ignores_tooling_dirs() {
        assert!(is_release_discovery_ignored_dir(OsStr::new("target")));
        assert!(is_release_discovery_ignored_dir(OsStr::new(".git")));
        assert!(!is_release_discovery_ignored_dir(OsStr::new("src")));
    }

    #[test]
    fn discover_manifest_roots_finds_nested_project_files() {
        let tmp = tempdir().expect("tempdir");
        let project = tmp.path().join("workspace").join("generic");
        fs::create_dir_all(&project).expect("create project dir");
        fs::write(project.join(STRICT_PROJECT_FILE), b"{}").expect("write manifest");

        let roots = discover_manifest_roots(tmp.path().join("workspace").as_path())
            .expect("discover manifest roots");
        assert_eq!(roots, vec![project]);
    }

    #[test]
    fn collect_release_shared_std_package_evidence_reads_schema_v2_lockfile() {
        let tmp = tempdir().expect("tempdir");
        let lockfile = json!({
            "schema_version": 2,
            "resolver_version": 1,
            "roots": [],
            "packages": [],
            "std": {
                "delivery": "shared",
                "packages": [
                    {
                        "package_id": "std::text",
                        "version": "1.2.0",
                        "verified_std_abi": {
                            "major": 1,
                            "minor_min": 0,
                            "minor_max": 0
                        },
                        "artifact": {
                            "format": "wasm",
                            "path": "std-packages/std-text-1.2.0.wasm",
                            "size_bytes": 4,
                            "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                        },
                        "signature": {
                            "key_id": "std-publisher-ed25519-2026q2",
                            "algorithm": "ed25519",
                            "signed_at": "2026-06-01T00:00:00Z",
                            "signature": "sig"
                        },
                        "provenance": {
                            "statement_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                            "statement_format": "in-toto-v1"
                        },
                        "symbols": [
                            "std::bytes::eq_ct",
                            "std::str::len"
                        ],
                        "dependencies": []
                    }
                ]
            }
        });
        fs::write(
            tmp.path().join("clg.lock.json"),
            serde_json::to_vec_pretty(&lockfile).expect("serialize lockfile"),
        )
        .expect("write lockfile");

        let shared_std =
            collect_release_shared_std_package_evidence(tmp.path()).expect("shared std evidence");
        assert_eq!(
            shared_std,
            vec![ReleaseSharedStdPackageEvidence {
                package_id: "std::text".to_string(),
                version: "1.2.0".to_string(),
                verified_std_abi: ReleaseSharedStdAbiClaim {
                    major: 1,
                    minor_min: 0,
                    minor_max: 0,
                },
                artifact_digest:
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                        .to_string(),
                signature_key_id: "std-publisher-ed25519-2026q2".to_string(),
                provenance_digest:
                    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                        .to_string(),
            }]
        );
    }

    #[test]
    fn release_shared_std_parity_violation_detects_manifest_provenance_mismatch() {
        let manifest = vec![ReleaseSharedStdPackageEvidence {
            package_id: "std::text".to_string(),
            version: "1.2.0".to_string(),
            verified_std_abi: ReleaseSharedStdAbiClaim {
                major: 1,
                minor_min: 0,
                minor_max: 0,
            },
            artifact_digest:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
            signature_key_id: "std-publisher-ed25519-2026q2".to_string(),
            provenance_digest:
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_string(),
        }];
        let provenance = parse_release_shared_std_package_evidence_value(Some(&json!([{
            "package_id": "std::text",
            "version": "1.2.0",
            "verified_std_abi": {
                "major": 1,
                "minor_min": 0,
                "minor_max": 1
            },
            "artifact_digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "signature_key_id": "std-publisher-ed25519-2026q2",
            "provenance_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        }])))
        .expect("parse provenance shared std");
        let message =
            release_shared_std_parity_violation(manifest.as_slice(), provenance.as_slice())
                .expect("parity mismatch");
        assert!(message.contains("shared std provenance mismatch"));
    }

    fn write_shared_std_lockfile(root: &std::path::Path, value: &serde_json::Value) {
        fs::write(
            root.join("clg.lock.json"),
            serde_json::to_vec_pretty(value).expect("serialize shared std lockfile"),
        )
        .expect("write shared std lockfile");
    }

    #[test]
    fn collect_release_shared_std_package_evidence_rejects_malformed_shared_std_cases() {
        for case in malformed_shared_std_lockfile_cases() {
            let tmp = tempdir().expect("tempdir");
            write_shared_std_lockfile(tmp.path(), &case.value);
            let err = collect_release_shared_std_package_evidence(tmp.path())
                .expect_err("malformed shared std release evidence should fail");
            assert!(
                err.to_string().contains(case.expected_substring),
                "{}: expected `{}`, got `{err}`",
                case.label,
                case.expected_substring
            );
        }
    }
}
