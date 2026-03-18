use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::commands::helpers::{
    canonical_json_bytes, make_single_json_error, sha256_hex, CommandError,
};
use crate::commands::validation::{
    validate_exact_semver, validate_package_id, validate_semver_requirement, validate_sha256_digest,
};
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
    abi_id: String,
    #[serde(default)]
    dependencies: Vec<PackageRequirementEntry>,
}

#[derive(Clone, Deserialize)]
struct PackageRequirementEntry {
    name: String,
    requirement: String,
}

#[derive(Clone)]
struct ValidatedPackage {
    name: String,
    version: String,
    digest: String,
    abi_id: String,
    dependencies: Vec<PackageRequirementEntry>,
}

#[derive(Debug, Serialize)]
struct StrictLockfileV1 {
    schema_version: u32,
    resolver_version: u32,
    roots: Vec<StrictLockRootV1>,
    packages: Vec<StrictLockedPackageV1>,
}

#[derive(Debug, Serialize)]
struct StrictLockRootV1 {
    name: String,
    dependencies: Vec<StrictLockRootDependencyV1>,
}

#[derive(Debug, Serialize)]
struct StrictLockRootDependencyV1 {
    name: String,
    requirement: String,
}

#[derive(Debug, Serialize)]
struct StrictLockedPackageV1 {
    id: String,
    name: String,
    version: String,
    digest: String,
    abi_id: String,
    dependencies: Vec<String>,
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

    let canonical_hash = {
        let _stage = timings.start(logger, "pkg_write_lockfile");
        write_lockfile(lockfile_path.as_path(), &lockfile)?
    };

    println!(
        "wrote {} with {} pinned package(s) [sha256:{}]",
        lockfile_path.display(),
        lockfile.packages.len(),
        canonical_hash
    );
    logger.summary(&timings);
    Ok(())
}

