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
    fn accepts_schema_v2_lockfile_and_projects_packages_to_dependencies() {
        let json = r#"
{
  "schema_version": 2,
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
  ],
  "std": {
    "delivery": "shared",
    "packages": [
      {
        "package_id": "std::text",
        "version": "1.2.0",
        "verified_std_abi": { "major": 1, "minor_min": 0, "minor_max": 0 },
        "artifact": {
          "format": "wasm",
          "path": "std-packages/std-text-1.2.0.wasm",
          "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
          "size_bytes": 4
        },
        "signature": {
          "key_id": "k1",
          "algorithm": "ed25519",
          "signed_at": "2026-06-01T00:00:00Z",
          "signature": "abcd"
        },
        "provenance": {
          "statement_digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
          "statement_format": "in-toto-v1"
        },
        "symbols": ["std::bytes", "std::str", "std::str_pattern"],
        "dependencies": []
      }
    ]
  }
}
        "#;
        let parsed = parse_strict_lockfile_v0(json, Path::new("clg.lock.json"))
            .expect("schema v2 lockfile should parse");
        assert_eq!(parsed.dependencies.len(), 2);
        assert_eq!(parsed.dependencies[0].name, "std::core");
        assert_eq!(parsed.dependencies[1].name, "std::text");
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
  "schema_version": 3,
  "dependencies": []
}
        "#;
        let err = parse_strict_lockfile_v0(json, Path::new("clg.lock.json"))
            .expect_err("expected schema version error");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("expected 0, 1, or 2"));
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
