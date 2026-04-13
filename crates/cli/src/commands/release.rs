use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::commands::build::{self, CompilerMode, ReleaseProfile, StdCoreLinkMode};
use crate::commands::helpers::{make_single_json_error, sha256_hex, CommandError};
use crate::commands::pkg;
use crate::commands::release_defaults::{
    enforce_project_clg_version_compatibility, is_release_defaults_placeholder,
    load_project_manifest_v1, load_required_release_defaults_v0, load_verify_trust_policy_v1,
    STRICT_PROJECT_FILE,
};
use crate::commands::verify::{self, VerifyMode};
use crate::logging::{Logger, StageTimings};
use crate::signing::SignScope;

#[derive(Debug, Clone)]
struct ReleasePaths {
    module: PathBuf,
    strict_import_map: PathBuf,
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
    strict_import_map: ReleaseArtifactFile,
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

pub fn run(
    key: PathBuf,
    pubkey: PathBuf,
    root: Option<PathBuf>,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    let root = resolve_release_root(root, json_errors)?;
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
                "release requires schema v1 `{}` with `project.entry` configured",
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
        let _stage = timings.start(logger, "release_artifact_scan");
        validate_release_import_map_has_no_test_paths(&paths, file.as_path(), json_errors)?;
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
            strict_import_map: artifact_file(paths.strict_import_map.as_path())?,
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
        discover_manifest_roots, is_release_discovery_ignored_dir, path_has_tests_or_mocks,
        validate_release_import_map_has_no_test_paths, ReleasePaths,
    };
    use crate::commands::release_defaults::STRICT_PROJECT_FILE;
    use serde_json::json;
    use std::ffi::OsStr;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn release_paths_for_import_map(import_map: PathBuf) -> ReleasePaths {
        ReleasePaths {
            module: PathBuf::from("ignored.wasm"),
            strict_import_map: import_map,
            vcs: PathBuf::from("ignored.vc.json"),
            proof: PathBuf::from("ignored.proof.json"),
            signature: PathBuf::from("ignored.sig.json"),
            assurance_manifest: PathBuf::from("ignored.assurance.json"),
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
}