fn load_lockfile_from_metadata(path: &Path) -> Result<StrictLockfileV1> {
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

    let mut seen_names = HashSet::with_capacity(raw.packages.len());
    let mut duplicate_names = BTreeSet::new();
    let mut packages = Vec::with_capacity(raw.packages.len());
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
        if pkg.abi_id.trim().is_empty() {
            anyhow::bail!("invalid abi_id for `{}`: abi_id is empty", pkg.name);
        }
        if !seen_names.insert(pkg.name.clone()) {
            duplicate_names.insert(pkg.name);
            continue;
        }

        let mut dependency_seen = HashSet::with_capacity(pkg.dependencies.len());
        let mut dependency_dupes = BTreeSet::new();
        for dep in &pkg.dependencies {
            validate_package_id(dep.name.as_str()).map_err(|msg| {
                anyhow!(
                    "invalid dependency name `{}` for package `{}`: {msg}",
                    dep.name,
                    pkg.name
                )
            })?;
            validate_semver_requirement(dep.requirement.as_str()).map_err(|msg| {
                anyhow!(
                    "invalid dependency requirement `{}` for package `{}` dependency `{}`: {msg}",
                    dep.requirement,
                    pkg.name,
                    dep.name
                )
            })?;
            if !dependency_seen.insert(dep.name.clone()) {
                dependency_dupes.insert(dep.name.clone());
            }
        }
        if let Some(first) = dependency_dupes.iter().next() {
            anyhow::bail!(
                "duplicate dependency name `{}` in package `{}` in canonical package metadata",
                first,
                pkg.name
            );
        }

        let mut dependencies = pkg.dependencies;
        dependencies.sort_by(|a, b| a.name.cmp(&b.name));
        packages.push(ValidatedPackage {
            name: pkg.name,
            version: pkg.version,
            digest: pkg.digest,
            abi_id: pkg.abi_id,
            dependencies,
        });
    }
    if let Some(first) = duplicate_names.iter().next() {
        anyhow::bail!(
            "duplicate package name `{}` in canonical package metadata",
            first
        );
    }

    packages.sort_by(|a, b| a.name.cmp(&b.name));
    let versions_by_name: HashMap<&str, &str> = packages
        .iter()
        .map(|pkg| (pkg.name.as_str(), pkg.version.as_str()))
        .collect();

    let mut locked_packages = Vec::with_capacity(packages.len());
    for pkg in &packages {
        let mut dependency_ids = Vec::with_capacity(pkg.dependencies.len());
        for dep in &pkg.dependencies {
            let version = versions_by_name.get(dep.name.as_str()).ok_or_else(|| {
                anyhow!(
                    "package `{}` references dependency `{}` which is not present in canonical package metadata",
                    pkg.name,
                    dep.name
                )
            })?;
            dependency_ids.push(format!("{}@{}", dep.name, version));
        }
        dependency_ids.sort();
        locked_packages.push(StrictLockedPackageV1 {
            id: format!("{}@{}", pkg.name, pkg.version),
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            digest: pkg.digest.clone(),
            abi_id: pkg.abi_id.clone(),
            dependencies: dependency_ids,
        });
    }
    locked_packages.sort_by(|a, b| a.id.cmp(&b.id));

    let mut referenced = HashSet::new();
    for pkg in &packages {
        for dep in &pkg.dependencies {
            referenced.insert(dep.name.as_str());
        }
    }
    let mut root_packages: Vec<&ValidatedPackage> = packages
        .iter()
        .filter(|pkg| !referenced.contains(pkg.name.as_str()))
        .collect();
    // If all packages are in a cycle, fall back to deterministic full root coverage.
    if root_packages.is_empty() {
        root_packages = packages.iter().collect();
    }
    root_packages.sort_by(|a, b| a.name.cmp(&b.name));
    let root_dependencies: Vec<StrictLockRootDependencyV1> = root_packages
        .into_iter()
        .map(|pkg| StrictLockRootDependencyV1 {
            name: pkg.name.clone(),
            requirement: format!("={}", pkg.version),
        })
        .collect();
    let roots = vec![StrictLockRootV1 {
        name: "app".to_string(),
        dependencies: root_dependencies,
    }];

    Ok(StrictLockfileV1 {
        schema_version: 1,
        resolver_version: 1,
        roots,
        packages: locked_packages,
    })
}

fn write_lockfile(path: &Path, lockfile: &StrictLockfileV1) -> Result<String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    let mut bytes = canonical_lockfile_bytes(lockfile)?;
    let canonical_hash = sha256_hex(bytes.as_slice());
    bytes.push(b'\n');
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(canonical_hash)
}

