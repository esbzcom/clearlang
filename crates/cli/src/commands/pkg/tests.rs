#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::release_defaults::{
        ProjectContactV1, ProjectMetadataV1, ProjectStdPackageRequirementV2,
    };

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
        let lockfile = load_lockfile_from_metadata(metadata_path.as_path(), None, None)
            .expect("load lockfile from metadata");
        assert_eq!(lockfile.schema_version(), 1);
        assert_eq!(lockfile.resolver_version(), 1);
        assert_eq!(lockfile.packages().len(), 2);
        assert_eq!(lockfile.packages()[0].id, "a::pkg@1.0.0");
        assert_eq!(lockfile.packages()[1].id, "z::pkg@2.0.0");
        assert_eq!(lockfile.roots().len(), 1);
        assert_eq!(lockfile.roots()[0].name, "app");
        assert_eq!(lockfile.roots()[0].dependencies[0].name, "a::pkg");
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
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [
        { "name": "lib::core", "requirement": "^1.0.0" }
      ]
    },
    {
      "name": "lib::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:lib::core:1.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");

        let lockfile = load_lockfile_from_metadata(metadata_path.as_path(), None, None)
            .expect("load lockfile from metadata");
        assert_eq!(lockfile.roots().len(), 1);
        assert_eq!(lockfile.roots()[0].dependencies.len(), 1);
        assert_eq!(lockfile.roots()[0].dependencies[0].name, "app::entry");
    }

    #[test]
    fn lockfile_from_metadata_rejects_duplicate_package_ids() {
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
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:dup:1.0.0"
    },
    {
      "name": "dup",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:dup-alt:1.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");
        let err = load_lockfile_from_metadata(metadata_path.as_path(), None, None)
            .expect_err("expected duplicate error");
        assert!(err.to_string().contains("duplicate package id `dup@1.0.0`"));
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
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:app::pkg:1.0.0",
      "dependencies": [
        { "name": "missing::pkg", "requirement": "^1.0.0" }
      ]
    }
  ]
}"#,
        )
        .expect("write metadata");
        let err = load_lockfile_from_metadata(metadata_path.as_path(), None, None)
            .expect_err("expected unknown dependency");
        assert!(err
            .to_string()
            .contains("not present in canonical package metadata"));
    }

    #[test]
    fn lockfile_from_metadata_solver_picks_highest_satisfying_transitive_version() {
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
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:lib::core:1.0.0"
    },
    {
      "name": "lib::core",
      "version": "1.2.0",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:lib::core:1.2.0"
    },
    {
      "name": "lib::core",
      "version": "2.0.0",
      "digest": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:lib::core:2.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");

        let lockfile = load_lockfile_from_metadata(metadata_path.as_path(), None, None)
            .expect("solver should resolve");
        let mut ids: Vec<String> = lockfile.packages().iter().map(|p| p.id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["app::entry@1.0.0", "lib::core@1.2.0"]);
    }

    #[test]
    fn lockfile_from_metadata_reports_c113_for_unsatisfiable_semver_constraints() {
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
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^2.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.2.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:lib::core:1.2.0"
    }
  ]
}"#,
        )
        .expect("write metadata");

        let err = load_lockfile_from_metadata(metadata_path.as_path(), None, None)
            .expect_err("unsatisfiable constraints should fail");
        assert_eq!(err.code(), "C113");
        assert!(err.to_string().contains("no satisfiable version set"));
    }

    #[test]
    fn lockfile_from_metadata_reports_c115_for_deny_advisory() {
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
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.1.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:lib::core:1.1.0"
    }
  ]
}"#,
        )
        .expect("write metadata");
        fs::write(
            tmp.path().join(ADVISORY_FILE),
            r#"{
  "schema_version": 1,
  "advisories": [
    {
      "id": "ADV-001",
      "package": "lib::core",
      "affected": "^1.0.0",
      "severity": "high",
      "action": "deny",
      "issued_at": "2026-01-01T00:00:00Z",
      "expires_at": "2027-01-01T00:00:00Z"
    }
  ]
}"#,
        )
        .expect("write advisories");

        let err = load_lockfile_from_metadata(metadata_path.as_path(), None, None)
            .expect_err("deny advisory should fail");
        assert_eq!(err.code(), "C115");
        assert!(err.to_string().contains("ADV-001"));
    }

    #[test]
    fn lockfile_from_metadata_reports_c116_for_force_upgrade_without_safe_candidate() {
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
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:lib::core:1.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");
        fs::write(
            tmp.path().join(ADVISORY_FILE),
            r#"{
  "schema_version": 1,
  "advisories": [
    {
      "id": "ADV-002",
      "package": "lib::core",
      "affected": "^1.0.0",
      "severity": "critical",
      "action": "force_upgrade",
      "minimum_safe_version": "2.0.0",
      "issued_at": "2026-01-01T00:00:00Z",
      "expires_at": "2027-01-01T00:00:00Z"
    }
  ]
}"#,
        )
        .expect("write advisories");

        let err = load_lockfile_from_metadata(metadata_path.as_path(), None, None)
            .expect_err("force-upgrade advisory should fail");
        assert_eq!(err.code(), "C116");
        assert!(err.to_string().contains("ADV-002"));
    }

    #[test]
    fn lockfile_from_metadata_force_upgrade_selects_safe_candidate_when_available() {
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
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "~1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:lib::core:1.0.0"
    },
    {
      "name": "lib::core",
      "version": "1.0.5",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:lib::core:1.0.5"
    }
  ]
}"#,
        )
        .expect("write metadata");
        fs::write(
            tmp.path().join(ADVISORY_FILE),
            r#"{
  "schema_version": 1,
  "advisories": [
    {
      "id": "ADV-003",
      "package": "lib::core",
      "affected": "^1.0.0",
      "severity": "high",
      "action": "force_upgrade",
      "minimum_safe_version": "1.0.5",
      "issued_at": "2026-01-01T00:00:00Z",
      "expires_at": "2027-01-01T00:00:00Z"
    }
  ]
}"#,
        )
        .expect("write advisories");

        let lockfile = load_lockfile_from_metadata(metadata_path.as_path(), None, None)
            .expect("force-upgrade should resolve to safe candidate");
        let mut ids: Vec<String> = lockfile.packages().iter().map(|p| p.id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["app::entry@1.0.0", "lib::core@1.0.5"]);
    }

    #[test]
    fn lockfile_from_metadata_reports_c117_for_invalid_advisory_input() {
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
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:app::entry:1.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");
        fs::write(
            tmp.path().join(ADVISORY_FILE),
            r#"{"schema_version":1,"advisories":[{"id":"ADV-BAD"}]}"#,
        )
        .expect("write advisories");

        let err = load_lockfile_from_metadata(metadata_path.as_path(), None, None)
            .expect_err("invalid advisory schema should fail");
        assert_eq!(err.code(), "C117");
    }

    #[test]
    fn canonical_lockfile_bytes_are_stable_and_hashed() {
        let lockfile = StrictLockfile::V1(StrictLockfileV1 {
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
        });
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
        let lockfile = StrictLockfile::V1(StrictLockfileV1 {
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
        });
        let hash = write_lockfile(path.as_path(), &lockfile).expect("write lockfile");
        let written = fs::read(path).expect("read lockfile");
        assert_eq!(written.last().copied(), Some(b'\n'));
        let canonical = &written[..written.len() - 1];
        assert_eq!(sha256_hex(canonical), hash);
    }

    #[test]
    fn canonical_resolved_graph_bytes_are_stable_and_hashed() {
        let lockfile = StrictLockfile::V1(StrictLockfileV1 {
            schema_version: 1,
            resolver_version: 1,
            roots: vec![StrictLockRootV1 {
                name: "app".to_string(),
                dependencies: vec![StrictLockRootDependencyV1 {
                    name: "std::core".to_string(),
                    requirement: "^1.0.0".to_string(),
                }],
            }],
            packages: vec![
                StrictLockedPackageV1 {
                    id: "std::host@1.0.0".to_string(),
                    name: "std::host".to_string(),
                    version: "1.0.0".to_string(),
                    digest:
                        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                            .to_string(),
                    abi_id: "abi:std::host:1.0.0".to_string(),
                    dependencies: Vec::new(),
                },
                StrictLockedPackageV1 {
                    id: "std::core@1.0.0".to_string(),
                    name: "std::core".to_string(),
                    version: "1.0.0".to_string(),
                    digest:
                        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    abi_id: "abi:std::core:1.0.0".to_string(),
                    dependencies: vec!["std::host@1.0.0".to_string()],
                },
            ],
        });
        let bytes_a = canonical_resolved_graph_bytes(&lockfile).expect("resolved graph bytes");
        let bytes_b = canonical_resolved_graph_bytes(&lockfile).expect("resolved graph bytes");
        assert_eq!(bytes_a, bytes_b);
        let hash = sha256_hex(bytes_a.as_slice());
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn write_resolved_graph_artifact_writes_bytes_and_hash_sidecar() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lockfile = StrictLockfile::V1(StrictLockfileV1 {
            schema_version: 1,
            resolver_version: 1,
            roots: vec![StrictLockRootV1 {
                name: "app".to_string(),
                dependencies: vec![StrictLockRootDependencyV1 {
                    name: "pkg::a".to_string(),
                    requirement: "=1.0.0".to_string(),
                }],
            }],
            packages: vec![StrictLockedPackageV1 {
                id: "pkg::a@1.0.0".to_string(),
                name: "pkg::a".to_string(),
                version: "1.0.0".to_string(),
                digest: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
                abi_id: "abi:pkg::a:1.0.0".to_string(),
                dependencies: Vec::new(),
            }],
        });
        let hash = write_resolved_graph_artifact(tmp.path(), &lockfile)
            .expect("write resolved graph artifact");
        let graph_bytes = fs::read(tmp.path().join(RESOLVED_GRAPH_FILE)).expect("read graph bytes");
        let graph_canonical = &graph_bytes[..graph_bytes.len() - 1];
        assert_eq!(sha256_hex(graph_canonical), hash);
        let hash_text = fs::read_to_string(tmp.path().join(RESOLVED_GRAPH_HASH_FILE))
            .expect("read graph hash sidecar");
        assert_eq!(hash_text.trim(), hash);
    }

    #[test]
    fn lockfile_from_metadata_reports_c112_for_transitive_cycle() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "z::pkg",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:z::pkg:1.0.0",
      "dependencies": [{ "name": "y::pkg", "requirement": "^1.0.0" }]
    },
    {
      "name": "y::pkg",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:y::pkg:1.0.0",
      "dependencies": [{ "name": "x::pkg", "requirement": "^1.0.0" }]
    },
    {
      "name": "x::pkg",
      "version": "1.0.0",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "artifact": { "format": "wasm", "path": "store/pkg.wasm" },
      "abi_id": "abi:x::pkg:1.0.0",
      "dependencies": [{ "name": "z::pkg", "requirement": "^1.0.0" }]
    }
  ]
}"#,
        )
        .expect("write metadata");

        let err = load_lockfile_from_metadata(metadata_path.as_path(), None, None)
            .expect_err("expected transitive cycle diagnostics");
        assert_eq!(err.code(), "C112");
        assert!(
            err.to_string()
                .contains("x::pkg@1.0.0 -> z::pkg@1.0.0 -> y::pkg@1.0.0 -> x::pkg@1.0.0"),
            "message: {}",
            err
        );
    }

    #[test]
    fn lockfile_from_metadata_emits_schema_v2_shared_std_section_for_shared_manifest() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        let store_dir = tmp.path().join("std-packages");
        fs::create_dir_all(&store_dir).expect("create std store");
        fs::write(store_dir.join("std-text-1.2.0.wasm"), b"wasm").expect("write artifact");
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "std::text",
      "version": "1.2.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "std-packages/std-text-1.2.0.wasm" },
      "abi_id": "abi:std::text:1.2.0",
      "signature": {
        "format": "ed25519",
        "key_id": "std-publisher-ed25519-2026q2",
        "signed_at": "2026-06-01T00:00:00Z",
        "signature": "deadbeef"
      },
      "trust": {
        "trusted_anchor_ids": ["std-root"]
      },
      "verified_std_abi": {
        "major": 1,
        "minor_min": 0,
        "minor_max": 0
      },
      "provenance": {
        "statement_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "statement_format": "in-toto-v1"
      }
    }
  ]
}"#,
        )
        .expect("write metadata");
        let manifest = ProjectManifestV1 {
            project: ProjectMetadataV1 {
                name: "example-app".to_string(),
                description: "Example project".to_string(),
                version: "0.1.0".to_string(),
                clg_version: "^0.1.0".to_string(),
                entry: "main.clear".to_string(),
                website: "https://example.com".to_string(),
                contact: ProjectContactV1 {
                    name: "Example Maintainer".to_string(),
                    email: "maintainer@example.com".to_string(),
                },
            },
            dependencies: Vec::new(),
            std: Some(ProjectStdConfigV2 {
                delivery: "shared".to_string(),
                packages: vec![ProjectStdPackageRequirementV2 {
                    package_id: "std::text".to_string(),
                    version_requirement: "^1.2.0".to_string(),
                    verified_std_abi: ProjectStdAbiRequirementV2 {
                        major: 1,
                        minor_min: 0,
                        minor_max: 0,
                    },
                    registry: Some("default".to_string()),
                    signer_policy: Some("std-publisher-prod".to_string()),
                    allow_compat_shims: false,
                }],
            }),
        };

        let lockfile = load_lockfile_from_metadata(
            metadata_path.as_path(),
            Some(vec![StrictLockRootV1 {
                name: "example-app".to_string(),
                dependencies: Vec::new(),
            }]),
            Some(&manifest),
        )
        .expect("shared std lockfile should resolve");
        let StrictLockfile::V2(lockfile) = lockfile else {
            panic!("shared std manifest must emit schema v2 lockfile");
        };
        assert_eq!(lockfile.std.delivery, "shared");
        assert_eq!(lockfile.std.packages.len(), 1);
        assert_eq!(lockfile.std.packages[0].package_id, "std::text");
        assert_eq!(lockfile.std.packages[0].artifact.size_bytes, 4);
        assert!(
            lockfile.std.packages[0]
                .symbols
                .contains(&"std::str::len".to_string())
        );
        assert!(
            lockfile.std.packages[0]
                .symbols
                .contains(&"std::bytes::eq_ct".to_string())
        );
    }
}
