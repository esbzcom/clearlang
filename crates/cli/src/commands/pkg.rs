use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::commands::helpers::{make_single_json_error, CommandError};
use crate::logging::{Logger, StageTimings};

const CANONICAL_PACKAGE_METADATA_FILE: &str = "clg.package-metadata.json";
const STRICT_LOCKFILE_FILE: &str = "clg.lock.json";

#[derive(Deserialize)]
struct PackageMetadataRoot {
    schema_version: u32,
    #[serde(default)]
    packages: Vec<PackageMetadataEntry>,
}

#[derive(Deserialize)]
struct PackageMetadataEntry {
    name: String,
    version: String,
    digest: String,
}

#[derive(Debug, Serialize)]
struct StrictLockfileV0 {
    schema_version: u32,
    dependencies: Vec<StrictLockDependency>,
}

#[derive(Debug, Serialize)]
struct StrictLockDependency {
    name: String,
    version: String,
    digest: String,
}

pub fn run_lock(
    generate: bool,
    update: bool,
    root: PathBuf,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    let fail_pkg = |code: &'static str, message: String| -> Result<()> {
        if json_errors {
            let json = make_single_json_error(code, "build", message, &root, 0, 0, None);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(message))
        }
    };

    if generate == update {
        return fail_pkg(
            "C027",
            "pkg lock requires exactly one of `--generate` or `--update`".to_string(),
        );
    }

    let mut timings = StageTimings::new();
    let metadata_path = root.join(CANONICAL_PACKAGE_METADATA_FILE);
    let lockfile_path = root.join(STRICT_LOCKFILE_FILE);

    if generate && lockfile_path.exists() {
        return fail_pkg(
            "C027",
            format!(
                "lockfile `{}` already exists; use `clg pkg lock --update`",
                lockfile_path.display()
            ),
        );
    }
    if update && !lockfile_path.exists() {
        return fail_pkg(
            "C027",
            format!(
                "lockfile `{}` does not exist; use `clg pkg lock --generate`",
                lockfile_path.display()
            ),
        );
    }

    let lockfile = {
        let _stage = timings.start(logger, "pkg_load_metadata");
        match load_lockfile_from_metadata(metadata_path.as_path()) {
            Ok(value) => value,
            Err(err) => return fail_pkg("C027", err.to_string()),
        }
    };

    {
        let _stage = timings.start(logger, "pkg_write_lockfile");
        write_lockfile(lockfile_path.as_path(), &lockfile)?;
    }

    println!(
        "wrote {} with {} pinned package(s)",
        lockfile_path.display(),
        lockfile.dependencies.len()
    );
    logger.summary(&timings);
    Ok(())
}

fn load_lockfile_from_metadata(path: &Path) -> Result<StrictLockfileV0> {
    if !path.exists() {
        anyhow::bail!("canonical package metadata `{}` is missing", path.display());
    }
    if !path.is_file() {
        anyhow::bail!(
            "canonical package metadata path `{}` exists but is not a file",
            path.display()
        );
    }

    let content =
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let raw: PackageMetadataRoot = serde_json::from_str(content.as_str())
        .with_context(|| format!("parsing {}", path.display()))?;
    if !matches!(raw.schema_version, 0 | 1) {
        anyhow::bail!(
            "unsupported package metadata schema_version {} in `{}`; expected 0 or 1",
            raw.schema_version,
            path.display()
        );
    }

    let mut seen = HashSet::with_capacity(raw.packages.len());
    let mut duplicates = BTreeSet::new();
    let mut dependencies = Vec::with_capacity(raw.packages.len());
    for pkg in raw.packages {
        validate_package_id(pkg.name.as_str())
            .map_err(|msg| anyhow!("invalid package name `{}`: {msg}", pkg.name))?;
        validate_exact_semver(pkg.version.as_str()).map_err(|msg| {
            anyhow!(
                "invalid package version `{}` for `{}`: {msg}",
                pkg.version,
                pkg.name
            )
        })?;
        validate_sha256_digest(pkg.digest.as_str())
            .map_err(|msg| anyhow!("invalid digest `{}` for `{}`: {msg}", pkg.digest, pkg.name))?;
        if !seen.insert(pkg.name.clone()) {
            duplicates.insert(pkg.name);
        } else {
            dependencies.push(StrictLockDependency {
                name: pkg.name,
                version: pkg.version,
                digest: pkg.digest,
            });
        }
    }
    if let Some(first) = duplicates.iter().next() {
        anyhow::bail!(
            "duplicate package name `{}` in canonical package metadata",
            first
        );
    }

    dependencies.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(StrictLockfileV0 {
        schema_version: 0,
        dependencies,
    })
}

