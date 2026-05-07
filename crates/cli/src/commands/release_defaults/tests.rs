use std::fs;
use std::path::Path;

use super::*;

#[test]
fn release_project_template_is_schema_v1() {
    let value: serde_json::Value =
        serde_json::from_slice(&release_project_template_pretty_json()).expect("json");
    assert_eq!(
        value.get("schema_version").and_then(|v| v.as_u64()),
        Some(1)
    );
    assert!(value.get("project").is_some());
    assert!(value.get("dependencies").is_some());
    assert!(value.get("release_defaults").is_some());
    assert_eq!(
        value
            .get("project")
            .and_then(|v| v.get("clg_version"))
            .and_then(|v| v.as_str()),
        Some(default_project_clg_version_requirement().as_str())
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
    let err = parse_project_manifest(content, Path::new("clg.project.json"))
        .expect_err("expected traversal rejection");
    assert!(err.message().contains("must not contain `..`"));
}

#[test]
fn parse_release_defaults_accepts_schema_v1_manifest() {
    let content = r#"{
  "schema_version": 1,
  "project": {
    "name": "example-app",
    "description": "Example project",
    "version": "1.2.3",
    "clg_version": "^0.1.0",
    "entry": "main.clear",
    "website": "https://example.com",
    "contact": {
      "name": "Example Maintainer",
      "email": "maintainer@example.com"
    }
  },
  "dependencies": [
    { "name": "std::core", "requirement": "^1.0.0" },
    { "name": "std::host", "requirement": "^1.0.0" }
  ],
  "release_defaults": {
    "advisory_as_of": "2026-03-31T00:00:00Z",
    "key_id": "release-2026q2",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}"#;
    let parsed = parse_project_manifest(content, Path::new("clg.project.json")).expect("schema v1");
    assert_eq!(
        parsed.release_defaults,
        ReleaseDefaultsV0 {
            advisory_as_of: "2026-03-31T00:00:00Z".to_string(),
            key_id: "release-2026q2".to_string(),
            out_dir: "out/release".to_string(),
            trust_policy: "trust-policy.json".to_string(),
        }
    );
    let manifest = parsed.manifest_v1.expect("manifest");
    assert_eq!(manifest.project.name, "example-app");
    assert_eq!(manifest.project.version, "1.2.3");
    assert_eq!(manifest.project.entry, "main.clear");
    assert_eq!(manifest.dependencies.len(), 2);
    assert_eq!(manifest.dependencies[0].name, "std::core");
    assert_eq!(manifest.dependencies[1].name, "std::host");
}

#[test]
fn load_project_manifest_v1_rejects_duplicate_dependencies() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join(STRICT_PROJECT_FILE),
        r#"{
  "schema_version": 1,
  "project": {
    "name": "example-app",
    "description": "Example project",
    "version": "1.2.3",
    "clg_version": "^0.1.0",
    "entry": "main.clear",
    "website": "https://example.com",
    "contact": {
      "name": "Example Maintainer",
      "email": "maintainer@example.com"
    }
  },
  "dependencies": [
    { "name": "std::core", "requirement": "^1.0.0" },
    { "name": "std::core", "requirement": "^1.0.1" }
  ],
  "release_defaults": {
    "advisory_as_of": "2026-03-31T00:00:00Z",
    "key_id": "release-2026q2",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}"#,
    )
    .expect("write manifest");
    let err = load_project_manifest_v1(dir.path()).expect_err("duplicate dependencies");
    assert!(err.message().contains("duplicate dependency `std::core`"));
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

#[test]
fn clg_version_compatibility_accepts_matching_requirement() {
    let content = r#"{
  "schema_version": 1,
  "project": {
    "name": "example-app",
    "description": "Example project",
    "version": "1.2.3",
    "clg_version": "__CLG_VERSION_REQ__",
    "entry": "main.clear",
    "website": "https://example.com",
    "contact": {
      "name": "Example Maintainer",
      "email": "maintainer@example.com"
    }
  },
  "dependencies": [],
  "release_defaults": {
    "advisory_as_of": "2026-03-31T00:00:00Z",
    "key_id": "release-2026q2",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}"#
    .replace(
        "__CLG_VERSION_REQ__",
        default_project_clg_version_requirement().as_str(),
    );
    let parsed =
        parse_project_manifest(content.as_str(), Path::new(STRICT_PROJECT_FILE)).expect("manifest");
    let manifest = parsed.manifest_v1.expect("schema v1 manifest");
    enforce_project_clg_version_compatibility(&manifest).expect("version should match");
}

#[test]
fn clg_version_compatibility_rejects_non_matching_requirement() {
    let content = r#"{
  "schema_version": 1,
  "project": {
    "name": "example-app",
    "description": "Example project",
    "version": "1.2.3",
    "clg_version": "^999.0.0",
    "entry": "main.clear",
    "website": "https://example.com",
    "contact": {
      "name": "Example Maintainer",
      "email": "maintainer@example.com"
    }
  },
  "dependencies": [],
  "release_defaults": {
    "advisory_as_of": "2026-03-31T00:00:00Z",
    "key_id": "release-2026q2",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}"#;
    let parsed = parse_project_manifest(content, Path::new(STRICT_PROJECT_FILE)).expect("manifest");
    let manifest = parsed.manifest_v1.expect("schema v1 manifest");
    let err = enforce_project_clg_version_compatibility(&manifest)
        .expect_err("version requirement should fail");
    assert!(err.message().contains("project.clg_version"));
    assert!(err.message().contains("current `clg` version"));
}

#[test]
fn default_project_clg_version_requirement_matches_current_clg_version() {
    let requirement = default_project_clg_version_requirement();
    let current_core = normalized_clg_core_version();
    assert!(
        semver_requirement_matches_version(requirement.as_str(), current_core)
            .expect("valid default requirement"),
        "default requirement `{}` should match current clg version `{}`",
        requirement,
        current_core
    );
}
