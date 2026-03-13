use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

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
struct RawStrictLockfile {
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

pub(super) fn load_strict_lockfile_v0_if_present(
    root: &Path,
) -> Result<Option<StrictLockfileV0>, StrictLockfileError> {
    let path = root.join(STRICT_LOCKFILE_FILE);
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path).map_err(|err| {
        StrictLockfileError::new(
            "C101",
            format!("failed to read strict lockfile `{}`: {err}", path.display()),
        )
    })?;

    parse_strict_lockfile_v0(&content, &path).map(Some)
}

fn parse_strict_lockfile_v0(
    content: &str,
    path: &PathBuf,
) -> Result<StrictLockfileV0, StrictLockfileError> {
    let raw: RawStrictLockfile = serde_json::from_str(content).map_err(|err| {
        StrictLockfileError::new(
            "C104",
            format!(
                "strict lockfile `{}` is not valid schema v0 JSON: {err}",
                path.display()
            ),
        )
    })?;

    if raw.schema_version != 0 {
        return Err(StrictLockfileError::new(
            "C104",
            format!(
                "strict lockfile `{}` has unsupported schema_version {}; expected 0",
                path.display(),
                raw.schema_version
            ),
        ));
    }

    let mut seen = HashSet::with_capacity(raw.dependencies.len());
    let mut duplicates = BTreeSet::new();
    for dep in &raw.dependencies {
        if !seen.insert(dep.name.clone()) {
            duplicates.insert(dep.name.clone());
        }
    }
    if let Some(duplicate) = duplicates.iter().next() {
        return Err(StrictLockfileError::new(
            "C104",
            format!(
                "strict lockfile `{}` has duplicate dependency name `{}`",
                path.display(),
                duplicate
            ),
        ));
    }

    let mut dependencies: Vec<StrictLockDependency> = raw
        .dependencies
        .into_iter()
        .map(|dep| StrictLockDependency {
            name: dep.name,
            version: dep.version,
            digest: dep.digest,
        })
        .collect();
    dependencies.sort_by(|a, b| a.name.cmp(&b.name));

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
        let parsed = parse_strict_lockfile_v0(json, &PathBuf::from("clg.lock.json"))
            .expect("valid lockfile");
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
        let err = parse_strict_lockfile_v0(json, &PathBuf::from("clg.lock.json"))
            .expect_err("expected schema error");
        assert_eq!(err.code(), "C104");
        assert!(
            err.message().contains("unknown field `extra`"),
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
        let err = parse_strict_lockfile_v0(json, &PathBuf::from("clg.lock.json"))
            .expect_err("expected schema error");
        assert_eq!(err.code(), "C104");
        assert!(
            err.message().contains("unknown field `note`"),
            "message: {}",
            err.message()
        );
    }

    #[test]
    fn rejects_non_zero_schema_version() {
        let json = r#"
{
  "schema_version": 1,
  "dependencies": []
}
        "#;
        let err = parse_strict_lockfile_v0(json, &PathBuf::from("clg.lock.json"))
            .expect_err("expected schema version error");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("expected 0"));
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
        let err = parse_strict_lockfile_v0(json, &PathBuf::from("clg.lock.json"))
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
        let err = parse_strict_lockfile_v0(json, &PathBuf::from("clg.lock.json"))
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
        let err = parse_strict_lockfile_v0(json, &PathBuf::from("clg.lock.json"))
            .expect_err("expected digest error");
        assert_eq!(err.code(), "C102");
        assert!(err.message().contains("lowercase hex"));
    }

    #[test]
    fn load_if_present_returns_none_when_lockfile_is_missing() {
        let tmp = tempdir().expect("tempdir");
        let loaded =
            load_strict_lockfile_v0_if_present(tmp.path()).expect("missing lockfile is allowed");
        assert!(loaded.is_none());
    }
}
