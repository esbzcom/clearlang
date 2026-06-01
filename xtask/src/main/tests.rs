#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    use std::sync::{Mutex, OnceLock};

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
    fn parse_milestone3_binary_bundle_verify_args_accepts_and_rejects_inputs() {
        let opts = parse_milestone3_binary_bundle_verify_args(vec![
            "--bundle-dir".to_string(),
            "tmp/milestone3/linux".to_string(),
        ])
        .expect("parse verify args");
        assert_eq!(opts.bundle_dir, PathBuf::from("tmp/milestone3/linux"));

        let err = parse_milestone3_binary_bundle_verify_args(Vec::new())
            .expect_err("expected missing --bundle-dir");
        assert!(err.contains("missing required `--bundle-dir`"));

        let err = parse_milestone3_binary_bundle_verify_args(vec!["--bad".to_string()])
            .expect_err("expected unknown arg");
        assert!(err.contains("unknown milestone3-binary-bundle-verify arg"));
    }

    #[test]
    fn milestone3_binary_bundle_emits_signed_artifacts_and_valid_checksums() {
        let _env_lock = test_env_lock().lock().expect("lock env");
        let root = unique_temp_dir("milestone3-binary-bundle");
        let out_dir = root.join("out").join("bundle");
        let binary = root.join("clg");
        std::fs::create_dir_all(out_dir.parent().expect("bundle parent"))
            .expect("create output dir");
        std::fs::write(root.join("LICENSE"), "test license\n").expect("write LICENSE");
        std::fs::write(&binary, b"fake-clg-binary").expect("write fake binary");

        let _key_guard = scoped_env_set(
            "CLG_BINARY_RELEASE_SIGNING_KEY_HEX",
            Some("1111111111111111111111111111111111111111111111111111111111111111"),
        );
        let metadata_fixture = prepare_supply_chain_metadata_fixture(root.as_path());
        let metadata_fixture_str = metadata_fixture.to_string_lossy().to_string();
        let _metadata_guard =
            scoped_env_set("CLG_SUPPLY_CHAIN_METADATA_JSON", Some(metadata_fixture_str.as_str()));
        let (tracked_metadata, runtime_link) = prepare_supply_chain_runtime_fixtures(root.as_path());
        let _tracked_metadata_guard = scoped_env_set(
            "CLG_SUPPLY_CHAIN_TRACKED_METADATA",
            Some(normalize_rel_path(root.as_path(), tracked_metadata.as_path()).as_str()),
        );
        let _runtime_guard = scoped_env_set(
            "CLG_SUPPLY_CHAIN_RUNTIME_LINKS",
            Some(normalize_rel_path(root.as_path(), runtime_link.as_path()).as_str()),
        );
        let _legacy_out_guard = scoped_env_set("CLG_SUPPLY_CHAIN_OUT_DIR", None);

        emit_milestone3_binary_bundle(
            root.as_path(),
            vec![
                "--platform".to_string(),
                "linux".to_string(),
                "--binary".to_string(),
                binary.to_string_lossy().to_string(),
                "--out-dir".to_string(),
                out_dir.to_string_lossy().to_string(),
                "--key-id".to_string(),
                "release-test-2026q2".to_string(),
            ],
        )
        .expect("emit milestone3 binary bundle");
        verify_milestone3_binary_bundle(
            root.as_path(),
            vec![
                "--bundle-dir".to_string(),
                out_dir.to_string_lossy().to_string(),
            ],
        )
        .expect("verify emitted milestone3 binary bundle");

        let signed_path = out_dir
            .join("metadata")
            .join("milestone3-binary-bundle.signed.json");
        let pubkey_path = out_dir
            .join("metadata")
            .join("milestone3-binary-bundle.pubkey.json");
        let checksums_path = out_dir.join("checksums").join("SHA256SUMS");
        assert!(signed_path.is_file(), "signed metadata should exist");
        assert!(pubkey_path.is_file(), "pubkey metadata should exist");
        assert!(checksums_path.is_file(), "checksums manifest should exist");

        let signed: Milestone3SignedBundleMetadata = serde_json::from_slice(
            &std::fs::read(&signed_path).expect("read signed metadata"),
        )
        .expect("parse signed metadata");
        let payload_canonical = serde_json::to_string(&signed.payload).expect("serialize payload");
        let payload_hash = hex::encode(Sha256::digest(payload_canonical.as_bytes()));
        assert_eq!(
            signed.signature.payload_hash.as_str(),
            payload_hash.as_str(),
            "signature payload hash should match canonical payload bytes"
        );
        assert_eq!(
            signed.signature.key_id.as_str(),
            "release-test-2026q2",
            "signature key_id should match explicit --key-id"
        );

        let pubkey: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&pubkey_path).expect("read pubkey metadata"))
                .expect("parse pubkey metadata");
        let public_key_hex = pubkey
            .get("public_key")
            .and_then(|v| v.as_str())
            .expect("pubkey.public_key");
        let mut public_key_bytes = [0u8; 32];
        let decoded_pubkey = hex::decode(public_key_hex).expect("decode pubkey hex");
        public_key_bytes.copy_from_slice(&decoded_pubkey);
        let verifying_key =
            VerifyingKey::from_bytes(&public_key_bytes).expect("verifying key from bytes");
        let signature_bytes =
            hex::decode(signed.signature.signature.as_str()).expect("decode signature hex");
        let signature = Signature::try_from(signature_bytes.as_slice()).expect("signature bytes");
        verifying_key
            .verify(payload_canonical.as_bytes(), &signature)
            .expect("signature should verify");

        let checksums = std::fs::read_to_string(&checksums_path).expect("read checksums");
        let mut listed_paths = Vec::new();
        for line in checksums.lines().filter(|line| !line.trim().is_empty()) {
            let (expected_hash, rel_path) = line
                .split_once("  ")
                .unwrap_or_else(|| panic!("invalid checksum line: {line}"));
            listed_paths.push(rel_path.to_string());
            let normalized_rel = rel_path.replace('/', &std::path::MAIN_SEPARATOR.to_string());
            let absolute = out_dir.join(normalized_rel);
            let actual_hash = file_sha256_hex(absolute.as_path()).expect("hash checksum member");
            assert_eq!(
                actual_hash, expected_hash,
                "checksum mismatch for `{}`",
                rel_path
            );
        }
        let mut sorted_copy = listed_paths.clone();
        sorted_copy.sort();
        assert_eq!(
            listed_paths, sorted_copy,
            "checksum manifest paths should be lexicographically sorted"
        );
        assert!(
            listed_paths.iter().any(|path| path.starts_with("sbom/")),
            "checksums should include SBOM artifacts"
        );
        assert!(
            listed_paths
                .iter()
                .any(|path| path.starts_with("licenses/third-party-licenses.json")),
            "checksums should include third-party license summary"
        );

        cleanup_temp_dir(root.as_path());
    }

    #[test]
    fn load_binary_release_signing_key_fails_closed_for_missing_and_invalid_values() {
        let _env_lock = test_env_lock().lock().expect("lock env");
        let _missing_guard = scoped_env_set("CLG_BINARY_RELEASE_SIGNING_KEY_HEX", None);
        let missing = load_binary_release_signing_key().expect_err("expected missing key error");
        assert!(missing.contains("missing `CLG_BINARY_RELEASE_SIGNING_KEY_HEX`"));
        drop(_missing_guard);

        let _invalid_guard = scoped_env_set("CLG_BINARY_RELEASE_SIGNING_KEY_HEX", Some("ABCDEF"));
        let invalid = load_binary_release_signing_key().expect_err("expected invalid key error");
        assert!(invalid.contains("must be a lowercase 32-byte hex key"));
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
    fn parse_std_arch_sync_args_accepts_write_refresh_and_emit() {
        let opts = parse_std_arch_sync_args(vec![
            "--write".to_string(),
            "--refresh-lock".to_string(),
            "--emit-artifact".to_string(),
            "tmp/std-arch".to_string(),
        ])
        .expect("parse args");
        assert!(opts.write);
        assert!(opts.refresh_lock);
        assert_eq!(opts.emit_artifact, Some(PathBuf::from("tmp/std-arch")));
    }

    #[test]
    fn parse_std_arch_sync_args_rejects_invalid_input() {
        let err =
            parse_std_arch_sync_args(vec!["--emit-artifact".to_string()]).expect_err("expected missing value");
        assert!(err.contains("missing value for `--emit-artifact`"));

        let err =
            parse_std_arch_sync_args(vec!["--unknown".to_string()]).expect_err("expected unknown arg");
        assert!(err.contains("unknown std-arch-sync arg"));
    }

    #[test]
    fn extract_std_symbols_from_coverage_matrix_parses_compact_and_explicit_forms() {
        let dir = unique_temp_dir("std-arch-coverage");
        let coverage_path = dir.join("coverage.md");
        std::fs::write(
            &coverage_path,
            r#"
| `std::list::{len,push}` | yes | yes | yes |
| `std::env::chain_id` | yes | yes | no |
| `std::contract::address::from_bytes` | yes | yes | no |
"#,
        )
        .expect("write coverage fixture");
        let symbols =
            extract_std_symbols_from_coverage_matrix(coverage_path.as_path()).expect("extract symbols");
        cleanup_temp_dir(dir.as_path());

        let expected = BTreeSet::from([
            "std::list::len".to_string(),
            "std::list::push".to_string(),
            "std::env::chain_id".to_string(),
            "std::contract::address::from_bytes".to_string(),
        ]);
        assert_eq!(symbols, expected);
    }

    #[test]
    fn parse_std_coverage_entries_expands_grouped_rows_with_statuses() {
        let entries = parse_std_coverage_entries(
            r#"
| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::host::env::{chain_id,caller}` | no | no | no | pending |
| `std::str::len` | yes | yes | no | ready |
"#,
        )
        .expect("parse coverage entries");

        assert_eq!(
            entries,
            vec![
                StdCoverageEntry {
                    symbol: "std::host::env::chain_id".to_string(),
                    module_path: "std::host::env".to_string(),
                    typed: "no".to_string(),
                    runtime: "no".to_string(),
                    proved: "no".to_string(),
                },
                StdCoverageEntry {
                    symbol: "std::host::env::caller".to_string(),
                    module_path: "std::host::env".to_string(),
                    typed: "no".to_string(),
                    runtime: "no".to_string(),
                    proved: "no".to_string(),
                },
                StdCoverageEntry {
                    symbol: "std::str::len".to_string(),
                    module_path: "std::str".to_string(),
                    typed: "yes".to_string(),
                    runtime: "yes".to_string(),
                    proved: "no".to_string(),
                },
            ]
        );
    }

    #[test]
    fn parse_std_contract_maturity_matrix_normalizes_labels() {
        let maturity = parse_std_contract_maturity_matrix(
            r#"
- `std::core`: `draft` (pending)
- Collections catalog (`std::list`, `std::set`, `std::map`): `ready`
"#,
        )
        .expect("parse maturity matrix");

        assert_eq!(maturity.get("std::core"), Some(&"draft".to_string()));
        assert_eq!(
            maturity.get("Collections catalog (std::list, std::set, std::map)"),
            Some(&"ready".to_string())
        );
    }

    #[test]
    fn evaluate_std_first_production_readiness_flags_pending_rows_missing_metadata_and_draft_core() {
        let coverage = parse_std_coverage_entries(
            r#"
| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::encoder::new` | no | no | no | pending |
| `std::encoder::finish` | no | no | no | pending |
| `std::str::len` | yes | yes | no | ready |
"#,
        )
        .expect("parse coverage");
        let metadata = StdMetadataRoot {
            schema_version: 2,
            modules: vec![StdMetadataModule {
                path: "std::str".to_string(),
                exports: vec![StdMetadataExport {
                    name: "len".to_string(),
                    kind: "value".to_string(),
                    layout: None,
                }],
            }],
        };
        let maturity = BTreeMap::from([("std::core".to_string(), "draft".to_string())]);
        let specs = vec![
            FirstProductionStdSpec {
                package: "std::core",
                maturity_key: "std::core",
                coverage_prefixes: Vec::new(),
                logical_surface: true,
            },
            FirstProductionStdSpec {
                package: "std::codec",
                maturity_key: "std::codec",
                coverage_prefixes: vec!["std::encoder"],
                logical_surface: false,
            },
            FirstProductionStdSpec {
                package: "std::str",
                maturity_key: "std::str",
                coverage_prefixes: vec!["std::str"],
                logical_surface: false,
            },
        ];

        let blockers = evaluate_std_first_production_readiness_for_specs(
            &specs,
            &coverage,
            &metadata,
            &maturity,
        );

        assert!(
            blockers.iter().any(|line| line.contains("std::core maturity is `draft`")),
            "expected draft std::core blocker, got {blockers:?}"
        );
        assert!(
            blockers
                .iter()
                .any(|line| line.contains("std::codec has first-production rows without implementation")),
            "expected codec implementation blocker, got {blockers:?}"
        );
        assert!(
            blockers
                .iter()
                .any(|line| line.contains("std::codec metadata is missing required modules")),
            "expected codec metadata blocker, got {blockers:?}"
        );
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

    struct ScopedEnvVar {
        key: &'static str,
        previous: Option<String>,
    }

    impl Drop for ScopedEnvVar {
        fn drop(&mut self) {
            match self.previous.as_ref() {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    fn scoped_env_set(key: &'static str, value: Option<&str>) -> ScopedEnvVar {
        let previous = std::env::var(key).ok();
        match value {
            Some(raw) => std::env::set_var(key, raw),
            None => std::env::remove_var(key),
        }
        ScopedEnvVar { key, previous }
    }

    fn test_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn prepare_supply_chain_metadata_fixture(root: &Path) -> PathBuf {
        let metadata_path = root.join("metadata.json");
        write_json_pretty(
            metadata_path.as_path(),
            &serde_json::json!({
                "packages": [{
                    "id": "pkg_a 1.0.0 (path+file:///pkg_a)",
                    "name": "pkg_a",
                    "version": "1.0.0",
                    "license": "MIT",
                    "license_file": ""
                }],
                "workspace_members": ["pkg_a 1.0.0 (path+file:///pkg_a)"],
                "resolve": { "nodes": [{ "id": "pkg_a 1.0.0 (path+file:///pkg_a)" }] }
            }),
        )
        .expect("write metadata fixture");
        metadata_path
    }

    fn prepare_supply_chain_runtime_fixtures(root: &Path) -> (PathBuf, PathBuf) {
        let fixture_dir = root.join("fixtures");
        let artifact_dir = fixture_dir.join("artifact");
        std::fs::create_dir_all(&artifact_dir).expect("create fixture artifact dir");
        std::fs::write(artifact_dir.join("pkg-a.wasm"), b"\0asm\x01\0\0\0")
            .expect("write package artifact");

        let package_metadata_path = fixture_dir.join("clg.package-metadata.json");
        write_json_pretty(
            package_metadata_path.as_path(),
            &serde_json::json!({
                "schema_version": 1,
                "packages": [{
                    "name": "pkg::a",
                    "version": "1.0.0",
                    "digest": "sha256:abc",
                    "artifact": { "path": "artifact/pkg-a.wasm" }
                }]
            }),
        )
        .expect("write package metadata fixture");

        let runtime_dir = fixture_dir.join("runtime");
        std::fs::create_dir_all(runtime_dir.join("store")).expect("create runtime store");
        std::fs::write(runtime_dir.join("store").join("pkg-a.wasm"), b"\0asm\x01\0\0\0")
            .expect("write runtime artifact");
        let digest = format!(
            "sha256:{}",
            file_sha256_hex(runtime_dir.join("store").join("pkg-a.wasm").as_path())
                .expect("runtime digest"),
        );

        let runtime_link_path = runtime_dir.join("clg.runtime-link.json");
        write_json_pretty(
            runtime_link_path.as_path(),
            &serde_json::json!({
                "schema_version": 0,
                "packages": [{
                    "id": "pkg::a@1.0.0",
                    "digest": digest,
                    "artifact_path": "store/pkg-a.wasm"
                }],
                "bindings": [{
                    "import_module": "pkg::a",
                    "import_name": "add",
                    "provider_package_id": "pkg::a@1.0.0"
                }]
            }),
        )
        .expect("write runtime-link fixture");
        write_json_pretty(
            runtime_dir.join("clg.lock.json").as_path(),
            &serde_json::json!({
                "schema_version": 1,
                "packages": [{
                    "id": "pkg::a@1.0.0",
                    "digest": digest
                }]
            }),
        )
        .expect("write runtime lock fixture");
        write_json_pretty(
            runtime_dir.join("clg.package-store-index.json").as_path(),
            &serde_json::json!({
                "schema_version": 0,
                "artifacts": [{
                    "id": "pkg::a@1.0.0",
                    "digest": digest,
                    "path": "store/pkg-a.wasm"
                }]
            }),
        )
        .expect("write runtime store index fixture");

        (package_metadata_path, runtime_link_path)
    }
}
