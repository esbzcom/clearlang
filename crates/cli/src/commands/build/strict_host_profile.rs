use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::commands::modules::host_capability_policy::is_known_host_capability;

pub(super) const STRICT_HOST_PROFILE_FILE: &str = "clg.host-profile.json";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictHostProfileV0 {
    pub(super) profile: String,
    pub(super) capabilities: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictHostProfileError {
    code: &'static str,
    message: String,
}

impl StrictHostProfileError {
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
struct RawHostProfileV0 {
    schema_version: u32,
    profile: String,
    capabilities: Vec<String>,
}

pub(super) fn load_required_host_profile_v0(
    root: &Path,
) -> Result<StrictHostProfileV0, StrictHostProfileError> {
    let path = root.join(STRICT_HOST_PROFILE_FILE);
    if !path.exists() {
        return Err(StrictHostProfileError::new(
            "C106",
            format!(
                "strict mode requires `{}` at `{}`",
                STRICT_HOST_PROFILE_FILE,
                path.display()
            ),
        ));
    }
    if !path.is_file() {
        return Err(StrictHostProfileError::new(
            "C106",
            format!(
                "strict host profile path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }
    let content = fs::read_to_string(&path).map_err(|err| {
        StrictHostProfileError::new(
            "C106",
            format!(
                "failed to read strict host profile `{}`: {err}",
                path.display()
            ),
        )
    })?;
    parse_host_profile_v0(&content, &path)
}

fn parse_host_profile_v0(
    content: &str,
    path: &Path,
) -> Result<StrictHostProfileV0, StrictHostProfileError> {
    let value: serde_json::Value = serde_json::from_str(content).map_err(|_| {
        StrictHostProfileError::new(
            "C106",
            format!(
                "strict host profile `{}` is not valid JSON (expected schema v0 object)",
                path.display()
            ),
        )
    })?;
    let raw: RawHostProfileV0 = serde_json::from_value(value).map_err(|_| {
        StrictHostProfileError::new(
            "C106",
            format!(
                "strict host profile `{}` does not match schema v0 (`schema_version`, `profile`, `capabilities`)",
                path.display()
            ),
        )
    })?;
    if raw.schema_version != 0 {
        return Err(StrictHostProfileError::new(
            "C106",
            format!(
                "strict host profile `{}` has unsupported schema_version {}; expected 0",
                path.display(),
                raw.schema_version
            ),
        ));
    }
    match raw.profile.as_str() {
        "contract_static" | "shared_app" => {}
        _ => {
            return Err(StrictHostProfileError::new(
                "C106",
                format!(
                    "strict host profile `{}` has unsupported profile `{}`; expected `contract_static` or `shared_app`",
                    path.display(),
                    raw.profile
                ),
            ));
        }
    }

    let mut capabilities = raw.capabilities;
    capabilities.sort();
    let mut seen = HashSet::with_capacity(capabilities.len());
    let mut duplicates = BTreeSet::new();
    for capability in &capabilities {
        if !seen.insert(capability.clone()) {
            duplicates.insert(capability.clone());
        }
    }
    if let Some(duplicate) = duplicates.iter().next() {
        return Err(StrictHostProfileError::new(
            "C106",
            format!(
                "strict host profile `{}` has duplicate capability `{}`",
                path.display(),
                duplicate
            ),
        ));
    }

    for capability in &capabilities {
        if capability.trim().is_empty() {
            return Err(StrictHostProfileError::new(
                "C106",
                format!(
                    "strict host profile `{}` contains an empty capability id",
                    path.display()
                ),
            ));
        }
        if !is_known_host_capability(capability) {
            return Err(StrictHostProfileError::new(
                "C106",
                format!(
                    "strict host profile `{}` has unsupported capability `{}`",
                    path.display(),
                    capability
                ),
            ));
        }
    }

    Ok(StrictHostProfileV0 {
        profile: raw.profile,
        capabilities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn valid_host_profile_sorts_capabilities() {
        let json = r#"
{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": [
    "std::wasi::print",
    "std::crypto::hash"
  ]
}
        "#;
        let parsed = parse_host_profile_v0(json, Path::new("clg.host-profile.json"))
            .expect("valid host profile");
        assert_eq!(
            parsed.capabilities,
            vec![
                "std::crypto::hash".to_string(),
                "std::wasi::print".to_string()
            ]
        );
    }

    #[test]
    fn missing_file_fails_with_c106() {
        let tmp = tempdir().expect("tempdir");
        let err = load_required_host_profile_v0(tmp.path()).expect_err("expected missing-file err");
        assert_eq!(err.code(), "C106");
        assert!(err.message().contains("strict mode requires"));
    }

    #[test]
    fn malformed_schema_reports_c106() {
        let json = r#"
{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": [],
  "extra": true
}
        "#;
        let err = parse_host_profile_v0(json, Path::new("clg.host-profile.json"))
            .expect_err("expected schema error");
        assert_eq!(err.code(), "C106");
        assert!(err.message().contains("does not match schema v0"));
    }

    #[test]
    fn duplicate_capability_reports_lexicographically_first() {
        let json = r#"
{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": [
    "std::wasi::print",
    "std::crypto::hash",
    "std::wasi::print",
    "std::crypto::hash"
  ]
}
        "#;
        let err = parse_host_profile_v0(json, Path::new("clg.host-profile.json"))
            .expect_err("expected duplicate capability");
        assert_eq!(err.code(), "C106");
        assert!(err.message().contains("std::crypto::hash"));
    }

    #[test]
    fn unsupported_profile_fails_with_c106() {
        let json = r#"
{
  "schema_version": 0,
  "profile": "dev_local",
  "capabilities": []
}
        "#;
        let err = parse_host_profile_v0(json, Path::new("clg.host-profile.json"))
            .expect_err("expected unsupported profile");
        assert_eq!(err.code(), "C106");
        assert!(err.message().contains("unsupported profile"));
    }

    #[test]
    fn unknown_capability_fails_with_c106() {
        let json = r#"
{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::env::unknown"]
}
        "#;
        let err = parse_host_profile_v0(json, Path::new("clg.host-profile.json"))
            .expect_err("expected unknown capability");
        assert_eq!(err.code(), "C106");
        assert!(err.message().contains("unsupported capability"));
    }

    #[test]
    fn env_time_capability_is_allowed_in_contract_static_schema() {
        let json = r#"
{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::env::time"]
}
        "#;
        let parsed = parse_host_profile_v0(json, Path::new("clg.host-profile.json"))
            .expect("env_time should be accepted by host-profile schema");
        assert_eq!(parsed.profile, "contract_static");
        assert_eq!(parsed.capabilities, vec!["std::env::time".to_string()]);
    }

    #[test]
    fn env_random_capability_is_allowed_in_shared_app_schema() {
        let json = r#"
{
  "schema_version": 0,
  "profile": "shared_app",
  "capabilities": ["std::env::random"]
}
        "#;
        let parsed = parse_host_profile_v0(json, Path::new("clg.host-profile.json"))
            .expect("env_random should be accepted by host-profile schema");
        assert_eq!(parsed.profile, "shared_app");
        assert_eq!(parsed.capabilities, vec!["std::env::random".to_string()]);
    }
}
