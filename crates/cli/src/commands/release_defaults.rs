use std::fs;
use std::path::{Component, Path};

use serde::Deserialize;

pub(crate) const STRICT_PROJECT_FILE: &str = "clg.project.json";
pub(crate) const RELEASE_DEFAULT_ADVISORY_PLACEHOLDER: &str = "REQUIRED_RFC3339_UTC";
pub(crate) const RELEASE_DEFAULT_KEY_ID_PLACEHOLDER: &str = "REQUIRED_KEY_ID";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseDefaultsV0 {
    pub(crate) advisory_as_of: String,
    pub(crate) key_id: String,
    pub(crate) out_dir: String,
    pub(crate) trust_policy: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerifyTrustAnchorsV1 {
    pub(crate) lean_checker: String,
    pub(crate) coq_checker: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseDefaultsError {
    message: String,
}

impl ReleaseDefaultsError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectRootV0 {
    schema_version: u32,
    release_defaults: RawReleaseDefaultsV0,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReleaseDefaultsV0 {
    advisory_as_of: String,
    key_id: String,
    out_dir: String,
    trust_policy: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVerifyTrustPolicyV1 {
    schema_version: u32,
    trust_anchors: RawVerifyTrustAnchors,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVerifyTrustAnchors {
    lean_checker: String,
    coq_checker: String,
}

pub(crate) fn release_project_template_pretty_json() -> Vec<u8> {
    serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": 0,
        "release_defaults": {
            "advisory_as_of": RELEASE_DEFAULT_ADVISORY_PLACEHOLDER,
            "key_id": RELEASE_DEFAULT_KEY_ID_PLACEHOLDER,
            "out_dir": "out/release",
            "trust_policy": "trust-policy.json",
        }
    }))
    .expect("serialize strict project template")
}

pub(crate) fn load_required_release_defaults_v0(
    root: &Path,
) -> Result<ReleaseDefaultsV0, ReleaseDefaultsError> {
    let path = root.join(STRICT_PROJECT_FILE);
    if !path.exists() {
        return Err(ReleaseDefaultsError::new(format!(
            "strict init/release defaults require `{}` at `{}`",
            STRICT_PROJECT_FILE,
            path.display()
        )));
    }
    if !path.is_file() {
        return Err(ReleaseDefaultsError::new(format!(
            "release defaults path `{}` exists but is not a file",
            path.display()
        )));
    }
    let content = fs::read_to_string(&path)
        .map_err(|err| ReleaseDefaultsError::new(format!("reading {}: {err}", path.display())))?;
    parse_release_defaults_v0(content.as_str(), &path)
}

pub(crate) fn load_optional_release_defaults_v0(
    root: &Path,
) -> Result<Option<ReleaseDefaultsV0>, ReleaseDefaultsError> {
    let path = root.join(STRICT_PROJECT_FILE);
    if !path.exists() {
        return Ok(None);
    }
    load_required_release_defaults_v0(root).map(Some)
}

fn parse_release_defaults_v0(
    content: &str,
    path: &Path,
) -> Result<ReleaseDefaultsV0, ReleaseDefaultsError> {
    let raw: RawProjectRootV0 = serde_json::from_str(content).map_err(|err| {
        ReleaseDefaultsError::new(format!(
            "release defaults `{}` are not valid schema v0 JSON: {err}",
            path.display()
        ))
    })?;
    if raw.schema_version != 0 {
        return Err(ReleaseDefaultsError::new(format!(
            "release defaults `{}` has unsupported schema_version {}; expected 0",
            path.display(),
            raw.schema_version
        )));
    }
    let defaults = raw.release_defaults;
    validate_non_empty(
        path,
        "release_defaults.advisory_as_of",
        defaults.advisory_as_of.as_str(),
    )?;
    validate_non_empty(path, "release_defaults.key_id", defaults.key_id.as_str())?;
    validate_project_relative_path(path, "release_defaults.out_dir", defaults.out_dir.as_str())?;
    validate_project_relative_path(
        path,
        "release_defaults.trust_policy",
        defaults.trust_policy.as_str(),
    )?;

    Ok(ReleaseDefaultsV0 {
        advisory_as_of: defaults.advisory_as_of,
        key_id: defaults.key_id,
        out_dir: defaults.out_dir,
        trust_policy: defaults.trust_policy,
    })
}

pub(crate) fn load_verify_trust_policy_v1(
    path: &Path,
) -> Result<VerifyTrustAnchorsV1, ReleaseDefaultsError> {
    let bytes = fs::read(path)
        .map_err(|err| ReleaseDefaultsError::new(format!("reading {}: {err}", path.display())))?;
    let policy: RawVerifyTrustPolicyV1 =
        serde_json::from_slice(bytes.as_slice()).map_err(|err| {
            ReleaseDefaultsError::new(format!("parsing trust policy `{}`: {err}", path.display()))
        })?;
    if policy.schema_version != 1 {
        return Err(ReleaseDefaultsError::new(format!(
            "unsupported trust policy schema_version {} (expected 1) in {}",
            policy.schema_version,
            path.display()
        )));
    }
    if policy.trust_anchors.lean_checker.trim().is_empty()
        || policy.trust_anchors.coq_checker.trim().is_empty()
    {
        return Err(ReleaseDefaultsError::new(format!(
            "trust policy `{}` must provide non-empty trust_anchors.lean_checker and trust_anchors.coq_checker",
            path.display()
        )));
    }
    Ok(VerifyTrustAnchorsV1 {
        lean_checker: policy.trust_anchors.lean_checker,
        coq_checker: policy.trust_anchors.coq_checker,
    })
}

pub(crate) fn is_release_defaults_placeholder(value: &str) -> bool {
    value == RELEASE_DEFAULT_ADVISORY_PLACEHOLDER || value == RELEASE_DEFAULT_KEY_ID_PLACEHOLDER
}

fn validate_non_empty(path: &Path, field: &str, value: &str) -> Result<(), ReleaseDefaultsError> {
    if value.trim().is_empty() {
        return Err(ReleaseDefaultsError::new(format!(
            "release defaults `{}` field `{}` must be non-empty",
            path.display(),
            field
        )));
    }
    Ok(())
}

fn validate_project_relative_path(
    path: &Path,
    field: &str,
    value: &str,
) -> Result<(), ReleaseDefaultsError> {
    validate_non_empty(path, field, value)?;
    let relative = Path::new(value);
    if relative.is_absolute() {
        return Err(ReleaseDefaultsError::new(format!(
            "release defaults `{}` field `{}` must be a relative path",
            path.display(),
            field
        )));
    }
    for component in relative.components() {
        match component {
            Component::ParentDir => {
                return Err(ReleaseDefaultsError::new(format!(
                    "release defaults `{}` field `{}` must not contain `..`",
                    path.display(),
                    field
                )));
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(ReleaseDefaultsError::new(format!(
                    "release defaults `{}` field `{}` must be a relative path",
                    path.display(),
                    field
                )));
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn release_project_template_is_schema_v0() {
        let value: serde_json::Value =
            serde_json::from_slice(&release_project_template_pretty_json()).expect("json");
        assert_eq!(
            value.get("schema_version").and_then(|v| v.as_u64()),
            Some(0)
        );
    }

    #[test]
    fn parse_release_defaults_rejects_parent_traversal() {
        let content = r#"{
  "schema_version": 0,
  "release_defaults": {
    "advisory_as_of": "2026-03-31T00:00:00Z",
    "key_id": "release-2026q2",
    "out_dir": "../out/release",
    "trust_policy": "trust-policy.json"
  }
}"#;
        let err = parse_release_defaults_v0(content, Path::new("clg.project.json"))
            .expect_err("expected traversal rejection");
        assert!(err.message().contains("must not contain `..`"));
    }

    #[test]
    fn load_verify_trust_policy_v1_rejects_empty_anchors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("trust-policy.json");
        fs::write(
            &path,
            r#"{
  "schema_version": 1,
  "trust_anchors": {
    "lean_checker": "",
    "coq_checker": "8.19.2"
  }
}"#,
        )
        .expect("write trust policy");
        let err = load_verify_trust_policy_v1(path.as_path()).expect_err("expected anchor error");
        assert!(err.message().contains("must provide non-empty"));
    }
}
