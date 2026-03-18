use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::strict_validation::{
    validate_exact_semver, validate_package_id, validate_semver_requirement, validate_sha256_digest,
};

pub(super) const STRICT_LOCKFILE_FILE: &str = "clg.lock.json";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictLockfileV0 {
    pub(super) dependencies: Vec<StrictLockDependency>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictLockDependency {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictLockfileError {
    code: &'static str,
    message: String,
}

impl StrictLockfileError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(super) fn code(&self) -> &'static str {
        self.code
    }

    pub(super) fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictLockfileV0 {
    schema_version: u32,
    dependencies: Vec<RawStrictDependency>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictDependency {
    name: String,
    version: String,
    digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictLockfileV1 {
    schema_version: u32,
    resolver_version: u32,
    roots: Vec<RawStrictRootV1>,
    packages: Vec<RawStrictPackageV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictRootV1 {
    name: String,
    dependencies: Vec<RawStrictRootDependencyV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictRootDependencyV1 {
    name: String,
    requirement: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictPackageV1 {
    id: String,
    name: String,
    version: String,
    digest: String,
    abi_id: String,
    dependencies: Vec<String>,
}

pub(super) fn load_required_strict_lockfile_v0(
    root: &Path,
) -> Result<StrictLockfileV0, StrictLockfileError> {
    let path = root.join(STRICT_LOCKFILE_FILE);
    if !path.exists() {
        return Err(StrictLockfileError::new(
            "C101",
            format!(
                "strict mode requires `{}` at `{}`",
                STRICT_LOCKFILE_FILE,
                path.display()
            ),
        ));
    }
    if !path.is_file() {
        return Err(StrictLockfileError::new(
            "C101",
            format!(
                "strict lockfile path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }

    let content = fs::read_to_string(&path).map_err(|err| {
        StrictLockfileError::new(
            "C101",
            format!("failed to read strict lockfile `{}`: {err}", path.display()),
        )
    })?;

    parse_strict_lockfile_v0(&content, &path)
}

fn parse_strict_lockfile_v0(
    content: &str,
    path: &Path,
) -> Result<StrictLockfileV0, StrictLockfileError> {
    let value: serde_json::Value = serde_json::from_str(content).map_err(|_| {
        StrictLockfileError::new(
            "C104",
            format!(
                "strict lockfile `{}` is not valid JSON (expected schema v0/v1 object)",
                path.display()
            ),
        )
    })?;
    let schema_version = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            StrictLockfileError::new(
                "C104",
                format!(
                    "strict lockfile `{}` does not match schema v0/v1 (`schema_version` required)",
                    path.display()
                ),
            )
        })?;

    match schema_version {
        0 => {
            let raw: RawStrictLockfileV0 = serde_json::from_value(value).map_err(|_| {
                StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` does not match schema v0 (`schema_version`, `dependencies[]` with `name`, `version`, `digest`)",
                        path.display()
                    ),
                )
            })?;
            debug_assert_eq!(raw.schema_version, 0);
            let dependencies: Vec<StrictLockDependency> = raw
                .dependencies
                .into_iter()
                .map(|dep| StrictLockDependency {
                    name: dep.name,
                    version: dep.version,
                    digest: dep.digest,
                })
                .collect();
            validate_lock_dependencies(path, dependencies, false)
        }
        1 => {
            let raw: RawStrictLockfileV1 = serde_json::from_value(value).map_err(|_| {
                StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` does not match schema v1 (`schema_version`, `resolver_version`, `roots[]`, `packages[]`)",
                        path.display()
                    ),
                )
            })?;
            debug_assert_eq!(raw.schema_version, 1);
            if raw.resolver_version != 1 {
                return Err(StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` has unsupported resolver_version {}; expected 1",
                        path.display(),
                        raw.resolver_version
                    ),
                ));
            }

            validate_v1_roots(path, &raw.roots)?;

            let mut package_ids = HashSet::with_capacity(raw.packages.len());
            let mut package_id_dupes = BTreeSet::new();
            let mut dependencies = Vec::with_capacity(raw.packages.len());

            for pkg in &raw.packages {
                if !package_ids.insert(pkg.id.clone()) {
                    package_id_dupes.insert(pkg.id.clone());
                }

                validate_package_id(pkg.name.as_str()).map_err(|msg| {
                    StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` has invalid package name `{}`: {msg}",
                            path.display(),
                            pkg.name
                        ),
                    )
                })?;
                validate_exact_semver(pkg.version.as_str()).map_err(|msg| {
                    StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` has invalid package version `{}` for `{}`: {msg}",
                            path.display(),
                            pkg.version,
                            pkg.name
                        ),
                    )
                })?;
                validate_sha256_digest(pkg.digest.as_str()).map_err(|msg| {
                    StrictLockfileError::new(
                        "C102",
                        format!(
                            "strict lockfile `{}` has invalid package digest `{}` for `{}`: {msg}",
                            path.display(),
                            pkg.digest,
                            pkg.name
                        ),
                    )
                })?;
                if pkg.abi_id.trim().is_empty() {
                    return Err(StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` package `{}` has empty abi_id",
                            path.display(),
                            pkg.name
                        ),
                    ));
                }
                let expected_id = format!("{}@{}", pkg.name, pkg.version);
                if pkg.id != expected_id {
                    return Err(StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` package id `{}` does not match `name@version` (`{}`)",
                            path.display(),
                            pkg.id,
                            expected_id
                        ),
                    ));
                }
                dependencies.push(StrictLockDependency {
                    name: pkg.name.clone(),
                    version: pkg.version.clone(),
                    digest: pkg.digest.clone(),
                });
            }

            if let Some(dupe) = package_id_dupes.iter().next() {
                return Err(StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` has duplicate package id `{}`",
                        path.display(),
                        dupe
                    ),
                ));
            }

            for pkg in &raw.packages {
                let mut seen_deps = HashSet::with_capacity(pkg.dependencies.len());
                let mut dep_dupes = BTreeSet::new();
                for dep in &pkg.dependencies {
                    if dep.trim().is_empty() {
                        return Err(StrictLockfileError::new(
                            "C104",
                            format!(
                                "strict lockfile `{}` package `{}` has empty dependency id",
                                path.display(),
                                pkg.id
                            ),
                        ));
                    }
                    if !seen_deps.insert(dep.clone()) {
                        dep_dupes.insert(dep.clone());
                    }
                    if !package_ids.contains(dep) {
                        return Err(StrictLockfileError::new(
                            "C104",
                            format!(
                                "strict lockfile `{}` package `{}` references dependency id `{}` that is missing from `packages[]`",
                                path.display(),
                                pkg.id,
                                dep
                            ),
                        ));
                    }
                }
                if let Some(dupe) = dep_dupes.iter().next() {
                    return Err(StrictLockfileError::new(
                        "C104",
                        format!(
                            "strict lockfile `{}` package `{}` has duplicate dependency id `{}`",
                            path.display(),
                            pkg.id,
                            dupe
                        ),
                    ));
                }
            }

            validate_lock_dependencies(path, dependencies, true)
        }
        other => Err(StrictLockfileError::new(
            "C104",
            format!(
                "strict lockfile `{}` has unsupported schema_version {}; expected 0 or 1",
                path.display(),
                other
            ),
        )),
    }
}

