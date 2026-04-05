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
      "mock_sets": ["common"],
      "status": "passed",
      "captured_stdout": "",
      "captured_stderr": "",
      "replay": {
        "argv": ["clg", "test", "<project-root>", "--filter", "tests/unit/discount_tests.clear::test_discount", "--report", "json"]
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
}
