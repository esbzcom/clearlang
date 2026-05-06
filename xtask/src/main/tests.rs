#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_std_core_args_uses_defaults() {
        let opts = parse_std_core_args(Vec::new()).expect("parse args");
        assert_eq!(opts.version, "1.0.0");
        assert!(opts.out_dir.is_none());
    }

    #[test]
    fn parse_std_core_args_accepts_version_and_out_dir() {
        let opts = parse_std_core_args(vec![
            "--version".to_string(),
            "2.3.4".to_string(),
            "--out-dir".to_string(),
            "tmp/std-core".to_string(),
        ])
        .expect("parse args");
        assert_eq!(opts.version, "2.3.4");
        assert_eq!(opts.out_dir, Some(PathBuf::from("tmp/std-core")));
    }

    #[test]
    fn parse_std_core_args_rejects_invalid_version() {
        let err = parse_std_core_args(vec!["--version".to_string(), "1.0".to_string()])
            .expect_err("expected invalid semver");
        assert!(err.contains("invalid version"));
    }

    #[test]
    fn parse_std_core_args_rejects_unknown_flag() {
        let err =
            parse_std_core_args(vec!["--unknown".to_string()]).expect_err("expected parse error");
        assert!(err.contains("unknown std-core-artifact arg"));
    }

    #[test]
    fn parse_solver_vendor_stage_args_accepts_defaults_and_platform() {
        let opts =
            parse_solver_vendor_stage_args(vec!["--from".to_string(), "tmp/z3.exe".to_string()])
                .expect("parse args");
        assert_eq!(opts.from, PathBuf::from("tmp/z3.exe"));
        assert_eq!(opts.platform, "windows");
        assert_eq!(opts.key_id, "z3-vendor-k7-2026q2");

        let opts = parse_solver_vendor_stage_args(vec![
            "--from".to_string(),
            "tmp/z3".to_string(),
            "--platform".to_string(),
            "linux".to_string(),
            "--key-id".to_string(),
            "z3-vendor-k8-2026q3".to_string(),
        ])
        .expect("parse args");
        assert_eq!(opts.from, PathBuf::from("tmp/z3"));
        assert_eq!(opts.platform, "linux");
        assert_eq!(opts.key_id, "z3-vendor-k8-2026q3");
    }

    #[test]
    fn parse_solver_vendor_stage_args_rejects_missing_from_and_unknown_flag() {
        let err = parse_solver_vendor_stage_args(Vec::new()).expect_err("expected missing --from");
        assert!(err.contains("missing required `--from`"));

        let err = parse_solver_vendor_stage_args(vec![
            "--from".to_string(),
            "tmp/z3.exe".to_string(),
            "--bad".to_string(),
        ])
        .expect_err("expected unknown flag");
        assert!(err.contains("unknown solver-vendor-stage arg"));
    }

    #[test]
    fn parse_solver_vendor_stage_args_rejects_empty_key_id() {
        let err = parse_solver_vendor_stage_args(vec![
            "--from".to_string(),
            "tmp/z3.exe".to_string(),
            "--key-id".to_string(),
            "".to_string(),
        ])
        .expect_err("expected empty key-id error");
        assert!(err.contains("`--key-id` cannot be empty"));
    }

    #[test]
    fn parse_milestone3_binary_bundle_args_accepts_defaults() {
        let opts = parse_milestone3_binary_bundle_args(Vec::new()).expect("parse args");
        assert!(matches!(
            opts.platform.as_str(),
            "windows" | "linux" | "macos"
        ));
        assert!(opts.binary.is_none());
        assert!(opts.out_dir.is_none());
        assert_eq!(opts.key_id, "milestone3-binary-ed25519-2026q2");
    }

    #[test]
    fn parse_milestone3_binary_bundle_args_accepts_explicit_values() {
        let opts = parse_milestone3_binary_bundle_args(vec![
            "--platform".to_string(),
            "linux".to_string(),
            "--binary".to_string(),
            "target/release/clg".to_string(),
            "--out-dir".to_string(),
            "tmp/m3-bundle".to_string(),
            "--key-id".to_string(),
            "release-2026q2".to_string(),
        ])
        .expect("parse args");
        assert_eq!(opts.platform, "linux");
        assert_eq!(opts.binary, Some(PathBuf::from("target/release/clg")));
        assert_eq!(opts.out_dir, Some(PathBuf::from("tmp/m3-bundle")));
        assert_eq!(opts.key_id, "release-2026q2");
    }

    #[test]
    fn parse_milestone3_binary_bundle_args_rejects_invalid_input() {
        let err =
            parse_milestone3_binary_bundle_args(vec!["--platform".to_string(), "bsd".to_string()])
                .expect_err("expected platform validation");
        assert!(err.contains("unsupported `--platform`"));

        let err = parse_milestone3_binary_bundle_args(vec!["--key-id".to_string(), "".to_string()])
            .expect_err("expected empty key-id error");
        assert!(err.contains("`--key-id` cannot be empty"));

        let err = parse_milestone3_binary_bundle_args(vec!["--unknown".to_string()])
            .expect_err("expected unknown arg error");
        assert!(err.contains("unknown milestone3-binary-bundle arg"));
    }

    #[test]
    fn parse_std_surface_args_accepts_emit_and_refresh() {
        let opts = parse_std_surface_args(vec![
            "--emit-artifact".to_string(),
            "tmp/std-surface".to_string(),
            "--refresh-lock".to_string(),
        ])
        .expect("parse args");
        assert_eq!(opts.emit_artifact, Some(PathBuf::from("tmp/std-surface")));
        assert!(opts.refresh_lock);
    }

    #[test]
    fn parse_host_capability_policy_args_accepts_emit_and_refresh() {
        let opts = parse_host_capability_policy_args(vec![
            "--emit-artifact".to_string(),
            "tmp/host-policy".to_string(),
            "--refresh-lock".to_string(),
        ])
        .expect("parse args");
        assert_eq!(opts.emit_artifact, Some(PathBuf::from("tmp/host-policy")));
        assert!(opts.refresh_lock);
    }

    #[test]
    fn parse_manifest_lock_drift_args_defaults_to_phase25_fixture_path() {
        let opts = parse_manifest_lock_drift_args(Vec::new()).expect("parse args");
        assert_eq!(
            opts.paths,
            vec![PathBuf::from(
                "docs/fixtures/phase-25.4/manifest-lock-consistency"
            )]
        );
    }

    #[test]
    fn check_manifest_lock_consistency_accepts_matching_roots() {
        let dir = unique_temp_dir("manifest-lock-ok");
        std::fs::write(
            dir.join("clg.project.json"),
            r#"{
  "schema_version": 1,
  "project": { "name": "fixture-app" },
  "dependencies": [
    { "name": "std::core", "requirement": "^1.0.0" }
  ],
  "release_defaults": {
    "advisory_as_of": "REQUIRED_RFC3339_UTC",
    "key_id": "REQUIRED_KEY_ID",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}"#,
        )
        .expect("write manifest");
        let lock_value = DriftLockFile {
            schema_version: 1,
            resolver_version: 1,
            roots: vec![DriftLockRoot {
                name: "fixture-app".to_string(),
                dependencies: vec![DriftLockRootDependency {
                    name: "std::core".to_string(),
                    requirement: "^1.0.0".to_string(),
                }],
            }],
            packages: Vec::new(),
        };
        let lock_bytes = pretty_json_bytes(&lock_value).expect("serialize lock");
        std::fs::write(dir.join("clg.lock.json"), &lock_bytes).expect("write lock");
        std::fs::write(dir.join("clg.resolved-graph.json"), &lock_bytes)
            .expect("write resolved graph");
        let graph_hash = hex::encode(Sha256::digest(&lock_bytes[..lock_bytes.len() - 1]));
        std::fs::write(
            dir.join("clg.resolved-graph.sha256"),
            format!("{graph_hash}\n"),
        )
        .expect("write resolved graph hash");

        let result = check_manifest_lock_consistency_for_dir(dir.as_path());
        cleanup_temp_dir(dir.as_path());
        result.expect("expected consistency");
    }

    #[test]
    fn check_manifest_lock_consistency_rejects_root_dependency_mismatch() {
        let dir = unique_temp_dir("manifest-lock-mismatch");
        std::fs::write(
            dir.join("clg.project.json"),
            r#"{
  "schema_version": 1,
  "project": { "name": "fixture-app" },
  "dependencies": [
    { "name": "std::core", "requirement": "^1.0.0" }
  ],
  "release_defaults": {
    "advisory_as_of": "REQUIRED_RFC3339_UTC",
    "key_id": "REQUIRED_KEY_ID",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}"#,
        )
        .expect("write manifest");
        let lock_value = DriftLockFile {
            schema_version: 1,
            resolver_version: 1,
            roots: vec![DriftLockRoot {
                name: "fixture-app".to_string(),
                dependencies: vec![DriftLockRootDependency {
                    name: "std::core".to_string(),
                    requirement: "~1.0.0".to_string(),
                }],
            }],
            packages: Vec::new(),
        };
        let lock_bytes = pretty_json_bytes(&lock_value).expect("serialize lock");
        std::fs::write(dir.join("clg.lock.json"), &lock_bytes).expect("write lock");
        std::fs::write(dir.join("clg.resolved-graph.json"), &lock_bytes)
            .expect("write resolved graph");
        let graph_hash = hex::encode(Sha256::digest(&lock_bytes[..lock_bytes.len() - 1]));
        std::fs::write(
            dir.join("clg.resolved-graph.sha256"),
            format!("{graph_hash}\n"),
        )
        .expect("write resolved graph hash");

        let err =
            check_manifest_lock_consistency_for_dir(dir.as_path()).expect_err("expected mismatch");
        cleanup_temp_dir(dir.as_path());
        assert!(err.contains("manifest/lock inconsistency"));
    }

    #[test]
    fn check_manifest_lock_consistency_rejects_non_canonical_lockfile_bytes() {
        let dir = unique_temp_dir("manifest-lock-noncanonical");
        std::fs::write(
            dir.join("clg.project.json"),
            r#"{
  "schema_version": 1,
  "project": { "name": "fixture-app" },
  "dependencies": [
    { "name": "std::core", "requirement": "^1.0.0" }
  ],
  "release_defaults": {
    "advisory_as_of": "REQUIRED_RFC3339_UTC",
    "key_id": "REQUIRED_KEY_ID",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}"#,
        )
        .expect("write manifest");

        let lock_value = serde_json::json!({
            "schema_version": 1,
            "resolver_version": 1,
            "roots": [
                {
                    "name": "fixture-app",
                    "dependencies": [
                        { "name": "std::core", "requirement": "^1.0.0" }
                    ]
                }
            ],
            "packages": []
        });
        let lock_canonical =
            serde_json::to_vec_pretty(&lock_value).expect("serialize canonical lock");
        let mut graph_bytes = lock_canonical.clone();
        graph_bytes.push(b'\n');
        std::fs::write(dir.join("clg.lock.json"), &lock_canonical)
            .expect("write noncanonical lock");
        std::fs::write(dir.join("clg.resolved-graph.json"), &graph_bytes)
            .expect("write resolved graph");
        let graph_hash = hex::encode(Sha256::digest(&graph_bytes[..graph_bytes.len() - 1]));
        std::fs::write(
            dir.join("clg.resolved-graph.sha256"),
            format!("{graph_hash}\n"),
        )
        .expect("write resolved graph hash");

        let err = check_manifest_lock_consistency_for_dir(dir.as_path())
            .expect_err("expected non-canonical lockfile rejection");
        cleanup_temp_dir(dir.as_path());
        assert!(err.contains("canonical tool-owned format"));
    }

    #[test]
    fn parse_phase21_locked_symbols_extracts_entries() {
        let markdown = r#"
locked surface:
- std::list::{len,push,pop}
- std::map::{len,insert,remove,get,contains}
"#;
        let symbols = parse_phase21_locked_symbols_from_design(markdown).expect("parse symbols");
        let expected = BTreeSet::from([
            "std::list::len".to_string(),
            "std::list::push".to_string(),
            "std::list::pop".to_string(),
            "std::map::len".to_string(),
            "std::map::insert".to_string(),
            "std::map::remove".to_string(),
            "std::map::get".to_string(),
            "std::map::contains".to_string(),
        ]);
        assert_eq!(symbols, expected);
    }

    #[test]
    fn parse_phase21_locked_symbols_rejects_empty_input() {
        let err = parse_phase21_locked_symbols_from_design("no symbols here")
            .expect_err("expected parse error");
        assert!(err.contains("yielded zero std symbols"));
    }

    #[test]
    fn normalize_binding_map_lock_sorts_and_validates() {
        let lock = StdBindingMapLockFile {
            schema_version: 1,
            symbols: vec![
                StdBindingRouteEntry {
                    symbol: "std::map::get".to_string(),
                    route: "package_import".to_string(),
                },
                StdBindingRouteEntry {
                    symbol: "std::list::len".to_string(),
                    route: "intrinsic".to_string(),
                },
            ],
        };
        let normalized = normalize_binding_map_lock(lock).expect("normalize");
        assert_eq!(normalized.symbols[0].symbol, "std::list::len");
        assert_eq!(normalized.symbols[1].symbol, "std::map::get");
    }

    #[test]
    fn normalize_binding_map_lock_rejects_invalid_route_and_duplicates() {
        let invalid_route = StdBindingMapLockFile {
            schema_version: 1,
            symbols: vec![StdBindingRouteEntry {
                symbol: "std::map::get".to_string(),
                route: "invalid".to_string(),
            }],
        };
        let err = normalize_binding_map_lock(invalid_route).expect_err("expected route error");
        assert!(err.contains("invalid std binding-map route"));

        let duplicate = StdBindingMapLockFile {
            schema_version: 1,
            symbols: vec![
                StdBindingRouteEntry {
                    symbol: "std::map::get".to_string(),
                    route: "intrinsic".to_string(),
                },
                StdBindingRouteEntry {
                    symbol: "std::map::get".to_string(),
                    route: "package_import".to_string(),
                },
            ],
        };
        let err = normalize_binding_map_lock(duplicate).expect_err("expected duplicate error");
        assert!(err.contains("duplicate std binding-map lock symbol"));
    }

    #[test]
    fn normalize_host_capability_policy_sorts_and_validates() {
        let policy = HostCapabilityPolicyFile {
            schema_version: 1,
            profiles: vec![HostCapabilityProfile {
                profile: "shared_app".to_string(),
                capabilities: vec![
                    HostCapabilityRule {
                        capability: "std::env::time".to_string(),
                        strict_mode: "deny".to_string(),
                        reason: "determinism".to_string(),
                    },
                    HostCapabilityRule {
                        capability: "std::crypto::hash".to_string(),
                        strict_mode: "allow".to_string(),
                        reason: "allowed".to_string(),
                    },
                ],
            }],
        };
        let normalized = normalize_host_capability_policy(policy).expect("normalize");
        assert_eq!(
            normalized.profiles[0].capabilities[0].capability,
            "std::crypto::hash"
        );
        assert_eq!(
            normalized.profiles[0].capabilities[1].capability,
            "std::env::time"
        );
    }

    #[test]
    fn normalize_host_capability_policy_rejects_invalid_entries() {
        let invalid_mode = HostCapabilityPolicyFile {
            schema_version: 1,
            profiles: vec![HostCapabilityProfile {
                profile: "contract_static".to_string(),
                capabilities: vec![HostCapabilityRule {
                    capability: "std::env::time".to_string(),
                    strict_mode: "maybe".to_string(),
                    reason: "bad".to_string(),
                }],
            }],
        };
        let err =
            normalize_host_capability_policy(invalid_mode).expect_err("expected strict_mode error");
        assert!(err.contains("invalid strict_mode"));

        let duplicate_cap = HostCapabilityPolicyFile {
            schema_version: 1,
            profiles: vec![HostCapabilityProfile {
                profile: "contract_static".to_string(),
                capabilities: vec![
                    HostCapabilityRule {
                        capability: "std::env::time".to_string(),
                        strict_mode: "deny".to_string(),
                        reason: "determinism".to_string(),
                    },
                    HostCapabilityRule {
                        capability: "std::env::time".to_string(),
                        strict_mode: "deny".to_string(),
                        reason: "determinism".to_string(),
                    },
                ],
            }],
        };
        let err = normalize_host_capability_policy(duplicate_cap)
            .expect_err("expected duplicate capability error");
        assert!(err.contains("duplicate capability"));
    }

    #[test]
    fn validate_clg_test_report_schema_accepts_expected_shape() {
        let report = r#"{
  "schema_version": 1,
  "status": "ok",
  "report": "json",
  "discovered": 2,
  "selected": 2,
  "executed": 2,
  "passed": 2,
  "failed": 0,
  "tests": [
    {
      "id": "tests/unit/discount_tests.clear::test_discount",
      "file": "tests/unit/discount_tests.clear",
      "function": "test_discount",
      "timeout_ms": 120000,
      "mock_sets": ["promo"],
      "status": "passed",
      "captured_stdout": "",
      "captured_stderr": "",
      "replay": {
        "argv": ["clg", "test", "<project-root>", "--filter", "tests/unit/discount_tests.clear::test_discount", "--report", "json"]
      }
    },
    {
      "id": "tests/unit/discount_tests.clear::test_discount_real",
      "file": "tests/unit/discount_tests.clear",
      "function": "test_discount_real",
      "timeout_ms": 120000,
      "mock_sets": [],
      "status": "passed",
      "captured_stdout": "",
      "captured_stderr": "",
      "replay": {
        "argv": ["clg", "test", "<project-root>", "--filter", "tests/unit/discount_tests.clear::test_discount_real", "--report", "json"]
      }
    }
  ]
}"#;
        validate_clg_test_report_schema(report).expect("schema should validate");
    }

    #[test]
    fn validate_clg_test_report_schema_rejects_unsorted_ids() {
        let report = r#"{
  "schema_version": 1,
  "status": "ok",
  "report": "json",
  "discovered": 2,
  "selected": 2,
  "executed": 2,
  "passed": 2,
  "failed": 0,
  "tests": [
    {
      "id": "tests/unit/b.clear::test_b",
      "file": "tests/unit/b.clear",
      "function": "test_b",
      "timeout_ms": 120000,
      "mock_sets": [],
      "status": "passed",
      "captured_stdout": "",
      "captured_stderr": "",
      "replay": { "argv": ["clg", "test", "<project-root>"] }
    },
    {
      "id": "tests/unit/a.clear::test_a",
      "file": "tests/unit/a.clear",
      "function": "test_a",
      "timeout_ms": 120000,
      "mock_sets": [],
      "status": "passed",
      "captured_stdout": "",
      "captured_stderr": "",
      "replay": { "argv": ["clg", "test", "<project-root>"] }
    }
  ]
}"#;
        let err = validate_clg_test_report_schema(report).expect_err("expected unsorted ids error");
        assert!(err.contains("must be sorted deterministically"));
    }

    #[test]
    fn validate_clg_test_report_schema_rejects_mock_only_coverage() {
        let report = r#"{
  "schema_version": 1,
  "status": "ok",
  "report": "json",
  "discovered": 1,
  "selected": 1,
  "executed": 1,
  "passed": 1,
  "failed": 0,
  "tests": [
    {
      "id": "tests/unit/discount_tests.clear::test_discount",
      "file": "tests/unit/discount_tests.clear",
      "function": "test_discount",
      "timeout_ms": 120000,
      "mock_sets": ["promo"],
      "status": "passed",
      "captured_stdout": "",
      "captured_stderr": "",
      "replay": {
        "argv": ["clg", "test", "<project-root>", "--filter", "tests/unit/discount_tests.clear::test_discount", "--report", "json"]
      }
    }
  ]
}"#;
        let err = validate_clg_test_report_schema(report)
            .expect_err("expected balanced mocked/non-mocked coverage error");
        assert!(err.contains("balanced critical-path coverage"));
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("clg-xtask-{label}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn cleanup_temp_dir(path: &Path) {
        if path.exists() {
            std::fs::remove_dir_all(path).expect("cleanup temp dir");
        }
    }
}