fn write_lockfile(path: &Path, lockfile: &StrictLockfileV0) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    let mut bytes = serde_json::to_vec_pretty(lockfile).context("serializing strict lockfile")?;
    bytes.push(b'\n');
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn validate_package_id(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("name is empty".to_string());
    }
    for segment in name.split("::") {
        validate_identifier_segment(segment)?;
    }
    Ok(())
}

fn validate_identifier_segment(segment: &str) -> Result<(), String> {
    if segment.is_empty() {
        return Err("contains empty `::` segment".to_string());
    }
    let mut chars = segment.chars();
    let first = chars.next().expect("segment non-empty");
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(format!(
            "segment `{segment}` must start with ASCII letter or `_`"
        ));
    }
    for ch in chars {
        if !(ch == '_' || ch.is_ascii_alphanumeric()) {
            return Err(format!(
                "segment `{segment}` contains invalid character `{ch}`"
            ));
        }
    }
    Ok(())
}

fn validate_exact_semver(version: &str) -> Result<(), String> {
    if version.contains('-') || version.contains('+') {
        return Err(
            "must use exact MAJOR.MINOR.PATCH without pre-release/build metadata".to_string(),
        );
    }
    if !version.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        return Err("must use exact MAJOR.MINOR.PATCH".to_string());
    }
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return Err("must use exact MAJOR.MINOR.PATCH".to_string());
    }
    for part in parts {
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("version segment `{part}` is not numeric"));
        }
    }
    Ok(())
}

fn validate_sha256_digest(digest: &str) -> Result<(), String> {
    const PREFIX: &str = "sha256:";
    if !digest.starts_with(PREFIX) {
        return Err("digest must start with `sha256:`".to_string());
    }
    let hex = &digest[PREFIX.len()..];
    if hex.len() != 64 {
        return Err("digest must have exactly 64 lowercase hex characters".to_string());
    }
    if !hex
        .chars()
        .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return Err("digest must be lowercase hex (`0-9`, `a-f`)".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfile_from_metadata_sorts_and_pins() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "z::pkg",
      "version": "2.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/z.wasm" },
      "abi_id": "abi:z"
    },
    {
      "name": "a::pkg",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/a.wasm" },
      "abi_id": "abi:a"
    }
  ]
}"#,
        )
        .expect("write metadata");
        let lockfile = load_lockfile_from_metadata(metadata_path.as_path())
            .expect("load lockfile from metadata");
        assert_eq!(lockfile.schema_version, 0);
        assert_eq!(lockfile.dependencies.len(), 2);
        assert_eq!(lockfile.dependencies[0].name, "a::pkg");
        assert_eq!(lockfile.dependencies[1].name, "z::pkg");
    }

    #[test]
    fn lockfile_from_metadata_rejects_duplicate_names() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "dup",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    {
      "name": "dup",
      "version": "2.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    }
  ]
}"#,
        )
        .expect("write metadata");
        let err = load_lockfile_from_metadata(metadata_path.as_path())
            .expect_err("expected duplicate error");
        assert!(err.to_string().contains("duplicate package name `dup`"));
    }
}