fn canonical_lockfile_bytes(lockfile: &StrictLockfileV1) -> Result<Vec<u8>> {
    let value = serde_json::to_value(lockfile).context("serializing strict lockfile value")?;
    Ok(canonical_json_bytes(&value))
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
      "abi_id": "abi:z::pkg:2.0.0"
    },
    {
      "name": "a::pkg",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/a.wasm" },
      "abi_id": "abi:a::pkg:1.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");
        let lockfile = load_lockfile_from_metadata(metadata_path.as_path())
            .expect("load lockfile from metadata");
        assert_eq!(lockfile.schema_version, 1);
        assert_eq!(lockfile.resolver_version, 1);
        assert_eq!(lockfile.packages.len(), 2);
        assert_eq!(lockfile.packages[0].id, "a::pkg@1.0.0");
        assert_eq!(lockfile.packages[1].id, "z::pkg@2.0.0");
        assert_eq!(lockfile.roots.len(), 1);
        assert_eq!(lockfile.roots[0].name, "app");
        assert_eq!(lockfile.roots[0].dependencies[0].name, "a::pkg");
    }

    #[test]
    fn lockfile_from_metadata_derives_roots_from_unreferenced_packages() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [
        { "name": "lib::core", "requirement": "^1.0.0" }
      ]
    },
    {
      "name": "lib::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:lib::core:1.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");

        let lockfile = load_lockfile_from_metadata(metadata_path.as_path())
            .expect("load lockfile from metadata");
        assert_eq!(lockfile.roots.len(), 1);
        assert_eq!(lockfile.roots[0].dependencies.len(), 1);
        assert_eq!(lockfile.roots[0].dependencies[0].name, "app::entry");
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
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:dup:1.0.0"
    },
    {
      "name": "dup",
      "version": "2.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:dup:2.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");
        let err = load_lockfile_from_metadata(metadata_path.as_path())
            .expect_err("expected duplicate error");
        assert!(err.to_string().contains("duplicate package name `dup`"));
    }

    #[test]
    fn lockfile_from_metadata_rejects_unknown_dependency() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::pkg",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::pkg:1.0.0",
      "dependencies": [
        { "name": "missing::pkg", "requirement": "^1.0.0" }
      ]
    }
  ]
}"#,
        )
        .expect("write metadata");
        let err = load_lockfile_from_metadata(metadata_path.as_path())
            .expect_err("expected unknown dependency");
        assert!(err
            .to_string()
            .contains("not present in canonical package metadata"));
    }

    #[test]
    fn canonical_lockfile_bytes_are_stable_and_hashed() {
        let lockfile = StrictLockfileV1 {
            schema_version: 1,
            resolver_version: 1,
            roots: vec![StrictLockRootV1 {
                name: "app".to_string(),
                dependencies: vec![StrictLockRootDependencyV1 {
                    name: "std::core".to_string(),
                    requirement: "=1.0.0".to_string(),
                }],
            }],
            packages: vec![StrictLockedPackageV1 {
                id: "std::core@1.0.0".to_string(),
                name: "std::core".to_string(),
                version: "1.0.0".to_string(),
                digest: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
                abi_id: "abi:std::core:1.0.0".to_string(),
                dependencies: Vec::new(),
            }],
        };
        let bytes_a = canonical_lockfile_bytes(&lockfile).expect("canonical bytes");
        let bytes_b = canonical_lockfile_bytes(&lockfile).expect("canonical bytes");
        assert_eq!(bytes_a, bytes_b);
        assert_eq!(
            String::from_utf8(bytes_a.clone()).expect("utf8"),
            "{\"packages\":[{\"abi_id\":\"abi:std::core:1.0.0\",\"dependencies\":[],\"digest\":\"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"id\":\"std::core@1.0.0\",\"name\":\"std::core\",\"version\":\"1.0.0\"}],\"resolver_version\":1,\"roots\":[{\"dependencies\":[{\"name\":\"std::core\",\"requirement\":\"=1.0.0\"}],\"name\":\"app\"}],\"schema_version\":1}"
        );
        let hash = sha256_hex(bytes_a.as_slice());
        assert_eq!(hash.len(), 64);
        assert!(hash
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()));
    }

    #[test]
    fn write_lockfile_appends_newline_and_returns_canonical_hash() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join(STRICT_LOCKFILE_FILE);
        let lockfile = StrictLockfileV1 {
            schema_version: 1,
            resolver_version: 1,
            roots: vec![StrictLockRootV1 {
                name: "app".to_string(),
                dependencies: vec![StrictLockRootDependencyV1 {
                    name: "std::host".to_string(),
                    requirement: "=1.0.0".to_string(),
                }],
            }],
            packages: vec![StrictLockedPackageV1 {
                id: "std::host@1.0.0".to_string(),
                name: "std::host".to_string(),
                version: "1.0.0".to_string(),
                digest: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_string(),
                abi_id: "abi:std::host:1.0.0".to_string(),
                dependencies: Vec::new(),
            }],
        };
        let hash = write_lockfile(path.as_path(), &lockfile).expect("write lockfile");
        let written = fs::read(path).expect("read lockfile");
        assert_eq!(written.last().copied(), Some(b'\n'));
        let canonical = &written[..written.len() - 1];
        assert_eq!(sha256_hex(canonical), hash);
    }
}
