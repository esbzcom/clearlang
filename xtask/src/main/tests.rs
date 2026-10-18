#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    use serde::{Deserialize, Serialize};
    use std::sync::{Mutex, OnceLock};

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct PilotPrecheckDigestProjection {
        schema_version: u32,
        kind: String,
        report_schema_version: u32,
        report_kind: String,
        scope: String,
        gate_order: Vec<String>,
        gates: Vec<PilotPrecheckDigestProjectionGate>,
    }

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct PilotPrecheckDigestProjectionGate {
        id: String,
        input_digests: Vec<PilotPrecheckDigest>,
        evidence_digests: Vec<PilotPrecheckDigest>,
    }

    fn pilot_precheck_digest_projection(
        report: &PilotPrecheckReport,
    ) -> PilotPrecheckDigestProjection {
        PilotPrecheckDigestProjection {
            schema_version: 2,
            kind: "clg.pilot-precheck-digest-projection.v2".to_string(),
            report_schema_version: report.schema_version,
            report_kind: report.kind.clone(),
            scope: report.scope.clone(),
            gate_order: report.gate_order.clone(),
            gates: report
                .gates
                .iter()
                .map(|gate| PilotPrecheckDigestProjectionGate {
                    id: gate.id.clone(),
                    input_digests: gate.input_digests.clone(),
                    evidence_digests: gate.evidence_digests.clone(),
                })
                .collect(),
        }
    }

    fn normalized_document_text(input: &str) -> String {
        input
            .replace("**", "")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

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
    fn parse_shared_std_publish_args_accepts_defaults() {
        let opts = parse_shared_std_publish_args(vec![
            "--package-id".to_string(),
            "std::text".to_string(),
            "--signed-at".to_string(),
            "2026-06-27T00:00:00Z".to_string(),
        ])
        .expect("parse args");
        assert_eq!(opts.package_id, "std::text");
        assert_eq!(opts.version, "1.0.0");
        assert!(opts.out_dir.is_none());
        assert!(opts.registry_dir.is_none());
        assert_eq!(opts.key_id, "shared-std-ed25519-2026q2");
        assert_eq!(opts.trusted_anchor_id, "std-root");
        assert_eq!(opts.statement_format, "in-toto-v1");
        assert_eq!(opts.abi_major, 1);
        assert_eq!(opts.abi_minor_min, 0);
        assert_eq!(opts.abi_minor_max, 0);
    }

    #[test]
    fn parse_shared_std_publish_args_accepts_explicit_values() {
        let opts = parse_shared_std_publish_args(vec![
            "--package-id".to_string(),
            "std::cosmos".to_string(),
            "--version".to_string(),
            "1.2.3".to_string(),
            "--signed-at".to_string(),
            "2026-06-27T01:02:03Z".to_string(),
            "--out-dir".to_string(),
            "tmp/shared".to_string(),
            "--registry-dir".to_string(),
            "tmp/registry".to_string(),
            "--key-id".to_string(),
            "shared-std-2026q3".to_string(),
            "--trusted-anchor-id".to_string(),
            "std-publisher-prod".to_string(),
            "--statement-format".to_string(),
            "in-toto-v1".to_string(),
            "--abi-major".to_string(),
            "2".to_string(),
            "--abi-minor-min".to_string(),
            "4".to_string(),
            "--abi-minor-max".to_string(),
            "7".to_string(),
        ])
        .expect("parse args");
        assert_eq!(opts.package_id, "std::cosmos");
        assert_eq!(opts.version, "1.2.3");
        assert_eq!(opts.out_dir, Some(PathBuf::from("tmp/shared")));
        assert_eq!(opts.registry_dir, Some(PathBuf::from("tmp/registry")));
        assert_eq!(opts.key_id, "shared-std-2026q3");
        assert_eq!(opts.trusted_anchor_id, "std-publisher-prod");
        assert_eq!(opts.signed_at, "2026-06-27T01:02:03Z");
        assert_eq!(opts.abi_major, 2);
        assert_eq!(opts.abi_minor_min, 4);
        assert_eq!(opts.abi_minor_max, 7);
    }

    #[test]
    fn parse_shared_std_publish_args_rejects_invalid_input() {
        let err = parse_shared_std_publish_args(Vec::new())
            .expect_err("expected missing package-id");
        assert!(err.contains("missing required `--package-id`"));

        let accepted = parse_shared_std_publish_args(vec![
            "--package-id".to_string(),
            "std::eth".to_string(),
            "--signed-at".to_string(),
            "2026-06-27T00:00:00Z".to_string(),
        ])
        .expect("std::eth should be accepted");
        assert_eq!(accepted.package_id, "std::eth");

        let accepted = parse_shared_std_publish_args(vec![
            "--package-id".to_string(),
            "std::solana".to_string(),
            "--signed-at".to_string(),
            "2026-06-27T00:00:00Z".to_string(),
        ])
        .expect("std::solana should be accepted");
        assert_eq!(accepted.package_id, "std::solana");

        let accepted = parse_shared_std_publish_args(vec![
            "--package-id".to_string(),
            "std::cosmos".to_string(),
            "--signed-at".to_string(),
            "2026-06-27T00:00:00Z".to_string(),
        ])
        .expect("std::cosmos should be accepted");
        assert_eq!(accepted.package_id, "std::cosmos");

        let err = parse_shared_std_publish_args(vec![
            "--package-id".to_string(),
            "std::text".to_string(),
            "--signed-at".to_string(),
            "2026-06-27".to_string(),
        ])
        .expect_err("expected invalid timestamp");
        assert!(err.contains("invalid timestamp"));

        let err = parse_shared_std_publish_args(vec![
            "--package-id".to_string(),
            "std::text".to_string(),
            "--signed-at".to_string(),
            "2026-06-27T00:00:00Z".to_string(),
            "--abi-minor-min".to_string(),
            "8".to_string(),
            "--abi-minor-max".to_string(),
            "3".to_string(),
        ])
        .expect_err("expected invalid abi range");
        assert!(err.contains("must be <= `--abi-minor-max`"));
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
    fn contract_verifier_package_args_require_the_locked_platform_set_and_trust_inputs() {
        let opts = parse_contract_verifier_package_args(vec![
            "--version".to_string(),
            "0.1.0".to_string(),
            "--artifact".to_string(),
            "windows-x86_64=target/windows/clg-contract-verifier.exe".to_string(),
            "--artifact".to_string(),
            "linux-x86_64=target/linux/clg-contract-verifier".to_string(),
            "--artifact".to_string(),
            "macos-x86_64=target/macos/clg-contract-verifier".to_string(),
            "--signing-key".to_string(),
            "keys/distribution.json".to_string(),
            "--key-id".to_string(),
            "distribution-2026q4".to_string(),
            "--out-dir".to_string(),
            "tmp/verifier-package".to_string(),
        ])
        .expect("parse verifier package args");
        assert_eq!(opts.version, "0.1.0");
        assert_eq!(opts.artifacts.len(), 3);
        assert_eq!(opts.artifacts[0].0, "linux-x86_64");
        assert_eq!(opts.key_id, "distribution-2026q4");

        let err = parse_contract_verifier_package_args(vec![
            "--version".to_string(),
            "0.1.0".to_string(),
        ])
        .expect_err("missing package inputs must fail");
        assert!(err.contains("missing required `--key-id`"));

        let err = parse_contract_verifier_package_verify_args(vec![
            "--package-dir".to_string(),
            "tmp/package".to_string(),
        ])
        .expect_err("missing public key must fail");
        assert!(err.contains("missing required `--pubkey`"));
    }

    #[test]
    fn contract_verifier_package_is_deterministic_signed_and_self_verified() {
        let root = unique_temp_dir("contract-verifier-package");
        let binaries = root.join("binaries");
        std::fs::create_dir_all(&binaries).expect("create binary fixture directory");
        let linux = binaries.join("linux-verifier");
        let macos = binaries.join("macos-verifier");
        let windows = binaries.join("windows-verifier.exe");
        std::fs::write(&linux, b"linux verifier bytes").expect("write linux fixture");
        std::fs::write(&macos, b"macos verifier bytes").expect("write macos fixture");
        std::fs::write(&windows, b"windows verifier bytes").expect("write windows fixture");

        let signing_key = root.join("distribution-key.json");
        let private_key = "3333333333333333333333333333333333333333333333333333333333333333";
        std::fs::write(
            &signing_key,
            format!(r#"{{"scheme":"ed25519","private_key":"{private_key}"}}"#),
        )
        .expect("write signing key");
        let signing = SigningKey::from_bytes(&[0x33; 32]);
        let pubkey = root.join("distribution-pubkey.json");
        std::fs::write(
            &pubkey,
            format!(
                r#"{{"scheme":"ed25519","public_key":"{}"}}"#,
                hex::encode(signing.verifying_key().to_bytes())
            ),
        )
        .expect("write public key");

        let first = root.join("first");
        let second = root.join("second");
        let package = |out_dir: &Path| {
            emit_contract_verifier_package(
                root.as_path(),
                vec![
                    "--version".to_string(),
                    "0.1.0".to_string(),
                    "--artifact".to_string(),
                    format!("windows-x86_64={}", windows.display()),
                    "--artifact".to_string(),
                    format!("linux-x86_64={}", linux.display()),
                    "--artifact".to_string(),
                    format!("macos-x86_64={}", macos.display()),
                    "--signing-key".to_string(),
                    signing_key.display().to_string(),
                    "--key-id".to_string(),
                    "distribution-test-2026q4".to_string(),
                    "--out-dir".to_string(),
                    out_dir.display().to_string(),
                ],
            )
        };
        package(&first).expect("emit first package");
        package(&second).expect("emit second package");

        let expected_names = [
            "clg-contract-verifier-v0.1.0-checksums.txt",
            "clg-contract-verifier-v0.1.0-linux-x86_64.tar.gz",
            "clg-contract-verifier-v0.1.0-macos-x86_64.tar.gz",
            "clg-contract-verifier-v0.1.0-manifest.json",
            "clg-contract-verifier-v0.1.0-manifest.sig.json",
            "clg-contract-verifier-v0.1.0-windows-x86_64.zip",
        ];
        for name in expected_names {
            assert_eq!(
                std::fs::read(first.join(name)).expect("read first package member"),
                std::fs::read(second.join(name)).expect("read second package member"),
                "package member `{name}` must be deterministic"
            );
        }

        verify_contract_verifier_package(
            root.as_path(),
            vec![
                "--package-dir".to_string(),
                first.display().to_string(),
                "--pubkey".to_string(),
                pubkey.display().to_string(),
            ],
        )
        .expect("verify generated package with explicit public key");
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(first.join("clg-contract-verifier-v0.1.0-manifest.json"))
                .expect("read manifest"),
        )
        .expect("parse manifest");
        assert_eq!(manifest["format"], CONTRACT_VERIFIER_MANIFEST_FORMAT);
        assert_eq!(manifest["artifacts"].as_array().map(Vec::len), Some(3));

        std::fs::write(
            first.join("clg-contract-verifier-v0.1.0-linux-x86_64.tar.gz"),
            b"tampered archive",
        )
        .expect("tamper archive");
        let err = verify_contract_verifier_package(
            root.as_path(),
            vec![
                "--package-dir".to_string(),
                first.display().to_string(),
                "--pubkey".to_string(),
                pubkey.display().to_string(),
            ],
        )
        .expect_err("tampered package must be rejected");
        assert!(err.contains("archive digest mismatch"));

        std::fs::write(second.join("unexpected.txt"), b"unbound file")
            .expect("add unbound package file");
        let err = verify_contract_verifier_package(
            root.as_path(),
            vec![
                "--package-dir".to_string(),
                second.display().to_string(),
                "--pubkey".to_string(),
                pubkey.display().to_string(),
            ],
        )
        .expect_err("package with an unbound file must be rejected");
        assert!(err.contains("missing or unbound files"));

        cleanup_temp_dir(root.as_path());
    }

    #[test]
    fn parse_milestone3_installer_channels_args_accepts_explicit_values() {
        let opts = parse_milestone3_installer_channels_args(vec![
            "--version".to_string(),
            "0.1.0".to_string(),
            "--release-tag".to_string(),
            "v0.1.0".to_string(),
            "--repo".to_string(),
            "example/clearlang".to_string(),
            "--windows-bundle-dir".to_string(),
            "tmp/windows".to_string(),
            "--linux-bundle-dir".to_string(),
            "tmp/linux".to_string(),
            "--macos-bundle-dir".to_string(),
            "tmp/macos".to_string(),
            "--out-dir".to_string(),
            "tmp/channels".to_string(),
        ])
        .expect("parse args");
        assert_eq!(opts.version, "0.1.0");
        assert_eq!(opts.release_tag, "v0.1.0");
        assert_eq!(opts.repo, "example/clearlang");
        assert_eq!(opts.windows_bundle_dir, PathBuf::from("tmp/windows"));
        assert_eq!(opts.linux_bundle_dir, PathBuf::from("tmp/linux"));
        assert_eq!(opts.macos_bundle_dir, PathBuf::from("tmp/macos"));
        assert_eq!(opts.out_dir, Some(PathBuf::from("tmp/channels")));
    }

    #[test]
    fn parse_milestone3_installer_channels_args_rejects_invalid_input() {
        let err = parse_milestone3_installer_channels_args(Vec::new())
            .expect_err("expected missing args");
        assert!(err.contains("missing required `--version`"));

        let err = parse_milestone3_installer_channels_args(vec![
            "--version".to_string(),
            "0.1".to_string(),
        ])
        .expect_err("expected invalid semver");
        assert!(err.contains("invalid version"));

        let err = parse_milestone3_installer_channels_args(vec![
            "--version".to_string(),
            "0.1.0".to_string(),
            "--release-tag".to_string(),
            "v0.1.0".to_string(),
            "--repo".to_string(),
            "badrepo".to_string(),
            "--windows-bundle-dir".to_string(),
            "tmp/windows".to_string(),
            "--linux-bundle-dir".to_string(),
            "tmp/linux".to_string(),
            "--macos-bundle-dir".to_string(),
            "tmp/macos".to_string(),
        ])
        .expect_err("expected invalid repo");
        assert!(err.contains("invalid `--repo` value"));
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
            let normalized_rel = rel_path.replace('/', std::path::MAIN_SEPARATOR_STR);
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
    fn milestone3_installer_channels_emit_archives_and_metadata() {
        let _env_lock = test_env_lock().lock().expect("lock env");
        let root = unique_temp_dir("milestone3-installer-channels");
        std::fs::write(root.join("LICENSE"), "test license\n").expect("write LICENSE");

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

        let windows_binary = root.join("clg.exe");
        let linux_binary = root.join("clg-linux");
        let macos_binary = root.join("clg-macos");
        std::fs::write(&windows_binary, b"fake-clg-windows").expect("write windows binary");
        std::fs::write(&linux_binary, b"fake-clg-linux").expect("write linux binary");
        std::fs::write(&macos_binary, b"fake-clg-macos").expect("write macos binary");

        let windows_bundle = root.join("bundles").join("windows");
        let linux_bundle = root.join("bundles").join("linux");
        let macos_bundle = root.join("bundles").join("macos");
        for (platform, binary, bundle_dir) in [
            ("windows", windows_binary.as_path(), windows_bundle.as_path()),
            ("linux", linux_binary.as_path(), linux_bundle.as_path()),
            ("macos", macos_binary.as_path(), macos_bundle.as_path()),
        ] {
            emit_milestone3_binary_bundle(
                root.as_path(),
                vec![
                    "--platform".to_string(),
                    platform.to_string(),
                    "--binary".to_string(),
                    binary.to_string_lossy().to_string(),
                    "--out-dir".to_string(),
                    bundle_dir.to_string_lossy().to_string(),
                ],
            )
            .expect("emit platform bundle");
        }

        let out_dir = root.join("out").join("channels");
        emit_milestone3_installer_channels(
            root.as_path(),
            vec![
                "--version".to_string(),
                "0.1.0".to_string(),
                "--release-tag".to_string(),
                "v0.1.0".to_string(),
                "--windows-bundle-dir".to_string(),
                windows_bundle.to_string_lossy().to_string(),
                "--linux-bundle-dir".to_string(),
                linux_bundle.to_string_lossy().to_string(),
                "--macos-bundle-dir".to_string(),
                macos_bundle.to_string_lossy().to_string(),
                "--out-dir".to_string(),
                out_dir.to_string_lossy().to_string(),
            ],
        )
        .expect("emit installer channels");

        let manifest_path = out_dir
            .join("metadata")
            .join("milestone3-installer-channels.json");
        let manifest: Milestone3InstallerChannelsManifest = serde_json::from_slice(
            &std::fs::read(&manifest_path).expect("read installer manifest"),
        )
        .expect("parse installer manifest");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.release_tag, "v0.1.0");
        assert_eq!(manifest.archives.len(), 3);
        assert_eq!(manifest.homebrew.formula_path, "homebrew/Formula/clg.rb");
        assert_eq!(
            manifest.winget.nested_installer_relative_path,
            "bin/windows/clg.exe"
        );

        let formula = std::fs::read_to_string(
            out_dir.join("homebrew").join("Formula").join("clg.rb"),
        )
        .expect("read homebrew formula");
        assert!(
            formula.contains("https://github.com/clearlang/clearlang/releases/download/v0.1.0/clg-milestone3-linux-x64-bundle.zip"),
            "formula should reference canonical linux archive URL"
        );
        assert!(
            formula.contains("https://github.com/clearlang/clearlang/releases/download/v0.1.0/clg-milestone3-macos-x64-bundle.zip"),
            "formula should reference canonical macos archive URL"
        );

        let winget_installer = std::fs::read_to_string(
            out_dir
                .join("winget")
                .join("manifests")
                .join("c")
                .join("ClearLang")
                .join("ClearLang")
                .join("0.1.0")
                .join("ClearLang.ClearLang.installer.yaml"),
        )
        .expect("read winget installer manifest");
        assert!(winget_installer.contains("InstallerType: zip"));
        assert!(winget_installer.contains("NestedInstallerType: portable"));
        assert!(winget_installer.contains("RelativeFilePath: bin/windows/clg.exe"));

        let windows_archive_path = out_dir
            .join("archives")
            .join("clg-milestone3-windows-x64-bundle.zip");
        let windows_archive =
            std::fs::File::open(&windows_archive_path).expect("open windows archive");
        let mut zip = zip::ZipArchive::new(windows_archive).expect("parse windows zip");
        let mut names = Vec::new();
        for idx in 0..zip.len() {
            let file = zip.by_index(idx).expect("zip entry");
            names.push(file.name().to_string());
        }
        assert!(
            names.contains(&"bin/windows/clg.exe".to_string()),
            "archive should contain the portable windows binary"
        );
        assert!(
            names.contains(&"metadata/milestone3-binary-bundle.signed.json".to_string()),
            "archive should retain signed bundle metadata"
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
    fn load_shared_std_signing_key_fails_closed_for_missing_and_invalid_values() {
        let _env_lock = test_env_lock().lock().expect("lock env");
        let _missing_guard = scoped_env_set("CLG_SHARED_STD_SIGNING_KEY_HEX", None);
        let missing = load_shared_std_signing_key().expect_err("expected missing key error");
        assert!(missing.contains("missing `CLG_SHARED_STD_SIGNING_KEY_HEX`"));
        drop(_missing_guard);

        let _invalid_guard = scoped_env_set("CLG_SHARED_STD_SIGNING_KEY_HEX", Some("ABCDEF"));
        let invalid = load_shared_std_signing_key().expect_err("expected invalid key error");
        assert!(invalid.contains("must be a lowercase 32-byte hex key"));
    }

    #[test]
    fn publish_shared_std_artifact_emits_signed_publish_bundle_and_registry_copy() {
        let _env_lock = test_env_lock().lock().expect("lock env");
        let repo = repo_root();
        let out_dir = unique_temp_dir("shared-std-publish-out");
        let registry_dir = unique_temp_dir("shared-std-publish-registry");
        let _key_guard = scoped_env_set(
            "CLG_SHARED_STD_SIGNING_KEY_HEX",
            Some("2222222222222222222222222222222222222222222222222222222222222222"),
        );

        publish_shared_std_artifact(
            repo.as_path(),
            vec![
                "--package-id".to_string(),
                "std::cosmos".to_string(),
                "--version".to_string(),
                "1.2.3".to_string(),
                "--signed-at".to_string(),
                "2026-06-27T00:00:00Z".to_string(),
                "--out-dir".to_string(),
                out_dir.to_string_lossy().to_string(),
                "--registry-dir".to_string(),
                registry_dir.to_string_lossy().to_string(),
                "--key-id".to_string(),
                "shared-std-2026q2".to_string(),
                "--trusted-anchor-id".to_string(),
                "std-publisher-prod".to_string(),
            ],
        )
        .expect("publish shared std artifact");

        let publish_manifest_path = out_dir.join("std-cosmos-1.2.3.publish.json");
        let package_manifest_path = out_dir.join("std-cosmos-1.2.3.shared-std-package.json");
        let signatures_path = out_dir.join("std-cosmos-1.2.3.package-signatures.json");
        let pubkey_path = out_dir.join("std-cosmos-1.2.3.pubkey.json");
        let provenance_path = out_dir.join("std-cosmos-1.2.3.provenance.json");
        let canonical_metadata_path = out_dir.join("clg.package-metadata.json");
        let canonical_abi_path = out_dir.join("clg.package-abi.json");

        assert!(publish_manifest_path.is_file());
        assert!(package_manifest_path.is_file());
        assert!(signatures_path.is_file());
        assert!(pubkey_path.is_file());
        assert!(provenance_path.is_file());
        assert!(canonical_metadata_path.is_file());
        assert!(canonical_abi_path.is_file());

        let publish_manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&publish_manifest_path).expect("read publish manifest"),
        )
        .expect("parse publish manifest");
        assert_eq!(publish_manifest["package_id"], "std::cosmos");
        assert_eq!(publish_manifest["version"], "1.2.3");
        assert_eq!(publish_manifest["registry_mode"], "file_registry");

        let package_manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&package_manifest_path).expect("read shared std package manifest"),
        )
        .expect("parse shared std package manifest");
        assert_eq!(package_manifest["package_id"], "std::cosmos");
        assert_eq!(package_manifest["version"], "1.2.3");
        assert_eq!(package_manifest["signature"]["key_id"], "shared-std-2026q2");
        assert_eq!(package_manifest["provenance"]["statement_format"], "in-toto-v1");

        let signatures: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&signatures_path).expect("read package signatures"),
        )
        .expect("parse package signatures");
        assert_eq!(signatures["signatures"][0]["name"], "std::cosmos");
        assert_eq!(signatures["signatures"][0]["version"], "1.2.3");

        let pubkey: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&pubkey_path).expect("read pubkey"))
                .expect("parse pubkey");
        let public_key_hex = pubkey["public_key"].as_str().expect("pubkey");
        let mut public_key_bytes = [0u8; 32];
        let decoded_pubkey = hex::decode(public_key_hex).expect("decode pubkey hex");
        public_key_bytes.copy_from_slice(&decoded_pubkey);
        let verifying_key =
            VerifyingKey::from_bytes(&public_key_bytes).expect("verifying key from bytes");

        let signed_digest = signatures["signatures"][0]["digest"]
            .as_str()
            .expect("signature digest");
        let signed_at = signatures["signatures"][0]["signed_at"]
            .as_str()
            .expect("signature signed_at");
        let payload = canonical_package_signature_payload(
            "std::cosmos",
            "1.2.3",
            signed_digest,
            signed_at,
        );
        let signature_bytes = hex::decode(
            signatures["signatures"][0]["signature"]
                .as_str()
                .expect("signature hex"),
        )
        .expect("decode signature");
        let signature = Signature::try_from(signature_bytes.as_slice()).expect("signature bytes");
        verifying_key
            .verify(payload.as_bytes(), &signature)
            .expect("shared std package signature should verify");

        let provenance: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&provenance_path).expect("read provenance"),
        )
        .expect("parse provenance");
        assert_eq!(provenance["registry"]["mode"], "file_registry");

        let canonical_metadata: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&canonical_metadata_path).expect("read canonical metadata"),
        )
        .expect("parse canonical metadata");
        assert_eq!(canonical_metadata["packages"][0]["name"], "std::cosmos");
        assert_eq!(
            canonical_metadata["packages"][0]["artifact"]["path"],
            "std-packages/std-cosmos-1.2.3.wasm"
        );
        assert_eq!(
            canonical_metadata["packages"][0]["trust"]["trusted_anchor_ids"][0],
            "std-publisher-prod"
        );
        let cosmos_symbols = package_manifest["symbols"]
            .as_array()
            .expect("shared std package symbols should be an array")
            .iter()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            cosmos_symbols,
            vec!["std::cosmos::from_array", "std::cosmos::from_bytes",]
        );

        let registry_copy = registry_dir
            .join("std__cosmos")
            .join("1.2.3")
            .join("std-cosmos-1.2.3.shared-std-package.json");
        assert!(registry_copy.is_file(), "registry package manifest should exist");
        assert!(
            registry_dir
                .join("std__cosmos")
                .join("1.2.3")
                .join("std-packages")
                .join("std-cosmos-1.2.3.wasm")
                .is_file(),
            "registry artifact copy should preserve nested std-packages layout"
        );

        cleanup_temp_dir(out_dir.as_path());
        cleanup_temp_dir(registry_dir.as_path());
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
    fn extract_std_symbols_from_source_captures_deep_std_paths() {
        let symbols = extract_std_symbols_from_source(
            r#"
fn demo() {
    let _ = "std::bytes::len";
    let _ = "std::contract::address::from_bytes";
    let _ = "std::host::env::chain_id";
}
"#,
        )
        .expect("extract source symbols");

        let expected = BTreeSet::from([
            "std::bytes::len".to_string(),
            "std::contract::address::from_bytes".to_string(),
            "std::host::env::chain_id".to_string(),
        ]);
        assert_eq!(symbols, expected);
    }

    #[test]
    fn external_package_reference_policy_allows_documented_transition_sites_only() {
        let external_intrinsic_symbols = BTreeSet::from([
            "std::bytes::len".to_string(),
            "std::bytes::eq_ct".to_string(),
            "std::str::len".to_string(),
            "std::u64::rotl".to_string(),
        ]);

        assert!(is_authorized_external_package_compiler_reference(
            "crates/codegen-wasm/src/ir/mod.rs",
            "std::bytes::len",
            &external_intrinsic_symbols,
        ));
        assert!(is_authorized_external_package_compiler_reference(
            "crates/typer/src/lower/contract.rs",
            "std::contract::address::from_bytes",
            &external_intrinsic_symbols,
        ));
        assert!(is_authorized_external_package_compiler_reference(
            "crates/typer/src/vc/generate/helpers.rs",
            "std::bytes::equals_ct",
            &external_intrinsic_symbols,
        ));
        for path in [
            "crates/typer/src/check/caller_authorization.rs",
            "crates/typer/src/lower/calls.rs",
            "crates/typer/src/vc/generate.rs",
            "crates/typer/src/vc/refinements/smt.rs",
            "crates/cli/src/commands/build/contract_state_schema.rs",
            "crates/cli/src/commands/build/evm_artifact.rs",
            "crates/cli/src/commands/build/evm_wire_abi.rs",
            "crates/cli/src/commands/simulate.rs",
        ] {
            assert!(is_authorized_external_package_compiler_reference(
                path,
                "std::eth::Address",
                &external_intrinsic_symbols,
            ));
        }
        for path in [
            "crates/typer/src/check/caller_authorization.rs",
            "crates/typer/src/lower/calls.rs",
            "crates/typer/src/vc/generate.rs",
            "crates/cli/src/commands/build/contract_state_schema.rs",
        ] {
            for symbol in ["std::eth::address::equals", "std::eth::context::caller"] {
                assert!(is_authorized_external_package_compiler_reference(
                    path,
                    symbol,
                    &external_intrinsic_symbols,
                ));
            }
        }
        assert!(!is_authorized_external_package_compiler_reference(
            "crates/codegen-wasm/src/ir/mod.rs",
            "std::contract::address::from_bytes",
            &external_intrinsic_symbols,
        ));
        assert!(!is_authorized_external_package_compiler_reference(
            "crates/cli/src/commands/modules/verified_std_abi.rs",
            "std::str::len",
            &external_intrinsic_symbols,
        ));
        assert!(!is_authorized_external_package_compiler_reference(
            "crates/typer/src/check/expr/calls/collections.rs",
            "std::eth::Address",
            &external_intrinsic_symbols,
        ));
        assert!(!is_authorized_external_package_compiler_reference(
            "crates/typer/src/check/caller_authorization.rs",
            "std::eth::from_bytes",
            &external_intrinsic_symbols,
        ));
        assert!(!is_authorized_external_package_compiler_reference(
            "crates/cli/src/commands/simulate.rs",
            "std::eth::context::caller",
            &external_intrinsic_symbols,
        ));
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
    fn external_std_package_plan_groups_first_wave_modules_deterministically() {
        let catalog = StdCatalogLockFile {
            schema_version: 1,
            modules: vec![
                StdCatalogModule {
                    path: "std::bytes".to_string(),
                    module_class: "pure_std".to_string(),
                    exports: vec![StdCatalogExport {
                        name: "len".to_string(),
                        kind: "value".to_string(),
                        arity: Some(1),
                        effect: Some("pure".to_string()),
                        route: Some("intrinsic".to_string()),
                        capability: None,
                        deprecated_alias_of: None,
                        layout: None,
                    }],
                },
                StdCatalogModule {
                    path: "std::str".to_string(),
                    module_class: "pure_std".to_string(),
                    exports: vec![StdCatalogExport {
                        name: "len".to_string(),
                        kind: "value".to_string(),
                        arity: Some(1),
                        effect: Some("pure".to_string()),
                        route: Some("intrinsic".to_string()),
                        capability: None,
                        deprecated_alias_of: None,
                        layout: None,
                    }],
                },
                StdCatalogModule {
                    path: "std::u64".to_string(),
                    module_class: "pure_std".to_string(),
                    exports: vec![StdCatalogExport {
                        name: "rotl".to_string(),
                        kind: "value".to_string(),
                        arity: Some(2),
                        effect: Some("pure".to_string()),
                        route: Some("intrinsic".to_string()),
                        capability: None,
                        deprecated_alias_of: None,
                        layout: None,
                    }],
                },
                StdCatalogModule {
                    path: "std::encoder".to_string(),
                    module_class: "pure_std".to_string(),
                    exports: vec![StdCatalogExport {
                        name: "new".to_string(),
                        kind: "value".to_string(),
                        arity: Some(0),
                        effect: Some("pure".to_string()),
                        route: Some("package_import".to_string()),
                        capability: None,
                        deprecated_alias_of: None,
                        layout: None,
                    }],
                },
                StdCatalogModule {
                    path: "std::host".to_string(),
                    module_class: "pure_std".to_string(),
                    exports: vec![StdCatalogExport {
                        name: "ignored".to_string(),
                        kind: "value".to_string(),
                        arity: Some(0),
                        effect: Some("io".to_string()),
                        route: Some("intrinsic".to_string()),
                        capability: Some("std::host::ignored".to_string()),
                        deprecated_alias_of: None,
                        layout: None,
                    }],
                },
            ],
        };

        let plan = external_std_package_plan_from_catalog(&catalog);
        assert_eq!(plan.schema_version, 1);
        assert_eq!(plan.packages.len(), 3);

        let text = plan
            .packages
            .iter()
            .find(|entry| entry.package_id == "std::text")
            .expect("std::text package plan");
        assert_eq!(text.wave, "wave1");
        assert_eq!(
            text.modules,
            vec!["std::bytes".to_string(), "std::str".to_string()]
        );
        assert_eq!(
            text.symbols,
            vec!["std::bytes::len".to_string(), "std::str::len".to_string()]
        );

        let int = plan
            .packages
            .iter()
            .find(|entry| entry.package_id == "std::int")
            .expect("std::int package plan");
        assert_eq!(int.wave, "wave1");
        assert_eq!(int.modules, vec!["std::u64".to_string()]);
        assert_eq!(int.symbols, vec!["std::u64::rotl".to_string()]);

        let codec = plan
            .packages
            .iter()
            .find(|entry| entry.package_id == "std::codec")
            .expect("std::codec package plan");
        assert_eq!(codec.wave, "wave1");
        assert_eq!(codec.modules, vec!["std::encoder".to_string()]);
        assert_eq!(codec.symbols, vec!["std::encoder::new".to_string()]);
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
    fn parse_pilot_precheck_args_accepts_only_one_report_destination() {
        let opts = parse_pilot_precheck_args(vec![
            "--report".to_string(),
            "tmp/pilot-precheck.json".to_string(),
        ])
        .expect("parse report destination");
        assert_eq!(opts.report, PathBuf::from("tmp/pilot-precheck.json"));

        for args in [
            Vec::new(),
            vec!["--report".to_string()],
            vec!["--unknown".to_string()],
            vec![
                "--report".to_string(),
                "one.json".to_string(),
                "--report".to_string(),
                "two.json".to_string(),
            ],
        ] {
            assert!(parse_pilot_precheck_args(args).is_err());
        }
    }

    #[test]
    fn safe_policy_validator_accepts_only_the_locked_deployment_shape() {
        let root = unique_temp_dir("safe-policy-validator");
        let artifact_root = root.join("artifacts");
        std::fs::create_dir_all(artifact_root.join("release")).expect("create release root");
        std::fs::create_dir_all(artifact_root.join("policy")).expect("create policy root");
        std::fs::create_dir_all(artifact_root.join("approvals")).expect("create approval root");

        let bundle = b"bundle";
        let verifier = b"verifier";
        let operator_policy = b"operator-policy";
        let safe_amendment = b"safe-amendment";
        std::fs::write(artifact_root.join("release/bundle.json"), bundle).expect("write bundle");
        std::fs::write(artifact_root.join("release/verifier.json"), verifier).expect("write verifier");
        std::fs::write(artifact_root.join("policy/operator.json"), operator_policy).expect("write policy");
        std::fs::write(artifact_root.join("policy/safe-amendment.json"), safe_amendment).expect("write amendment");

        let owners = vec![
            "0x1111111111111111111111111111111111111111".to_string(),
            "0x2222222222222222222222222222222222222222".to_string(),
            "0x3333333333333333333333333333333333333333".to_string(),
        ];
        let safe_identity = SafeIdentity {
            safe_address: "0x4444444444444444444444444444444444444444".to_string(),
            safe_version: "1.4.1".to_string(),
            proxy_runtime_code_sha256: "a".repeat(64),
            singleton_address: "0x5555555555555555555555555555555555555555".to_string(),
            singleton_runtime_code_sha256: "b".repeat(64),
            owners: owners.clone(),
            threshold: "2".to_string(),
            safe_nonce: "0".to_string(),
            modules: Vec::new(),
            guard: None,
            fallback_handler: None,
        };
        let mut creation = vec![0xaa, 0xbb, 0xcc, 0xdd];
        creation.extend_from_slice(&[0; 12]);
        creation.extend_from_slice(&hex::decode(&safe_identity.safe_address[2..]).expect("safe bytes"));
        creation.extend_from_slice(&[0; 24]);
        creation.extend_from_slice(&7_u64.to_be_bytes());
        let request = SafePolicyRequest {
            format: SAFE_POLICY_REQUEST_FORMAT.to_string(),
            schema_version: 1,
            request_id: format!("clgreq-{}", "c".repeat(64)),
            environment: "production".to_string(),
            target: SafePolicyTarget {
                profile: SAFE_POLICY_PROFILE.to_string(),
                chain_id: "1".to_string(),
                submission_endpoint: "https://operator.example".to_string(),
            },
            created_at: "2030-01-02T03:04:05Z".to_string(),
            expires_at: "2030-01-03T03:04:05Z".to_string(),
            release: SafePolicyRelease {
                bundle_path: "release/bundle.json".to_string(),
                bundle_sha256: safe_policy_sha256_hex(bundle),
                verifier_result_path: "release/verifier.json".to_string(),
                verifier_result_sha256: safe_policy_sha256_hex(verifier),
            },
            policy: SafePolicyBinding {
                operator_policy_path: "policy/operator.json".to_string(),
                operator_policy_sha256: safe_policy_sha256_hex(operator_policy),
                safe_owner_amendment_path: "policy/safe-amendment.json".to_string(),
                safe_owner_amendment_sha256: safe_policy_sha256_hex(safe_amendment),
            },
            action: SafePolicyAction::RegistryDeploy(RegistryDeployAction {
                deployment_sender: "0x6666666666666666666666666666666666666666".to_string(),
                deployment_nonce: "0".to_string(),
                value_wei: "0".to_string(),
                gas_limit: "250000".to_string(),
                fee: SafeFee {
                    kind: "legacy-eip155-gas-price".to_string(),
                    gas_price_wei: "1".to_string(),
                },
                creation_calldata: format!("0x{}", hex::encode(&creation)),
                creation_calldata_sha256: safe_policy_sha256_hex(&creation),
                constructor: RegistryConstructor {
                    owner_safe_address: safe_identity.safe_address.clone(),
                    initial_value_u64: "7".to_string(),
                },
                safe_identity: safe_identity.clone(),
            }),
        };
        let request_bytes = canonical_json_bytes(&request).expect("canonical request");
        let request_digest = safe_policy_sha256_hex(&request_bytes);
        let request_path = root.join("request.json");
        std::fs::write(&request_path, &request_bytes).expect("write request");

        let mut approvals = Vec::new();
        for (index, owner) in owners.iter().take(2).enumerate() {
            let approval = SafeOwnerApproval {
                format: SAFE_POLICY_APPROVAL_FORMAT.to_string(),
                schema_version: 1,
                transaction_request_sha256: request_digest.clone(),
                environment: "production".to_string(),
                target_profile: SAFE_POLICY_PROFILE.to_string(),
                chain_id: "1".to_string(),
                release_bundle_sha256: request.release.bundle_sha256.clone(),
                operator_policy_sha256: request.policy.operator_policy_sha256.clone(),
                action_kind: "registry_deploy".to_string(),
                owner_address: owner.clone(),
                approved_at: "2030-01-02T03:04:05Z".to_string(),
                expires_at: "2030-01-03T03:04:05Z".to_string(),
            };
            let bytes = canonical_json_bytes(&approval).expect("canonical approval");
            let path = format!("approvals/owner-{}.json", index + 1);
            std::fs::write(artifact_root.join(&path), &bytes).expect("write approval");
            approvals.push(SafeEvidenceApproval {
                owner_address: owner.clone(),
                approval_record_path: path,
                approval_record_sha256: safe_policy_sha256_hex(&bytes),
                approved_at: approval.approved_at,
                expires_at: approval.expires_at,
            });
        }
        let evidence = SafePolicyEvidence {
            format: SAFE_POLICY_EVIDENCE_FORMAT.to_string(),
            schema_version: 1,
            transaction_request_sha256: request_digest,
            environment: "production".to_string(),
            target_profile: SAFE_POLICY_PROFILE.to_string(),
            chain_id: "1".to_string(),
            release_bundle_sha256: request.release.bundle_sha256.clone(),
            operator_policy_sha256: request.policy.operator_policy_sha256.clone(),
            approval_kind: "safe_owner_deployment_consent".to_string(),
            required_owner_count: "2".to_string(),
            frozen_owner_set_sha256: safe_policy_sha256_hex(
                &serde_json::to_vec(&owners).expect("serialize owners"),
            ),
            approvals,
            review_disposition: "approved".to_string(),
        };
        let evidence_path = root.join("evidence.json");
        std::fs::write(
            &evidence_path,
            canonical_json_bytes(&evidence).expect("canonical evidence"),
        )
        .expect("write evidence");

        assert_eq!(
            validate_safe_policy_files(&request_path, &evidence_path, &artifact_root),
            Ok((
                safe_policy_sha256_hex(&request_bytes),
                safe_policy_sha256_hex(
                    &canonical_json_bytes(&evidence).expect("canonical evidence digest"),
                ),
                "registry_deploy"
            ))
        );
        let accepted_report_path = root.join("accepted-report.json");
        run_safe_policy_validate(
            repo_root().as_path(),
            vec![
                "--request".to_string(),
                request_path.to_string_lossy().into_owned(),
                "--evidence".to_string(),
                evidence_path.to_string_lossy().into_owned(),
                "--artifact-root".to_string(),
                artifact_root.to_string_lossy().into_owned(),
                "--report".to_string(),
                accepted_report_path.to_string_lossy().into_owned(),
            ],
        )
        .expect("accepted request writes report");
        let accepted_report: SafePolicyValidationReport = serde_json::from_slice(
            &std::fs::read(&accepted_report_path).expect("read accepted report"),
        )
        .expect("parse accepted report");
        assert_eq!(accepted_report.status, "accepted");
        assert!(accepted_report.rejection_code.is_none());

        let mut unsafe_request = request;
        let SafePolicyAction::RegistryDeploy(action) = &mut unsafe_request.action else {
            panic!("expected deployment action");
        };
        action.value_wei = "1".to_string();
        std::fs::write(
            &request_path,
            canonical_json_bytes(&unsafe_request).expect("canonical unsafe request"),
        )
        .expect("write unsafe request");
        assert_eq!(
            validate_safe_policy_files(&request_path, &evidence_path, &artifact_root)
                .expect_err("nonzero deployment value must reject")
                .code,
            "M6B006"
        );
        let rejected_report_path = root.join("rejected-report.json");
        assert!(run_safe_policy_validate(
            repo_root().as_path(),
            vec![
                "--request".to_string(),
                request_path.to_string_lossy().into_owned(),
                "--evidence".to_string(),
                evidence_path.to_string_lossy().into_owned(),
                "--artifact-root".to_string(),
                artifact_root.to_string_lossy().into_owned(),
                "--report".to_string(),
                rejected_report_path.to_string_lossy().into_owned(),
            ],
        )
        .is_err());
        let rejected_report: SafePolicyValidationReport = serde_json::from_slice(
            &std::fs::read(&rejected_report_path).expect("read rejected report"),
        )
        .expect("parse rejected report");
        assert_eq!(rejected_report.status, "rejected");
        assert_eq!(rejected_report.rejection_code.as_deref(), Some("M6B006"));

        let signature_bundle = SafeSignatureBundle {
            format: SAFE_SIGNATURE_BUNDLE_FORMAT.to_string(),
            schema_version: 1,
            safe_tx_digest: format!("0x{}", "d".repeat(64)),
            signer_addresses: owners[..2].to_vec(),
        };
        let signature_bundle_bytes = canonical_json_bytes(&signature_bundle)
            .expect("canonical signature bundle");
        std::fs::write(
            artifact_root.join("approvals/safe-signatures.json"),
            &signature_bundle_bytes,
        )
        .expect("write signature bundle");
        let mut set_value_calldata = vec![0x13, 0x76, 0x58, 0x38];
        set_value_calldata.extend_from_slice(&[0; 24]);
        set_value_calldata.extend_from_slice(&9_u64.to_be_bytes());
        let mut revision = SafeRegistryRevisionAction {
            executor_sender: "0x7777777777777777777777777777777777777777".to_string(),
            executor_nonce: "3".to_string(),
            outer_value_wei: "0".to_string(),
            outer_gas_limit: "100000".to_string(),
            outer_fee: SafeFee {
                kind: "legacy-eip155-gas-price".to_string(),
                gas_price_wei: "1".to_string(),
            },
            safe_identity,
            safe_transaction: SafeTransaction {
                safe_nonce: "0".to_string(),
                to: "0x8888888888888888888888888888888888888888".to_string(),
                value_wei: "0".to_string(),
                operation: "CALL".to_string(),
                set_value_next_u64: "9".to_string(),
                set_value_calldata: format!("0x{}", hex::encode(&set_value_calldata)),
                set_value_calldata_sha256: safe_policy_sha256_hex(&set_value_calldata),
                safe_tx_digest: signature_bundle.safe_tx_digest,
                signature_bundle_path: "approvals/safe-signatures.json".to_string(),
                signature_bundle_sha256: safe_policy_sha256_hex(&signature_bundle_bytes),
            },
        };
        assert!(validate_safe_registry_revision(&revision, &artifact_root).is_ok());
        revision.safe_transaction.operation = "DELEGATECALL".to_string();
        assert_eq!(
            validate_safe_registry_revision(&revision, &artifact_root)
                .expect_err("delegatecall must reject")
                .code,
            "M6B007"
        );
        cleanup_temp_dir(root.as_path());
    }

    #[test]
    fn milestone_six_operator_control_matrix_and_runbook_are_offline_and_complete() {
        const MATRIX: &str = include_str!("../../../docs/fixtures/phase-51.3/safe-operator-control-adversarial-matrix.json");
        const RUNBOOK: &str = include_str!("../../../docs/release/safe-registry-operator-runbook.md");
        let matrix: serde_json::Value = serde_json::from_str(MATRIX).expect("parse matrix");
        assert_eq!(
            matrix["format"].as_str(),
            Some("clg.safe-operator-control-adversarial-matrix.v1")
        );
        assert_eq!(matrix["schema_version"].as_u64(), Some(1));
        assert_eq!(matrix["scope"].as_str(), Some("offline_only"));
        assert_eq!(
            matrix["positive_cases"].as_array().expect("positive cases").len(),
            2
        );
        let names = matrix["rejection_cases"]
            .as_array()
            .expect("rejection cases")
            .iter()
            .filter_map(|entry| entry["name"].as_str())
            .collect::<BTreeSet<_>>();
        for required in [
            "wrong_chain",
            "wrong_signer_or_approval_record",
            "inadequate_owner_approval",
            "expired_request_or_approval",
            "safe_nonce_inconsistent",
            "unsupported_fee_envelope",
            "zero_fee",
            "nonzero_value",
            "safe_version_owner_set_or_threshold_drift",
            "non_call_operation",
            "arbitrary_or_malformed_calldata",
            "release_or_policy_digest_drift",
            "owner_rotation_or_safe_governance_change",
            "unsafe_path",
            "secret_marker",
        ] {
            assert!(names.contains(required), "matrix missing {required}");
        }
        assert_eq!(
            matrix["observation_deferred_cases"]
                .as_array()
                .expect("deferred cases")
                .len(),
            2
        );
        let runbook = normalized_document_text(RUNBOOK);
        for required in [
            "Offline Preparation",
            "Two-Owner Handoff",
            "Abort and Stop",
            "Compromise Response",
            "Evidence Retention",
            "Offline Drill",
            "safe-policy-validate",
            "does not authorize",
            "break-glass key",
        ] {
            assert!(runbook.contains(required), "runbook missing {required}");
        }
        for forbidden in ["eth_sendrawtransaction", "eth_sendtransaction", "sign a transaction"] {
            assert!(!runbook.to_ascii_lowercase().contains(forbidden));
        }
    }

    #[test]
    fn milestone_six_target_observation_lock_keeps_safe_correspondence_fail_closed() {
        const FIXTURES: &str = include_str!(
            "../../../docs/fixtures/phase-52.0/safe-target-observation-lock-fixtures.json"
        );
        const LOCK: &str = include_str!(
            "../../../docs/design/phase-52.0-safe-registry-correspondence-lock.md"
        );
        let fixtures: serde_json::Value = serde_json::from_str(FIXTURES).expect("parse fixtures");
        assert_eq!(
            fixtures["format"].as_str(),
            Some("clg.safe-target-observation-lock-fixtures.v1")
        );
        assert_eq!(
            fixtures["target_profile"].as_str(),
            Some("clg.evm-ethereum-mainnet-safe-registry.v1")
        );
        assert_eq!(fixtures["chain_id"].as_str(), Some("1"));
        assert_eq!(
            fixtures["identity_chain"]
                .as_array()
                .expect("identity chain")
                .len(),
            11
        );
        assert_eq!(
            fixtures["independent_observer_rule"]["required_observers"].as_u64(),
            Some(2)
        );
        assert_eq!(
            fixtures["independent_observer_rule"]["majority_vote"].as_str(),
            Some("prohibited")
        );
        for entry in fixtures["status_cases"].as_array().expect("status cases") {
            assert_eq!(entry["accepted"].as_bool(), Some(false));
        }
        let lock = normalized_document_text(LOCK);
        for required in [
            "Safe identity",
            "ValueChanged(next)",
            "Keccak-256(RLP([sender, nonce]))",
            "12 blocks",
            "two additional observations",
            "preservation-and-stop",
            "authorizes no RPC contact",
        ] {
            assert!(lock.contains(required), "observation lock missing {required}");
        }
        for forbidden in ["eth_sendrawtransaction", "eth_sendtransaction"] {
            assert!(!lock.to_ascii_lowercase().contains(forbidden));
        }
    }

    #[test]
    fn milestone_six_evidence_replay_schema_is_secret_free_and_offline_only() {
        const FIXTURES: &str = include_str!(
            "../../../docs/fixtures/phase-52.1/safe-registry-evidence-schema-fixtures.json"
        );
        const SCHEMA: &str = include_str!(
            "../../../docs/design/phase-52.1-safe-registry-evidence-replay-schemas.md"
        );
        let fixtures: serde_json::Value = serde_json::from_str(FIXTURES).expect("parse fixtures");
        assert_eq!(
            fixtures["format"].as_str(),
            Some("clg.safe-registry-evidence-schema-fixtures.v1")
        );
        assert_eq!(
            fixtures["target_evidence_format"].as_str(),
            Some("clg.safe-registry-target-evidence.v1")
        );
        assert_eq!(
            fixtures["replay_report_format"].as_str(),
            Some("clg.safe-registry-evidence-replay-report.v1")
        );
        assert_eq!(
            fixtures["observation_kinds"].as_array().expect("observation kinds").len(),
            5
        );
        let statuses = fixtures["replay_statuses"].as_array().expect("replay statuses");
        assert_eq!(statuses.len(), 8);
        for entry in statuses {
            assert_eq!(entry["production_claim"].as_str(), Some("not_established"));
        }
        let invariants = fixtures["replay_invariants"]
            .as_array()
            .expect("replay invariants")
            .iter()
            .filter_map(|entry| entry.as_str())
            .collect::<BTreeSet<_>>();
        for required in [
            "network_contacted_false",
            "no_endpoint_wallet_key_or_override_input",
            "unknown_versions_fail_closed",
            "two_observer_agreement_required",
        ] {
            assert!(invariants.contains(required), "missing replay invariant {required}");
        }
        let schema = normalized_document_text(SCHEMA);
        for required in [
            "canonical UTF-8 JSON",
            "clg.safe-registry-identity-anchor.v1",
            "one explicit separate identity anchor",
            "identity_anchor_sha256",
            "evidence_signature is always status not_authorized",
            "network_contacted | Exactly false",
            "Unknown status or diagnostic values reject",
            "No status means deployed",
        ] {
            assert!(schema.contains(required), "evidence schema missing {required}");
        }
    }

    #[test]
    fn safe_evidence_replay_tamper_matrix_is_complete_and_never_accepts_a_target() {
        const MATRIX: &str = include_str!(
            "../../../docs/fixtures/phase-52.3/safe-registry-evidence-replay-tamper-matrix.json"
        );
        let matrix: serde_json::Value = serde_json::from_str(MATRIX).expect("parse tamper matrix");
        assert_eq!(
            matrix["format"].as_str(),
            Some("clg.safe-registry-evidence-replay-tamper-matrix.v1")
        );
        assert_eq!(matrix["schema_version"].as_u64(), Some(1));
        assert_eq!(matrix["scope"].as_str(), Some("offline_only"));
        assert_eq!(
            matrix["acceptance_boundary"].as_str(),
            Some("no_case_establishes_deployment_or_production_acceptance")
        );
        assert_eq!(
            matrix["trusted_identity_anchor_requirement"].as_str(),
            Some("valid_digest_substitution_requires_a_separate_offline_secret_free_anchor")
        );
        let cases = matrix["cases"].as_array().expect("tamper cases");
        let names = cases
            .iter()
            .filter_map(|entry| entry["name"].as_str())
            .collect::<BTreeSet<_>>();
        for required in [
            "substituted_source_profile",
            "substituted_target_profile",
            "substituted_release_bundle",
            "substituted_approval_policy",
            "substituted_constructor_layout",
            "substituted_creation_or_runtime_bytecode",
            "substituted_derived_or_registry_address",
            "substituted_state_or_event_schema",
            "substituted_receipt_or_event",
            "substituted_storage_or_client_observation",
            "omitted_confirmation_or_reorganization",
            "duplicated_observation_id",
            "reordered_observations",
            "path_traversal_capture_reference",
            "cross_chain_target",
            "stale_observation_sequence",
            "secret_bearing_evidence",
            "unknown_schema_or_version",
            "malformed_evidence",
            "missing_receipt",
            "pending_receipt",
            "reverted_receipt",
            "replaced_or_conflicting_receipt",
            "observer_disagreement",
            "detected_reorganization",
            "unauthorized_successful_transition",
        ] {
            assert!(names.contains(required), "tamper matrix missing {required}");
        }
        for case in cases {
            assert_ne!(case["expected_status"].as_str(), Some("accepted"));
            assert_ne!(case["expected_status"].as_str(), Some("replayed"));
        }
    }

    #[test]
    fn safe_independent_observation_matrix_requires_diversity_and_stop_on_disagreement() {
        const MATRIX: &str = include_str!(
            "../../../docs/fixtures/phase-52.4/safe-registry-independent-observation-matrix.json"
        );
        let matrix: serde_json::Value = serde_json::from_str(MATRIX).expect("parse observation matrix");
        assert_eq!(
            matrix["format"].as_str(),
            Some("clg.safe-registry-independent-observation-matrix.v1")
        );
        assert_eq!(matrix["schema_version"].as_u64(), Some(1));
        assert_eq!(matrix["scope"].as_str(), Some("offline_only"));
        assert_eq!(matrix["minimum_observers"].as_u64(), Some(2));
        let agreement = matrix["agreement_cases"]
            .as_array()
            .expect("agreement cases")
            .iter()
            .filter_map(|entry| entry.as_str())
            .collect::<BTreeSet<_>>();
        for required in [
            "runtime_bytecode",
            "storage",
            "value_changed_event",
            "receipt_block_and_confirmation",
            "finalized_head",
            "reorganization_window",
        ] {
            assert!(agreement.contains(required), "missing agreement case {required}");
        }
        for case in matrix["disagreement_cases"].as_array().expect("disagreement cases") {
            assert_eq!(case["expected_status"].as_str(), Some("unaccepted"));
            assert_eq!(case["expected_diagnostic"].as_str(), Some("M6C006"));
            assert_eq!(case["required_action"].as_str(), Some("preserve_and_stop"));
        }
        let prohibited = matrix["prohibited_responses"]
            .as_array()
            .expect("prohibited responses")
            .iter()
            .filter_map(|entry| entry.as_str())
            .collect::<BTreeSet<_>>();
        for response in ["majority_vote", "automatic_retry", "production_acceptance"] {
            assert!(prohibited.contains(response), "response must remain prohibited: {response}");
        }
    }

    #[test]
    fn safe_public_testnet_soak_package_is_not_started_and_secret_free() {
        const FIXTURE: &str = include_str!(
            "../../../docs/fixtures/phase-52.5/safe-registry-public-testnet-soak-decision-fixture.json"
        );
        const PACKAGE: &str = include_str!(
            "../../../docs/design/phase-52.5-public-testnet-soak-decision-package.md"
        );
        let fixture: serde_json::Value = serde_json::from_str(FIXTURE).expect("parse soak fixture");
        assert_eq!(
            fixture["format"].as_str(),
            Some("clg.safe-registry-public-testnet-soak-decision.v1")
        );
        assert_eq!(fixture["status"].as_str(), Some("not_started"));
        assert_eq!(fixture["environment"].as_str(), Some("staging"));
        assert_eq!(
            fixture["scope"].as_str(),
            Some("preparation_only_no_network")
        );
        assert_eq!(fixture["exercises"].as_array().expect("exercises").len(), 5);
        let evidence = fixture["required_evidence"]
            .as_array()
            .expect("required evidence")
            .iter()
            .filter_map(|entry| entry.as_str())
            .collect::<BTreeSet<_>>();
        for required in ["target_evidence", "identity_anchor", "offline_replay_report"] {
            assert!(evidence.contains(required), "missing soak evidence {required}");
        }
        let package = normalized_document_text(PACKAGE);
        for required in [
            "separate staging security domain",
            "not_started",
            "not a production",
            "separate pre-execution authorization",
            "no endpoint contact",
        ] {
            assert!(package.contains(required), "soak package missing {required}");
        }
    }

    #[test]
    fn production_precheck_contract_is_offline_fixed_and_defers_gate_e() {
        const FIXTURE: &str = include_str!(
            "../../../docs/fixtures/phase-53.0/production-precheck-report-fixture.json"
        );
        const LOCK: &str = include_str!(
            "../../../docs/design/phase-53.0-production-precheck-report-lock.md"
        );
        let fixture: serde_json::Value = serde_json::from_str(FIXTURE).expect("parse precheck fixture");
        assert_eq!(
            fixture["report_kind"].as_str(),
            Some("clg.production-precheck-report.v1")
        );
        assert_eq!(fixture["report_schema_version"].as_u64(), Some(1));
        assert_eq!(fixture["scope"].as_str(), Some("candidate_evidence"));
        assert_eq!(
            fixture["gate_order"].as_array().expect("gate order").len(),
            6
        );
        let statuses = fixture["statuses"]
            .as_array()
            .expect("statuses")
            .iter()
            .filter_map(|entry| entry.as_str())
            .collect::<BTreeSet<_>>();
        for status in ["passed", "failed", "blocked", "not_run"] {
            assert!(statuses.contains(status), "missing precheck status {status}");
        }
        assert_eq!(fixture["gate_e"]["independent_review"].as_str(), Some("not_run"));
        assert_eq!(fixture["gate_e"]["launch_decision"].as_str(), Some("not_run"));
        let lock = normalized_document_text(LOCK);
        for required in [
            "--offline --locked",
            "no gate may open a network connection",
            "public-testnet-soak-decision",
            "M6D005",
            "Gate E",
        ] {
            assert!(lock.contains(required), "precheck lock missing {required}");
        }
    }

    #[test]
    fn production_precheck_report_is_canonical_and_blocks_unstarted_soak() {
        let opts = parse_production_precheck_args(vec![
            "--report".to_string(),
            "out/production-precheck.json".to_string(),
        ])
        .expect("parse production precheck args");
        assert_eq!(opts.report, PathBuf::from("out/production-precheck.json"));
        assert!(parse_production_precheck_args(vec!["--skip".to_string()]).is_err());

        let root = repo_root();
        let mut report = new_production_precheck_report(root.as_path())
            .expect("build production precheck report");
        for gate in report.gates.iter_mut().take(4) {
            gate.status = "passed".to_string();
        }
        report.gates[4].status = "blocked".to_string();
        report.gates[4].diagnostic = Some("M6D005".to_string());
        report.gates[5].status = "not_run".to_string();
        report.overall_status = "blocked".to_string();
        validate_production_precheck_report_against_root(root.as_path(), &report)
            .expect("validate blocked production precheck report");
        assert_eq!(report.gate_e.independent_review, "not_run");
        assert_eq!(report.gate_e.launch_decision, "not_run");
    }

    #[test]
    fn production_precheck_validation_rejects_fail_fast_and_diagnostic_tampering() {
        let root = repo_root();
        let mut report = new_production_precheck_report(root.as_path())
            .expect("build production precheck report");
        for gate in report.gates.iter_mut().take(4) {
            gate.status = "passed".to_string();
        }
        report.gates[4].status = "blocked".to_string();
        report.gates[4].diagnostic = Some("M6D005".to_string());
        report.gates[5].status = "not_run".to_string();
        report.overall_status = "blocked".to_string();

        validate_production_precheck_report_against_root(root.as_path(), &report)
            .expect("known blocked report is valid");

        let mut later_gate_ran = report.clone();
        later_gate_ran.gates[5].status = "passed".to_string();
        assert!(validate_production_precheck_report(&later_gate_ran).is_err());

        let mut missing_terminal_diagnostic = report.clone();
        missing_terminal_diagnostic.gates[4].diagnostic = None;
        assert!(validate_production_precheck_report(&missing_terminal_diagnostic).is_err());

        let mut unknown_diagnostic = report.clone();
        unknown_diagnostic.gates[4].diagnostic = Some("M6D999".to_string());
        assert!(validate_production_precheck_report(&unknown_diagnostic).is_err());

        let mut premature_not_run = report;
        premature_not_run.gates[2].status = "not_run".to_string();
        premature_not_run.gates[2].diagnostic = None;
        assert!(validate_production_precheck_report(&premature_not_run).is_err());
    }

    #[test]
    fn production_precheck_validation_rejects_identity_order_path_and_content_tampering() {
        let root = repo_root();
        let mut report = new_production_precheck_report(root.as_path())
            .expect("build production precheck report");
        for gate in report.gates.iter_mut().take(4) {
            gate.status = "passed".to_string();
        }
        report.gates[4].status = "blocked".to_string();
        report.gates[4].diagnostic = Some("M6D005".to_string());
        report.gates[5].status = "not_run".to_string();
        report.overall_status = "blocked".to_string();

        let mut reordered = report.clone();
        reordered.gates.swap(0, 1);
        assert!(validate_production_precheck_report(&reordered).is_err());

        let mut duplicated = report.clone();
        duplicated.gates[5].id = duplicated.gates[4].id.clone();
        assert!(validate_production_precheck_report(&duplicated).is_err());

        let mut unsafe_path = report.clone();
        unsafe_path.identity[0].id = "../outside.json".to_string();
        assert!(validate_production_precheck_report(&unsafe_path).is_err());

        let mut secret_bearing = report.clone();
        secret_bearing.gates[0].command.push("private_key".to_string());
        assert!(validate_production_precheck_report(&secret_bearing).is_err());

        let mut unsupported_version = report.clone();
        unsupported_version.schema_version = 2;
        assert!(validate_production_precheck_report(&unsupported_version).is_err());

        let mut cross_profile = report.clone();
        cross_profile.candidate_profile = "clg.evm-ethereum-mainnet-safe-registry.v1:11155111".to_string();
        assert!(validate_production_precheck_report(&cross_profile).is_err());

        let mut substituted_digest = report;
        substituted_digest.identity[0].sha256 = "0".repeat(64);
        assert!(validate_production_precheck_report_against_root(root.as_path(), &substituted_digest).is_err());
    }

    #[test]
    fn production_precheck_tamper_matrix_is_complete_and_fail_closed() {
        const MATRIX: &str = include_str!(
            "../../../docs/fixtures/phase-53.3/production-precheck-tamper-matrix.json"
        );
        let matrix: serde_json::Value = serde_json::from_str(MATRIX).expect("parse tamper matrix");
        assert_eq!(
            matrix["format"].as_str(),
            Some("clg.production-precheck-tamper-matrix.v1")
        );
        assert_eq!(matrix["schema_version"].as_u64(), Some(1));
        assert_eq!(matrix["scope"].as_str(), Some("offline_only"));
        let cases = matrix["cases"].as_array().expect("tamper cases");
        assert_eq!(cases.len(), 10);
        let names = cases
            .iter()
            .filter_map(|case| case["name"].as_str())
            .collect::<BTreeSet<_>>();
        for required in [
            "reordered_or_duplicated_gate",
            "later_gate_after_terminal_result",
            "unsafe_or_secret_bearing_input",
            "substituted_or_cross_profile_identity",
            "stale_or_mismatched_release_evidence",
            "unsupported_support_matrix",
            "unavailable_public_soak",
        ] {
            assert!(names.contains(required), "missing tamper case {required}");
        }
        for case in cases {
            assert!(
                matches!(
                    case["expected_status"].as_str(),
                    Some("rejected") | Some("blocked")
                ),
                "tamper matrix must fail closed"
            );
            assert!(
                matches!(
                    case["expected_diagnostic"].as_str(),
                    Some("M6D001" | "M6D002" | "M6D003" | "M6D004" | "M6D005")
                ),
                "tamper matrix must use a stable diagnostic"
            );
        }
    }

    #[test]
    fn independent_user_documentation_keeps_the_production_verifier_boundary_explicit() {
        const FIXTURE: &str = include_str!(
            "../../../docs/fixtures/phase-53.4/independent-user-documentation-fixtures.json"
        );
        const LOCK: &str = include_str!(
            "../../../docs/design/phase-53.4-independent-user-documentation-lock.md"
        );
        const VERIFIER: &str = include_str!(
            "../../../docs/release/production-verifier-offline-walkthrough.md"
        );
        const OPERATOR: &str = include_str!(
            "../../../docs/release/production-safe-registry-operator-walkthrough.md"
        );
        let fixture: serde_json::Value = serde_json::from_str(FIXTURE).expect("parse documentation fixture");
        assert_eq!(
            fixture["format"].as_str(),
            Some("clg.independent-user-documentation-fixture.v1")
        );
        assert_eq!(fixture["scope"].as_str(), Some("offline_only"));
        assert_eq!(
            fixture["verifier"]["required_future_result"]["target_profile"].as_str(),
            Some("clg.evm-ethereum-mainnet-safe-registry.v1")
        );
        assert_eq!(
            fixture["verifier"]["current_status"].as_str(),
            Some("blocked_unsupported_profile_version")
        );
        assert_eq!(fixture["operator"]["current_precheck_status"].as_str(), Some("blocked"));
        assert_eq!(
            fixture["operator"]["current_precheck_diagnostic"].as_str(),
            Some("M6D005")
        );
        assert_eq!(fixture["retention"]["minimum_years"].as_u64(), Some(7));
        assert_eq!(fixture["retention"]["secret_free_only"].as_bool(), Some(true));
        let commands = fixture["operator"]["commands"].as_array().expect("operator commands");
        assert_eq!(commands.len(), 2);
        for command in commands {
            let joined = command
                .as_array()
                .expect("command tokens")
                .iter()
                .filter_map(|token| token.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            assert!(joined.contains("--offline --locked"), "command must be offline and locked");
        }

        let lock = normalized_document_text(LOCK);
        for required in ["not a production-profile verifier", "not interchangeable", "no source checkout"] {
            assert!(lock.contains(required), "documentation lock missing {required}");
        }
        let verifier = normalized_document_text(VERIFIER);
        for required in [
            "source checkout",
            "private key",
            "unsupported profile/version",
            "must reject a mainnet Safe-registry candidate",
            "does not authorize",
        ] {
            assert!(verifier.contains(required), "verifier walkthrough missing {required}");
        }
        let operator = normalized_document_text(OPERATOR);
        for required in [
            "M6D005",
            "separately authorized Phase 52.5 public-testnet soak",
            "at least seven years",
            "No current document authorizes a mainnet action",
        ] {
            assert!(operator.contains(required), "operator walkthrough missing {required}");
        }
    }

    #[test]
    fn independent_review_handoff_is_secret_free_frozen_and_not_commissioned() {
        const FIXTURE: &str = include_str!(
            "../../../docs/fixtures/phase-54.0/independent-review-handoff-fixture.json"
        );
        const SCOPE: &str = include_str!(
            "../../../docs/design/phase-54.0-independent-review-commissioning-lock.md"
        );
        let fixture: serde_json::Value = serde_json::from_str(FIXTURE).expect("parse review handoff fixture");
        assert_eq!(
            fixture["format"].as_str(),
            Some("clg.independent-review-handoff-fixture.v1")
        );
        assert_eq!(fixture["schema_version"].as_u64(), Some(1));
        assert_eq!(fixture["status"].as_str(), Some("not_commissioned"));
        assert_eq!(
            fixture["candidate_profile"].as_str(),
            Some("clg.evm-ethereum-mainnet-safe-registry.v1")
        );
        assert_eq!(fixture["chain_id"].as_str(), Some("1"));
        let artifacts = fixture["artifacts"].as_array().expect("handoff artifacts");
        assert_eq!(artifacts.len(), 8);
        for artifact in artifacts {
            assert!(artifact["id"].as_str().is_some_and(|id| !id.contains("..") && !id.contains('\\')));
            assert!(artifact["sha256"].as_str().is_some_and(is_lower_hex_64));
        }
        let blockers = fixture["blockers"]
            .as_array()
            .expect("review blockers")
            .iter()
            .filter_map(|blocker| blocker.as_str())
            .collect::<BTreeSet<_>>();
        for required in [
            "production_verifier_unavailable",
            "public_testnet_soak_not_started",
            "independent_review_not_commissioned",
        ] {
            assert!(blockers.contains(required), "missing review blocker {required}");
        }
        let scope = normalized_document_text(SCOPE);
        for required in [
            "not_commissioned",
            "unresolved critical or high finding",
            "30 calendar days",
            "not approved",
            "private key",
        ] {
            assert!(scope.contains(required), "review scope missing {required}");
        }
    }

    #[test]
    fn independent_review_findings_template_stays_unstarted_and_blocks_launch() {
        const TEMPLATE: &str = include_str!(
            "../../../docs/fixtures/phase-54.1/independent-review-findings-template.json"
        );
        const CONTRACT: &str = include_str!(
            "../../../docs/design/phase-54.1-independent-review-findings-disposition.md"
        );
        let template: serde_json::Value = serde_json::from_str(TEMPLATE).expect("parse findings template");
        assert_eq!(
            template["format"].as_str(),
            Some("clg.independent-review-findings.v1")
        );
        assert_eq!(template["schema_version"].as_u64(), Some(1));
        assert_eq!(template["status"].as_str(), Some("not_started"));
        assert!(template["reviewer"].is_null());
        assert!(template["conflict_disclosure"].is_null());
        assert_eq!(template["findings"].as_array().map(Vec::len), Some(0));
        assert_eq!(template["launch_effect"].as_str(), Some("not_approved"));
        assert!(template["handoff"]["sha256"].as_str().is_some_and(is_lower_hex_64));
        let blockers = template["blockers"]
            .as_array()
            .expect("findings blockers")
            .iter()
            .filter_map(|blocker| blocker.as_str())
            .collect::<BTreeSet<_>>();
        for required in [
            "independent_review_not_commissioned",
            "production_verifier_unavailable",
            "public_testnet_soak_not_started",
            "production_precheck_M6D005",
        ] {
            assert!(blockers.contains(required), "missing findings blocker {required}");
        }
        let contract = normalized_document_text(CONTRACT);
        for required in [
            "unresolved critical/high",
            "not approved",
            "not_started",
            "cannot override a failed proof",
        ] {
            assert!(contract.contains(required), "findings contract missing {required}");
        }
    }

    #[test]
    fn independent_clean_reproduction_decision_requires_real_production_inputs() {
        const DECISION: &str = include_str!(
            "../../../docs/fixtures/phase-54.2/independent-clean-reproduction-decision.json"
        );
        const CONTRACT: &str = include_str!(
            "../../../docs/design/phase-54.2-independent-clean-environment-reproduction.md"
        );
        let decision: serde_json::Value = serde_json::from_str(DECISION).expect("parse reproduction decision");
        assert_eq!(
            decision["format"].as_str(),
            Some("clg.independent-clean-reproduction-decision.v1")
        );
        assert_eq!(decision["schema_version"].as_u64(), Some(1));
        assert_eq!(decision["status"].as_str(), Some("not_started"));
        assert_eq!(decision["launch_effect"].as_str(), Some("not_approved"));
        assert_eq!(
            decision["candidate"]["profile"].as_str(),
            Some("clg.evm-ethereum-mainnet-safe-registry.v1")
        );
        assert_eq!(decision["candidate"]["chain_id"].as_str(), Some("1"));
        let blockers = decision["blockers"]
            .as_array()
            .expect("reproduction blockers")
            .iter()
            .filter_map(|blocker| blocker.as_str())
            .collect::<BTreeSet<_>>();
        for required in [
            "production_verifier_unavailable",
            "public_testnet_soak_not_started",
            "independent_review_not_commissioned",
            "production_precheck_M6D005",
        ] {
            assert!(blockers.contains(required), "missing reproduction blocker {required}");
        }
        let fallbacks = decision["prohibited_fallbacks"]
            .as_array()
            .expect("prohibited fallbacks")
            .iter()
            .filter_map(|fallback| fallback.as_str())
            .collect::<BTreeSet<_>>();
        for forbidden in ["source_checkout", "cargo_build", "sepolia_verifier", "network_fetch"] {
            assert!(fallbacks.contains(forbidden), "fallback must remain prohibited: {forbidden}");
        }
        let contract = normalized_document_text(CONTRACT);
        for required in [
            "new local directory",
            "source build",
            "Any difference",
            "does not bypass",
        ] {
            assert!(contract.contains(required), "reproduction contract missing {required}");
        }
    }

    #[test]
    fn pre_launch_decision_is_explicitly_not_approved_with_live_blockers() {
        const DECISION: &str = include_str!(
            "../../../docs/fixtures/phase-54.3/pre-launch-decision.json"
        );
        const CONTRACT: &str = include_str!(
            "../../../docs/design/phase-54.3-pre-launch-decision-contract.md"
        );
        let decision: serde_json::Value = serde_json::from_str(DECISION).expect("parse pre-launch decision");
        assert_eq!(decision["format"].as_str(), Some("clg.pre-launch-decision.v1"));
        assert_eq!(decision["schema_version"].as_u64(), Some(1));
        assert_eq!(decision["decision"].as_str(), Some("not_approved"));
        assert_eq!(decision["candidate"]["chain_id"].as_str(), Some("1"));
        assert_eq!(decision["candidate"]["asset_exposure_wei"].as_str(), Some("0"));
        assert_eq!(decision["support_snapshot"]["rust_cargo"].as_str(), Some("1.89.0"));
        assert_eq!(decision["support_snapshot"]["solver"].as_str(), Some("z3-4.16.0"));
        assert_eq!(
            decision["support_snapshot"]["verifier"]["status"].as_str(),
            Some("not_distributed_for_production_profile")
        );
        assert!(decision["support_snapshot"]["verifier"]["supported_version"].is_null());
        assert_eq!(decision["support_snapshot"]["safe"]["version"].as_str(), Some("1.4.1"));
        assert_eq!(decision["support_snapshot"]["safe"]["threshold"].as_str(), Some("2-of-3"));
        assert_eq!(
            decision["operational_state"]["monitoring_owner"].as_str(),
            Some("not_assigned_no_launch_permitted")
        );
        let inputs = decision["bound_inputs"].as_array().expect("bound inputs");
        assert_eq!(inputs.len(), 3);
        for input in inputs {
            assert!(input["id"].as_str().is_some_and(|id| !id.contains("..") && !id.contains('\\')));
            assert!(input["sha256"].as_str().is_some_and(is_lower_hex_64));
        }
        let blockers = decision["blockers"]
            .as_array()
            .expect("launch blockers")
            .iter()
            .filter_map(|blocker| blocker.as_str())
            .collect::<BTreeSet<_>>();
        for required in [
            "production_verifier_unavailable",
            "public_testnet_soak_not_started",
            "production_precheck_M6D005",
            "independent_review_not_commissioned",
            "independent_clean_reproduction_not_started",
        ] {
            assert!(blockers.contains(required), "missing launch blocker {required}");
        }
        let prohibited = decision["prohibited_actions"]
            .as_array()
            .expect("prohibited actions")
            .iter()
            .filter_map(|action| action.as_str())
            .collect::<BTreeSet<_>>();
        for action in ["signing", "submission", "broadcast", "deployment", "invocation"] {
            assert!(prohibited.contains(action), "action must be prohibited: {action}");
        }
        let contract = normalized_document_text(CONTRACT);
        for required in [
            "Current decision is `not approved`",
            "before Phase 54.4",
            "does not close Milestone 6",
            "no mainnet transaction",
        ] {
            assert!(contract.contains(required), "pre-launch contract missing {required}");
        }
    }

    #[test]
    fn production_support_policy_is_pinned_and_fail_closed() {
        const FIXTURE: &str = include_str!(
            "../../../docs/fixtures/phase-53.1/production-support-compatibility-fixture.json"
        );
        const POLICY: &str = include_str!(
            "../../../docs/design/phase-53.1-production-support-compatibility-policy.md"
        );
        let fixture: serde_json::Value = serde_json::from_str(FIXTURE).expect("parse support fixture");
        assert_eq!(
            fixture["format"].as_str(),
            Some("clg.production-support-compatibility-fixture.v1")
        );
        assert_eq!(fixture["toolchain"]["rust_cargo"].as_str(), Some("1.89.0"));
        assert_eq!(fixture["toolchain"]["solver"]["family"].as_str(), Some("z3"));
        assert_eq!(fixture["toolchain"]["solver"]["version"].as_str(), Some("4.16.0"));
        assert_eq!(fixture["target"]["chain_id"].as_str(), Some("1"));
        assert_eq!(fixture["target"]["safe_version"].as_str(), Some("1.4.1"));
        assert_eq!(fixture["observation"]["required_observers"].as_u64(), Some(2));
        assert_eq!(fixture["retention"]["minimum_years"].as_u64(), Some(7));
        let prohibited = fixture["prohibited_compatibility"]
            .as_array()
            .expect("prohibited compatibility")
            .iter()
            .filter_map(|entry| entry.as_str())
            .collect::<BTreeSet<_>>();
        for required in ["automatic_upgrade", "downgrade", "fallback"] {
            assert!(prohibited.contains(required), "must reject {required}");
        }
        let policy = normalized_document_text(POLICY);
        for required in [
            "candidate support surface",
            "14-day maximum advisory-age policy",
            "at least seven years",
            "No automatic",
            "does not permit transaction cancellation",
        ] {
            assert!(policy.contains(required), "support policy missing {required}");
        }
    }

    #[test]
    fn safe_evidence_capture_and_replay_are_local_and_fail_closed() {
        let root = unique_temp_dir("safe-evidence-capture");
        std::fs::create_dir_all(root.join("records")).expect("create record directory");
        let identity = SafeEvidenceIdentity {
            source_profile_proof_sha256: "a".repeat(64),
            state_schema_abi_sha256: "b".repeat(64),
            evm_artifact_sha256: "c".repeat(64),
            release_bundle_sha256: "d".repeat(64),
            policy_sha256: "e".repeat(64),
            safe_identity_sha256: "f".repeat(64),
            observer_policy_version: "1".to_string(),
        };
        let observation = SafeEvidenceObservation {
            format: SAFE_EVIDENCE_OBSERVATION_FORMAT.to_string(),
            schema_version: 1,
            id: "01-deployment-finality".to_string(),
            kind: "deployment".to_string(),
            subject_action_id: "deploy-1".to_string(),
            captured_at: "2030-01-02T03:04:05Z".to_string(),
            primary_observer: SafeObserver {
                observer_id: "submission-observer".to_string(),
                observer_class: "primary".to_string(),
                client_family: "fixture-client-a".to_string(),
                client_version: "1".to_string(),
                exchange_digests: vec!["1".repeat(64)],
            },
            independent_observer: SafeObserver {
                observer_id: "verification-observer".to_string(),
                observer_class: "independent".to_string(),
                client_family: "fixture-client-b".to_string(),
                client_version: "1".to_string(),
                exchange_digests: vec!["2".repeat(64)],
            },
            block_number: "12".to_string(),
            block_hash: format!("0x{}", "a".repeat(64)),
            status: "qualified_finality_observation".to_string(),
            comparison_result: "match".to_string(),
        };
        let confirmation = SafeEvidenceObservation {
            id: "02-confirmation".to_string(),
            kind: "confirmation".to_string(),
            ..observation.clone()
        };
        let reorganization_observation = SafeEvidenceObservation {
            id: "03-reorganization-observation".to_string(),
            kind: "reorganization_observation".to_string(),
            ..observation.clone()
        };
        for (path, record) in [
            ("records/deployment.json", &observation),
            ("records/confirmation.json", &confirmation),
            (
                "records/reorganization-observation.json",
                &reorganization_observation,
            ),
        ] {
            std::fs::write(
                root.join(path),
                canonical_json_bytes(record).expect("canonical observation"),
            )
            .expect("write observation");
        }
        let input = SafeEvidenceCaptureInput {
            format: SAFE_EVIDENCE_CAPTURE_INPUT_FORMAT.to_string(),
            schema_version: 1,
            target: SafeEvidenceTarget {
                profile: SAFE_EVIDENCE_PROFILE.to_string(),
                chain_id: "1".to_string(),
            },
            identity,
            observations: vec![
                SafeEvidenceCaptureRef {
                    id: observation.id.clone(),
                    record_path: "records/deployment.json".to_string(),
                },
                SafeEvidenceCaptureRef {
                    id: confirmation.id.clone(),
                    record_path: "records/confirmation.json".to_string(),
                },
                SafeEvidenceCaptureRef {
                    id: reorganization_observation.id.clone(),
                    record_path: "records/reorganization-observation.json".to_string(),
                },
            ],
        };
        std::fs::write(
            root.join("capture.json"),
            canonical_json_bytes(&input).expect("canonical capture input"),
        )
        .expect("write capture input");
        let evidence_path = root.join("evidence.json");
        run_safe_evidence_capture(
            repo_root().as_path(),
            vec![
                "--root".to_string(),
                root.to_string_lossy().into_owned(),
                "--input".to_string(),
                "capture.json".to_string(),
                "--out".to_string(),
                "evidence.json".to_string(),
            ],
        )
        .expect("capture evidence");
        let replay_path = root.join("replay.json");
        let anchor_path = root.join("identity-anchor.json");
        let anchor = SafeEvidenceIdentityAnchor {
            format: SAFE_IDENTITY_ANCHOR_FORMAT.to_string(),
            schema_version: 1,
            target: input.target.clone(),
            identity: input.identity.clone(),
            verified_release_bundle_sha256: input.identity.release_bundle_sha256.clone(),
            verifier_result_sha256: "7".repeat(64),
            verifier_identity_sha256: "8".repeat(64),
            trusted_release_key_id: "fixture-release-key".to_string(),
        };
        std::fs::write(
            &anchor_path,
            canonical_json_bytes(&anchor).expect("canonical identity anchor"),
        )
        .expect("write identity anchor");
        run_safe_evidence_replay(
            repo_root().as_path(),
            vec![
                "--input".to_string(),
                evidence_path.to_string_lossy().into_owned(),
                "--anchor".to_string(),
                anchor_path.to_string_lossy().into_owned(),
                "--report-out".to_string(),
                replay_path.to_string_lossy().into_owned(),
            ],
        )
        .expect("replay evidence");
        let replay: SafeReplayReport = serde_json::from_slice(
            &std::fs::read(&replay_path).expect("read replay report"),
        )
        .expect("parse replay report");
        assert_eq!(replay.status, "replayed");
        assert_eq!(replay.diagnostic, "M6C000");
        assert_eq!(
            replay.identity_anchor_sha256,
            Some(safe_policy_sha256_hex(
                &std::fs::read(&anchor_path).expect("read identity anchor"),
            ))
        );
        assert!(!replay.network_contacted);
        assert_eq!(replay.production_claim, "not_established");

        let evidence: SafeTargetEvidence = serde_json::from_slice(
            &std::fs::read(&evidence_path).expect("read evidence"),
        )
        .expect("parse evidence");
        let mut mismatched_anchor = anchor.clone();
        mismatched_anchor.identity.source_profile_proof_sha256 = "9".repeat(64);
        let mismatched_anchor_path = root.join("mismatched-identity-anchor.json");
        std::fs::write(
            &mismatched_anchor_path,
            canonical_json_bytes(&mismatched_anchor).expect("canonical mismatched anchor"),
        )
        .expect("write mismatched anchor");
        assert_eq!(
            replay_safe_target_evidence(&evidence_path, &mismatched_anchor_path)
                .expect_err("mismatched anchor must reject")
                .code,
            "M6C003"
        );
        let replay_error = |name: &str, candidate: &SafeTargetEvidence| {
            let path = root.join(name);
            std::fs::write(
                &path,
                canonical_json_bytes(candidate).expect("canonical tamper evidence"),
            )
            .expect("write tamper evidence");
            replay_safe_target_evidence(&path, &anchor_path)
                .expect_err("tampered evidence must reject")
                .code
        };
        let mut wrong_profile = evidence.clone();
        wrong_profile.target.profile = "clg.other-profile.v1".to_string();
        assert_eq!(replay_error("wrong-profile.json", &wrong_profile), "M6C002");
        let mut malformed_identity = evidence.clone();
        malformed_identity.identity.release_bundle_sha256 = "g".repeat(64);
        assert_eq!(
            replay_error("malformed-identity.json", &malformed_identity),
            "M6C003"
        );
        let mut substituted_identity = evidence.clone();
        substituted_identity.identity.release_bundle_sha256 = "9".repeat(64);
        assert_eq!(
            replay_error("substituted-identity.json", &substituted_identity),
            "M6C003"
        );
        let mut missing_reorganization = evidence.clone();
        missing_reorganization.observations.pop();
        assert_eq!(
            replay_error("missing-reorganization.json", &missing_reorganization),
            "M6C004"
        );
        let mut duplicate_id = evidence.clone();
        duplicate_id.observations[1].id = duplicate_id.observations[0].id.clone();
        assert_eq!(replay_error("duplicate-id.json", &duplicate_id), "M6C001");
        let mut reordered = evidence.clone();
        reordered.observations.reverse();
        assert_eq!(replay_error("reordered.json", &reordered), "M6C001");
        let mut secret_bearing = evidence.clone();
        secret_bearing.observations[0].primary_observer.client_version = "private_key".to_string();
        assert_eq!(
            replay_error("secret-bearing.json", &secret_bearing),
            "M6C001"
        );
        let mut stale = evidence.clone();
        stale.observations[1].captured_at = "2030-01-02T03:04:04Z".to_string();
        assert_eq!(replay_error("stale.json", &stale), "M6C005");
        for status in [
            "missing_receipt",
            "pending_receipt",
            "reverted_receipt",
            "replaced_or_conflicting",
        ] {
            let mut receipt_failure = evidence.clone();
            receipt_failure.observations[0].status = status.to_string();
            assert_eq!(
                replay_error(&format!("{status}.json"), &receipt_failure),
                "M6C005"
            );
        }
        let mut disagreement = evidence.clone();
        disagreement.observations[0].independent_observer.observer_id = disagreement.observations[0]
            .primary_observer
            .observer_id
            .clone();
        assert_eq!(replay_error("disagreement.json", &disagreement), "M6C006");
        let mut same_client_family = evidence.clone();
        same_client_family.observations[0].independent_observer.client_family = same_client_family
            .observations[0]
            .primary_observer
            .client_family
            .clone();
        assert_eq!(
            replay_error("same-client-family.json", &same_client_family),
            "M6C006"
        );
        let mut reorganization = evidence.clone();
        reorganization.observations[2].status = "observer_disagreement_or_reorg".to_string();
        assert_eq!(replay_error("reorganization.json", &reorganization), "M6C006");
        let mut unauthorized_success = evidence.clone();
        unauthorized_success.observations[0].kind = "expected_unauthorized_invocation".to_string();
        assert_eq!(
            replay_error("unauthorized-success.json", &unauthorized_success),
            "M6C007"
        );

        let mut unsafe_path_input = input.clone();
        unsafe_path_input.observations[0].record_path = "../outside.json".to_string();
        std::fs::write(
            root.join("capture-path-traversal.json"),
            canonical_json_bytes(&unsafe_path_input).expect("canonical unsafe capture input"),
        )
        .expect("write unsafe capture input");
        assert!(run_safe_evidence_capture(
            repo_root().as_path(),
            vec![
                "--root".to_string(),
                root.to_string_lossy().into_owned(),
                "--input".to_string(),
                "capture-path-traversal.json".to_string(),
                "--out".to_string(),
                "path-traversal-output.json".to_string(),
            ],
        )
        .is_err());

        let mut evidence = evidence;
        evidence.observations[0].status = "pending_receipt".to_string();
        std::fs::write(
            &evidence_path,
            canonical_json_bytes(&evidence).expect("canonical pending evidence"),
        )
        .expect("write pending evidence");
        let rejected_replay_path = root.join("replay-rejected.json");
        assert!(run_safe_evidence_replay(
            repo_root().as_path(),
            vec![
                "--input".to_string(),
                evidence_path.to_string_lossy().into_owned(),
                "--anchor".to_string(),
                anchor_path.to_string_lossy().into_owned(),
                "--report-out".to_string(),
                rejected_replay_path.to_string_lossy().into_owned(),
            ],
        )
        .is_err());
        let rejected: SafeReplayReport = serde_json::from_slice(
            &std::fs::read(&rejected_replay_path).expect("read rejected replay"),
        )
        .expect("parse rejected replay");
        assert_eq!(rejected.status, "unaccepted");
        assert_eq!(rejected.diagnostic, "M6C005");
        assert!(!rejected.network_contacted);
        cleanup_temp_dir(root.as_path());
    }

    #[test]
    fn pilot_precheck_report_destination_is_new_and_safe() {
        let root = repo_root();
        let accepted = root.join("out").join("pilot-precheck-test-report.json");
        assert!(validate_pilot_precheck_report_destination(root.as_path(), &accepted).is_ok());
        assert!(validate_pilot_precheck_report_destination(root.as_path(), &root.join("Cargo.toml"))
            .is_err());
        assert!(validate_pilot_precheck_report_destination(
            root.as_path(),
            Path::new("../pilot-precheck-report.json")
        )
        .is_err());
        assert!(validate_pilot_precheck_report_destination(
            root.as_path(),
            &root.join("docs").join("pilot-precheck-report.json")
        )
        .is_err());
    }

    #[test]
    fn pilot_local_toolchain_support_lock_is_complete_and_pinned() {
        const POLICY: &str = include_str!("../../../docs/design/phase-40.2.1-local-toolchain-support-lock.md");
        const TOOLCHAIN: &str = include_str!("../../../rust-toolchain.toml");
        const LOCKFILE: &str = include_str!("../../../Cargo.lock");
        for required in [
            "cargo run --offline --locked -p xtask -- pilot-precheck --report out/pilot-precheck-full.json",
            "windows-x64",
            "linux-x64-glibc-2.39",
            "macos-x64-15.7.3",
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "Z3 `4.16.0`",
            "clg.evm-sepolia-pilot.v1",
            "clg.evm-stateful-scalar.v1",
            "clg.contract-release-verifier-result.v1",
            "clg.pilot-precheck-report.v2",
            "Remote automation is informational only.",
        ] {
            assert!(
                POLICY.contains(required),
                "pilot local toolchain support lock is missing `{required}`"
            );
        }
        assert!(TOOLCHAIN.contains("channel = \"1.89.0\""));
        assert!(TOOLCHAIN.contains("components = [\"clippy\", \"rustfmt\"]"));
        assert!(POLICY.contains("version-controlled `Cargo.lock`"));
        assert!(LOCKFILE.contains("name = \"keccak\""));
        assert!(LOCKFILE.contains("version = \"0.1.5\""));
    }

    #[test]
    fn pilot_evidence_compatibility_and_rollback_lock_is_fail_closed() {
        const POLICY: &str = include_str!(
            "../../../docs/design/phase-40.2.2-local-evidence-compatibility-and-rollback-lock.md"
        );
        for required in [
            "or it is rejected with an actionable deterministic explanation",
            "Reproduce; do not reuse the old acceptance report",
            "Remote output is informational and has no acceptance authority.",
            "falling back to a nearby version is not an action",
            "new committed compatibility lock",
            "never silently selected as the rollback target",
            "rollback decision remains local and explicit.",
        ] {
            assert!(
                POLICY.contains(required),
                "pilot evidence compatibility lock is missing `{required}`"
            );
        }
    }

    #[test]
    fn milestone_five_pilot_documents_keep_the_bounded_product_contract() {
        const ROADMAP: &str = include_str!("../../../docs/todo/milestone_5_roadmap.md");
        const PRODUCT: &str = include_str!(
            "../../../docs/release/milestone_5_pilot_product_contract.md"
        );
        const RELEASE_PROCESS: &str = include_str!("../../../docs/release-process.md");
        const VERIFIER: &str = include_str!(
            "../../../docs/release/contract-verifier-offline-walkthrough.md"
        );
        const RUNBOOK: &str = include_str!("../../../docs/release/pilot-operator-runbook.md");
        const INCIDENT: &str = include_str!(
            "../../../docs/release/milestone_5_pilot_incident_and_rollback.md"
        );
        const CLOSURE: &str = include_str!(
            "../../../docs/release/milestone_5_pilot_closure_decision.md"
        );
        const BOUNDARY: &str = "locally validated, testnet-only pilot";
        const EXCLUSIONS: &str =
            "must not be described as generic EVM support, remote-CI certification, mainnet readiness, or custody safety";

        for (name, document) in [
            ("roadmap", ROADMAP),
            ("product contract", PRODUCT),
            ("release process", RELEASE_PROCESS),
            ("verifier walkthrough", VERIFIER),
            ("pilot runbook", RUNBOOK),
            ("incident guide", INCIDENT),
            ("closure decision", CLOSURE),
        ] {
            let normalized = normalized_document_text(document);
            assert!(normalized.contains(BOUNDARY), "{name} is missing pilot boundary");
            assert!(
                normalized.contains(EXCLUSIONS),
                "{name} is missing bounded-pilot exclusions"
            );
        }

        for required in [
            "cargo run --offline --locked -p xtask -- pilot-precheck --report out/pilot-precheck-full.json",
            "clg.evm-sepolia-pilot.v1",
            "clg.evm-stateful-scalar.v1",
            "clg.contract-release-verifier-result.v1",
            "clg.pilot-precheck-report.v2",
            "No command may infer an endpoint, chain ID, signer, nonce, fee, gas policy",
        ] {
            assert!(
                PRODUCT.contains(required),
                "product contract is missing required locked value `{required}`"
            );
        }
        for required in [
            "## Failed release verification",
            "## Wrong-chain submission attempt",
            "## Unexpected receipt or target behavior",
            "## Leaked or suspected leaked test key",
            "## Discovered artifact or evidence mismatch",
            "chain-side remediation",
        ] {
            assert!(
                INCIDENT.contains(required),
                "incident guide is missing required procedure `{required}`"
            );
        }
        assert!(ROADMAP.contains("- [x] 44.0 Publish the final Milestone 5 product contract"));
        assert!(ROADMAP.contains("- [x] 44.0.3 Add document consistency checks"));
    }

    #[test]
    fn milestone_six_safe_owner_profile_keeps_qualified_claims_and_no_target_authority() {
        const ROADMAP: &str = include_str!("../../../docs/todo/milestone_6_roadmap.md");
        const DIRECT_LOCK: &str = include_str!(
            "../../../docs/design/phase-50.2-ethereum-mainnet-target-profile-lock.md"
        );
        const SAFE_AMENDMENT: &str = include_str!(
            "../../../docs/design/phase-50.2-safe-owner-amendment.md"
        );
        const FIXTURE: &str = include_str!(
            "../../../docs/fixtures/phase-50.2/ethereum-mainnet-claim-terminology-fixtures.json"
        );

        let fixture: serde_json::Value =
            serde_json::from_str(FIXTURE).expect("parse Ethereum claim terminology fixture");
        assert_eq!(fixture["schema_version"], 1);
        assert_eq!(
            fixture["kind"],
            "clg.ethereum-mainnet-claim-terminology-fixtures.v1"
        );
        assert_eq!(
            fixture["authoritative_lock"],
            "docs/design/phase-50.2-safe-owner-amendment.md"
        );
        assert_eq!(
            fixture["target_profile"],
            "clg.evm-ethereum-mainnet-safe-registry.v1"
        );
        assert_eq!(fixture["chain_id"], 1);

        let cases = fixture["cases"].as_array().expect("fixture cases array");
        assert_eq!(
            cases.iter().filter(|case| case["outcome"] == "accept").count(),
            6,
            "fixture must retain all six qualified evidence-layer claims"
        );
        assert_eq!(
            cases.iter().filter(|case| case["outcome"] == "reject").count(),
            5,
            "fixture must retain bare production/mainnet/verified/proved/Safe rejections"
        );
        for (id, evidence_layer) in [
            ("qualified-source-proof", "source_proof"),
            ("qualified-release-verification", "release_verification"),
            ("qualified-local-replay", "local_replay"),
            ("qualified-target-observation", "target_observation"),
            ("qualified-production-profile", "production_profile"),
            ("qualified-safe-governance", "safe_governance"),
        ] {
            assert!(
                cases.iter().any(|case| {
                    case["id"] == id
                        && case["outcome"] == "accept"
                        && case["evidence_layer"] == evidence_layer
                }),
                "fixture is missing qualified `{id}` case"
            );
        }
        for id in [
            "bare-production-ready",
            "bare-mainnet-ready",
            "bare-verified",
            "bare-proved-safe",
            "bare-multisig-safe",
        ] {
            assert!(
                cases.iter().any(|case| case["id"] == id && case["outcome"] == "reject"),
                "fixture is missing rejected `{id}` case"
            );
        }

        for required in [
            "`clg.evm-ethereum-mainnet-safe-registry.v1`",
            "Safe Smart Account v1.4.1",
            "threshold of two",
            "Exactly three separately held, hardware-backed Ethereum signing keys",
            "`operation` | `CALL` only; `DELEGATECALL` is prohibited",
            "transaction value is exactly zero",
            "12-block",
            "two post-finality observations",
        ] {
            assert!(
                SAFE_AMENDMENT.contains(required),
                "Safe-owner amendment is missing `{required}`"
            );
        }
        assert!(
            DIRECT_LOCK.contains("Status: Superseded before target action"),
            "direct-owner profile must be retained as a superseded record"
        );
        assert!(
            normalized_document_text(SAFE_AMENDMENT).contains(
                "It authorizes no Safe deployment, Safe configuration, endpoint contact, signer use, transaction construction, registry deployment, invocation, or mainnet submission."
            ),
            "Safe-owner amendment must retain its no-target-action boundary"
        );
        assert!(ROADMAP.contains("- [x] 50.2 Publish the named-mainnet profile"));
        assert!(ROADMAP.contains("- [x] 50.2.5 Record the 2-of-3 Safe owner decision"));
    }

    #[test]
    fn pilot_precheck_clean_directory_fixture_validates_schema_and_digest_projection() {
        const REPRODUCTION: &str = include_str!(
            "../../../docs/fixtures/phase-40.2.3/pilot-precheck-clean-directory-reproduction.md"
        );
        const SCRIPT: &str = include_str!(
            "../../../docs/fixtures/phase-40.2.3/pilot-precheck-clean-directory-reproduction.ps1"
        );
        let root = repo_root();
        let projection_path = root
            .join("docs")
            .join("fixtures")
            .join("phase-40.2.3")
            .join("pilot-precheck-digest-projection.v2.json");
        let expected_bytes = std::fs::read(&projection_path).expect("read digest projection");
        let expected: PilotPrecheckDigestProjection =
            serde_json::from_slice(&expected_bytes).expect("parse digest projection");

        let report = new_full_pilot_precheck_report(root.as_path()).expect("new full report");
        let actual = pilot_precheck_digest_projection(&report);
        assert_eq!(actual, expected, "digest projection must bind current fixture inputs");
        assert_eq!(
            pretty_json_bytes(&expected).expect("serialize digest projection"),
            expected_bytes,
            "digest projection must use canonical JSON bytes"
        );
        assert!(
            REPRODUCTION.contains("fail-closed report is still retained as diagnostic evidence"),
            "clean-directory reproduction fixture must retain failed reports as diagnostic evidence"
        );
        for required in [
            "cargo run --offline --locked -p xtask -- pilot-precheck --report $Report",
            "pilot-precheck-digest-projection.v2.json",
            "clean report directory contains unexpected entries",
        ] {
            assert!(
                SCRIPT.contains(required),
                "runnable clean-directory fixture is missing `{required}`"
            );
        }

        let mut successful = report;
        for gate in &mut successful.gates {
            gate.status = "passed".to_string();
        }
        validate_pilot_precheck_report_against_root(root.as_path(), &successful)
            .expect("successful report validates against checkout");

        let clean_dir = unique_temp_dir("pilot-precheck-clean-directory");
        let report_path = clean_dir.join("pilot-precheck-full.json");
        write_pilot_precheck_report(report_path.as_path(), &successful)
            .expect("write clean-directory report");
        let persisted: PilotPrecheckReport = serde_json::from_slice(
            &std::fs::read(&report_path).expect("read clean-directory report"),
        )
        .expect("parse clean-directory report");
        validate_pilot_precheck_report_against_root(root.as_path(), &persisted)
            .expect("persisted report validates against checkout");
        assert_eq!(
            pilot_precheck_digest_projection(&persisted),
            expected,
            "persisted report must retain the locked digest projection"
        );
        assert_eq!(
            std::fs::read_dir(&clean_dir)
                .expect("read clean directory")
                .count(),
            1,
            "clean directory must contain only the report"
        );
        cleanup_temp_dir(clean_dir.as_path());
    }

    #[test]
    fn pilot_precheck_full_report_is_deterministic_and_validated() {
        let root = repo_root();
        let mut report = new_full_pilot_precheck_report(root.as_path()).expect("new report");
        for gate in &mut report.gates {
            gate.status = "passed".to_string();
        }
        validate_pilot_precheck_report_against_root(root.as_path(), &report)
            .expect("valid full report");
        assert_eq!(report.scope, "full");
        assert_eq!(report.schema_version, 2);
        assert_eq!(report.kind, "clg.pilot-precheck-report.v2");
        assert_eq!(
            report
                .toolchain
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>(),
            vec!["cargo", "clippy", "rustc", "rustfmt", "xtask"]
        );
        assert!(report.future_required_gates.is_empty());
        assert_eq!(
            report.gate_order,
            vec![
                "release-precheck",
                "contract-release-fixtures",
                "verifier-distribution-fixtures",
                "caller-authorization-fixtures",
                "reference-pilot-offline-replay",
            ]
        );

        let bytes_one = pretty_json_bytes(&report).expect("serialize report");
        let bytes_two = pretty_json_bytes(&report).expect("serialize report again");
        assert_eq!(bytes_one, bytes_two);

        let dir = unique_temp_dir("pilot-precheck-report");
        let first = dir.join("one.json");
        let second = dir.join("two.json");
        write_pilot_precheck_report(first.as_path(), &report).expect("write first report");
        write_pilot_precheck_report(first.as_path(), &report).expect("replace first report");
        write_pilot_precheck_report(second.as_path(), &report).expect("write second report");
        assert_eq!(
            std::fs::read(first).expect("read first report"),
            std::fs::read(second).expect("read second report")
        );
        cleanup_temp_dir(dir.as_path());
    }

    #[test]
    fn pilot_precheck_report_rejects_tampering_and_unsafe_evidence() {
        let root = repo_root();
        let mut report = new_full_pilot_precheck_report(root.as_path()).expect("new report");
        for gate in &mut report.gates {
            gate.status = "passed".to_string();
        }

        let mut reordered = report.clone();
        reordered.gate_order.reverse();
        assert!(validate_pilot_precheck_report(&reordered).is_err());

        let mut unsafe_path = report.clone();
        unsafe_path.gates[0].input_digests[0].id = "../secret".to_string();
        assert!(validate_pilot_precheck_report(&unsafe_path).is_err());

        let mut malformed_digest = report.clone();
        malformed_digest.gates[0].evidence_digests[0].sha256 = "BAD".to_string();
        assert!(validate_pilot_precheck_report(&malformed_digest).is_err());

        let mut non_hex_digest = report.clone();
        non_hex_digest.gates[0].evidence_digests[0].sha256 = "z".repeat(64);
        assert!(validate_pilot_precheck_report(&non_hex_digest).is_err());

        let mut duplicate_identity = report.clone();
        duplicate_identity
            .identity
            .compatibility_digests
            .push(duplicate_identity.identity.compatibility_digests[0].clone());
        assert!(validate_pilot_precheck_report(&duplicate_identity).is_err());

        let mut duplicate_tool = report.clone();
        duplicate_tool.toolchain.push(duplicate_tool.toolchain[0].clone());
        assert!(validate_pilot_precheck_report(&duplicate_tool).is_err());

        let mut missing_evidence = report.clone();
        missing_evidence.gates[0].evidence_digests.clear();
        assert!(validate_pilot_precheck_report(&missing_evidence).is_err());

        let mut substituted = report;
        substituted.gates[0].input_digests[0].sha256 = "0".repeat(64);
        assert!(validate_pilot_precheck_report(&substituted).is_ok());
        assert!(validate_pilot_precheck_report_against_root(root.as_path(), &substituted).is_err());
    }

    #[test]
    fn pilot_precheck_schema_rejects_unknown_fields_and_substituted_identity() {
        let root = repo_root();
        let mut report = new_full_pilot_precheck_report(root.as_path()).expect("new report");
        for gate in &mut report.gates {
            gate.status = "passed".to_string();
        }

        let mut encoded = serde_json::to_value(&report).expect("serialize report");
        encoded["hostname"] = serde_json::Value::String("forbidden".to_string());
        assert!(serde_json::from_value::<PilotPrecheckReport>(encoded).is_err());

        report.identity.repository.commit = "0".repeat(40);
        assert!(validate_pilot_precheck_report(&report).is_ok());
        assert!(validate_pilot_precheck_report_against_root(root.as_path(), &report).is_err());
    }

    #[test]
    fn pilot_precheck_child_cargo_commands_are_offline_and_locked() {
        let root = repo_root();
        let command = pilot_cargo_command(root.as_path(), &["test", "--workspace"]);
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert_eq!(args, vec!["--offline", "--locked", "test", "--workspace"]);
        assert!(command.get_envs().any(|(key, value)| {
            key == "CARGO_NET_OFFLINE" && value.is_some_and(|value| value == "true")
        }));
        assert!(command.get_envs().any(|(key, value)| {
            key == "CLG_PILOT_PRECHECK" && value.is_some_and(|value| value == "1")
        }));
    }

    #[test]
    fn pilot_precheck_requires_a_clean_committed_checkout() {
        validate_clean_pilot_checkout_status("").expect("clean checkout is accepted");
        let error = validate_clean_pilot_checkout_status(" M xtask/src/main.rs\n")
            .expect_err("dirty checkout is rejected");
        assert!(error.contains("clean committed checkout"));
    }

    #[test]
    fn pilot_precheck_rejects_an_unlocked_toolchain_version() {
        let root = repo_root();
        let mut toolchain = new_pilot_precheck_toolchain(root.as_path()).expect("collect toolchain");
        validate_pilot_precheck_environment(&toolchain).expect("locked toolchain is accepted");
        toolchain
            .iter_mut()
            .find(|tool| tool.name == "cargo")
            .expect("cargo tool identity")
            .version = "cargo 0.0.0".to_string();
        assert!(validate_pilot_precheck_environment(&toolchain).is_err());
        assert!(new_pilot_precheck_identity(root.as_path()).is_ok());
    }

    #[test]
    fn pilot_precheck_failure_shape_is_fail_closed() {
        let root = repo_root();
        let mut report = new_full_pilot_precheck_report(root.as_path()).expect("new report");
        report.overall_status = "failed".to_string();
        report.gates[0].status = "failed".to_string();
        report.gates[0].failure_code = Some("command_failed".to_string());
        report.gates[1].status = "not_run".to_string();
        validate_pilot_precheck_report_against_root(root.as_path(), &report)
            .expect("fail-fast report is valid evidence");
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

    #[test]
    fn shared_std_distribution_gate_accepts_completed_markers_and_required_evidence_hooks() {
        let roadmap = r#"
- [x] 28.0.1
- [x] 28.0.2
- [x] 28.1.0
- [x] 28.1.1
- [x] 28.1.2
- [x] 28.2.0
- [x] 28.2.1
- [x] 28.2.2
- [x] 28.3.0
- [x] 28.3.1
- [x] 28.3.2
- [x] 28.6.4
- [x] 28.6.5
- [x] 28.6.6
"#;
        let release = r#"
fn collect_release_shared_std_package_evidence() {}
fn verify_bundle_strict_import_map_shared_std_parity() {}
let _stage = timings.start(logger, "verify_bundle_shared_std");
let marker = "shared_std";
"#;
        let strict_artifact = r#"let marker = "shared_std";"#;
        let release_tests = r#"
fn release_command_preserves_non_empty_shared_std_evidence_for_shared_manifest() {}
fn shared_std_pkg_lock_update_and_release_support_upgrade_rotation_and_rollback() {}
fn verify_bundle_fails_closed_when_strict_import_map_shared_std_evidence_is_tampered() {}
fn verify_bundle_fails_closed_when_bundle_manifest_shared_std_evidence_is_tampered() {}
fn verify_bundle_fails_closed_when_provenance_shared_std_evidence_is_tampered() {}
"#;
        let runtime_loader_smoke_tests = r#"
fn shared_std_runtime_loader_rejects_unsupported_verified_abi_minor_range_with_r015() {}
fn shared_std_runtime_loader_tamper_missing_artifact_reports_r012() {}
fn shared_std_runtime_loader_tamper_untrusted_signer_reports_r014() {}
"#;

        let blockers = evaluate_shared_std_distribution_drift_audit(
            roadmap,
            release,
            strict_artifact,
            release_tests,
            runtime_loader_smoke_tests,
        );
        assert!(blockers.is_empty(), "expected no blockers, got {blockers:?}");
    }

    #[test]
    fn shared_std_distribution_gate_reports_missing_roadmap_and_test_coverage() {
        let blockers =
            evaluate_shared_std_distribution_drift_audit("", "", "", "", "");
        assert!(
            blockers.iter().any(|line| line.contains("28.3.2")),
            "expected roadmap blocker, got {blockers:?}"
        );
        assert!(
            blockers
                .iter()
                .any(|line| line.contains(
                    "verify_bundle_fails_closed_when_provenance_shared_std_evidence_is_tampered"
                )),
            "expected shared std test blocker, got {blockers:?}"
        );
        assert!(
            blockers
                .iter()
                .any(|line| line.contains(
                    "shared_std_runtime_loader_rejects_unsupported_verified_abi_minor_range_with_r015"
                )),
            "expected runtime shared std smoke blocker, got {blockers:?}"
        );
    }
}