fn validate_v1_roots(path: &Path, roots: &[RawStrictRootV1]) -> Result<(), StrictLockfileError> {
    let mut seen_root_names = HashSet::with_capacity(roots.len());
    let mut root_name_dupes = BTreeSet::new();
    for root in roots {
        if root.name.trim().is_empty() {
            return Err(StrictLockfileError::new(
                "C104",
                format!(
                    "strict lockfile `{}` has root with empty `name`",
                    path.display()
                ),
            ));
        }
        if !seen_root_names.insert(root.name.clone()) {
            root_name_dupes.insert(root.name.clone());
        }
        let mut seen_dep_names = HashSet::with_capacity(root.dependencies.len());
        let mut dep_name_dupes = BTreeSet::new();
        for dep in &root.dependencies {
            validate_package_id(dep.name.as_str()).map_err(|msg| {
                StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` root `{}` has invalid dependency name `{}`: {msg}",
                        path.display(),
                        root.name,
                        dep.name
                    ),
                )
            })?;
            validate_semver_requirement(dep.requirement.as_str()).map_err(|msg| {
                StrictLockfileError::new(
                    "C104",
                    format!(
                        "strict lockfile `{}` root `{}` dependency `{}` has invalid requirement `{}`: {msg}",
                        path.display(),
                        root.name,
                        dep.name,
                        dep.requirement
                    ),
                )
            })?;
            if !seen_dep_names.insert(dep.name.clone()) {
                dep_name_dupes.insert(dep.name.clone());
            }
        }
        if let Some(dupe) = dep_name_dupes.iter().next() {
            return Err(StrictLockfileError::new(
                "C104",
                format!(
                    "strict lockfile `{}` root `{}` has duplicate dependency name `{}`",
                    path.display(),
                    root.name,
                    dupe
                ),
            ));
        }
    }
    if let Some(dupe) = root_name_dupes.iter().next() {
        return Err(StrictLockfileError::new(
            "C104",
            format!(
                "strict lockfile `{}` has duplicate root name `{}`",
                path.display(),
                dupe
            ),
        ));
    }
    Ok(())
}

fn validate_lock_dependencies(
    path: &Path,
    mut dependencies: Vec<StrictLockDependency>,
    allow_multi_version_per_name: bool,
) -> Result<StrictLockfileV0, StrictLockfileError> {
    let mut seen = HashSet::with_capacity(dependencies.len());
    let mut duplicates = BTreeSet::new();
    for dep in &dependencies {
        let key = if allow_multi_version_per_name {
            format!("{}@{}", dep.name, dep.version)
        } else {
            dep.name.clone()
        };
        if !seen.insert(key.clone()) {
            duplicates.insert(key);
        }
    }
    if let Some(duplicate) = duplicates.iter().next() {
        let kind = if allow_multi_version_per_name {
            "dependency id"
        } else {
            "dependency name"
        };
        return Err(StrictLockfileError::new(
            "C104",
            format!(
                "strict lockfile `{}` has duplicate {kind} `{}`",
                path.display(),
                duplicate
            ),
        ));
    }

    if allow_multi_version_per_name {
        dependencies.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.version.cmp(&b.version)));
    } else {
        dependencies.sort_by(|a, b| a.name.cmp(&b.name));
    }
    for dep in &dependencies {
        validate_package_id(&dep.name).map_err(|msg| {
            StrictLockfileError::new(
                "C104",
                format!(
                    "strict lockfile `{}` has invalid dependency name `{}`: {msg}",
                    path.display(),
                    dep.name
                ),
            )
        })?;
        validate_exact_semver(&dep.version).map_err(|msg| {
            StrictLockfileError::new(
                "C104",
                format!(
                    "strict lockfile `{}` has invalid version `{}` for `{}`: {msg}",
                    path.display(),
                    dep.version,
                    dep.name
                ),
            )
        })?;
        validate_sha256_digest(&dep.digest).map_err(|msg| {
            StrictLockfileError::new(
                "C102",
                format!(
                    "strict lockfile `{}` has invalid digest `{}` for `{}`: {msg}",
                    path.display(),
                    dep.digest,
                    dep.name
                ),
            )
        })?;
    }
    Ok(StrictLockfileV0 { dependencies })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn valid_lockfile_sorts_dependencies_by_name() {
        let json = r#"
{
  "schema_version": 0,
  "dependencies": [
    {"name":"z_pkg","version":"2.0.0","digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
    {"name":"a_pkg","version":"1.0.0","digest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}
  ]
}
        "#;
        let parsed =
            parse_strict_lockfile_v0(json, Path::new("clg.lock.json")).expect("valid lockfile");
        let names: Vec<&str> = parsed
            .dependencies
            .iter()
            .map(|dep| dep.name.as_str())
            .collect();
        assert_eq!(names, vec!["a_pkg", "z_pkg"]);
    }

    #[test]
    fn rejects_unknown_top_level_keys() {
        let json = r#"
{
  "schema_version": 0,
  "dependencies": [],
  "extra": true
}
        "#;
        let err = parse_strict_lockfile_v0(json, Path::new("clg.lock.json"))
            .expect_err("expected schema error");
        assert_eq!(err.code(), "C104");
        assert!(
            err.message().contains("does not match schema v0"),
            "message: {}",
            err.message()
        );
    }

    #[test]
    fn rejects_unknown_dependency_keys() {
        let json = r#"
{
  "schema_version": 0,
  "dependencies": [
    {
      "name":"pkg",
      "version":"1.0.0",
      "digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "note":"x"
    }
  ]
}
        "#;
        let err = parse_strict_lockfile_v0(json, Path::new("clg.lock.json"))
            .expect_err("expected schema error");
        assert_eq!(err.code(), "C104");
        assert!(
            err.message().contains("does not match schema v0"),
            "message: {}",
            err.message()
        );
    }

    #[test]
    fn accepts_schema_v1_lockfile_and_projects_packages_to_dependencies() {
        let json = r#"
{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [
    {
      "name": "app",
      "dependencies": [
        { "name": "std::core", "requirement": "^1.0.0" }
      ]
    }
  ],
  "packages": [
    {
      "id": "std::core@1.0.0",
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:std::core:1.0.0",
      "dependencies": []
    }
  ]
}
        "#;
        let parsed = parse_strict_lockfile_v0(json, Path::new("clg.lock.json"))
            .expect("schema v1 lockfile should parse");
        assert_eq!(parsed.dependencies.len(), 1);
        assert_eq!(parsed.dependencies[0].name, "std::core");
    }

    #[test]
    fn accepts_schema_v1_lockfile_with_same_package_name_at_different_versions() {
        let json = r#"
{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [
    {
      "name": "app",
      "dependencies": [
        { "name": "std::core", "requirement": "^1.0.0" }
      ]
    }
  ],
  "packages": [
    {
      "id": "std::core@1.0.0",
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:std::core:1.0.0",
      "dependencies": []
    },
    {
      "id": "std::core@2.0.0",
      "name": "std::core",
      "version": "2.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:std::core:2.0.0",
      "dependencies": []
    }
  ]
}
        "#;
        let parsed = parse_strict_lockfile_v0(json, Path::new("clg.lock.json"))
            .expect("schema v1 lockfile with multi-version package ids should parse");
        assert_eq!(parsed.dependencies.len(), 2);
        assert_eq!(parsed.dependencies[0].name, "std::core");
        assert_eq!(parsed.dependencies[0].version, "1.0.0");
        assert_eq!(parsed.dependencies[1].name, "std::core");
        assert_eq!(parsed.dependencies[1].version, "2.0.0");
    }

    #[test]
    fn rejects_schema_v1_lockfile_with_invalid_root_requirement() {
        let json = r#"
{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [
    {
      "name": "app",
      "dependencies": [
        { "name": "std::core", "requirement": "latest" }
      ]
    }
  ],
  "packages": [
    {
      "id": "std::core@1.0.0",
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:std::core:1.0.0",
      "dependencies": []
    }
  ]
}
        "#;
        let err = parse_strict_lockfile_v0(json, Path::new("clg.lock.json"))
            .expect_err("invalid root requirement should fail");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("invalid requirement"));
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let json = r#"
{
  "schema_version": 2,
  "dependencies": []
}
        "#;
        let err = parse_strict_lockfile_v0(json, Path::new("clg.lock.json"))
            .expect_err("expected schema version error");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("expected 0 or 1"));
    }

    #[test]
    fn duplicate_detection_reports_lexicographically_first_conflict() {
        let json = r#"
{
  "schema_version": 0,
  "dependencies": [
    {"name":"z_pkg","version":"1.0.0","digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
    {"name":"a_pkg","version":"1.0.0","digest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"},
    {"name":"z_pkg","version":"1.0.1","digest":"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"},
    {"name":"a_pkg","version":"1.0.1","digest":"sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"}
  ]
}
        "#;
        let err = parse_strict_lockfile_v0(json, Path::new("clg.lock.json"))
            .expect_err("expected duplicate-name error");
        assert_eq!(err.code(), "C104");
        assert!(
            err.message().contains("duplicate dependency name `a_pkg`"),
            "message: {}",
            err.message()
        );
    }

    #[test]
    fn rejects_non_exact_semver_versions() {
        let json = r#"
{
  "schema_version": 0,
  "dependencies": [
    {"name":"pkg","version":"^1.0.0","digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}
  ]
}
        "#;
        let err = parse_strict_lockfile_v0(json, Path::new("clg.lock.json"))
            .expect_err("expected version error");
        assert_eq!(err.code(), "C104");
        assert!(
            err.message().contains("exact MAJOR.MINOR.PATCH"),
            "message: {}",
            err.message()
        );
    }

    #[test]
    fn rejects_non_lowercase_sha256_digest() {
        let json = r#"
{
  "schema_version": 0,
  "dependencies": [
    {"name":"pkg","version":"1.0.0","digest":"sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}
  ]
}
        "#;
        let err = parse_strict_lockfile_v0(json, Path::new("clg.lock.json"))
            .expect_err("expected digest error");
        assert_eq!(err.code(), "C102");
        assert!(err.message().contains("lowercase hex"));
    }

    #[test]
    fn load_required_rejects_missing_lockfile() {
        let tmp = tempdir().expect("tempdir");
        let err = load_required_strict_lockfile_v0(tmp.path()).expect_err("missing lockfile");
        assert_eq!(err.code(), "C101");
        assert!(err.message().contains("strict mode requires"));
    }
}
