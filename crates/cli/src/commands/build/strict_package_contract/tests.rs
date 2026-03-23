#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn valid_metadata_json() -> String {
        r#"{
  "schema_version": 0,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0"
    }
  ]
}"#
        .to_string()
    }

    fn valid_metadata_json_v1() -> String {
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0",
      "signature": {
        "format": "ed25519",
        "key_id": "k1",
        "signed_at": "2026-01-15T00:00:00Z",
        "signature": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "trust": {
        "trusted_anchor_ids": ["k1"]
      },
      "dependencies": [
        { "name": "std::host", "requirement": "^1.0.0" }
      ]
    }
  ]
}"#
        .to_string()
    }

    fn valid_abi_json() -> String {
        r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:std::core:1.0.0",
      "package": "std::core",
      "version": "1.0.0",
      "imports": [
        {
          "symbol": "std::core::math::add",
          "effect": "pure",
          "params": ["Int", "Int"],
          "ret": "Int",
          "capability": null
        }
      ]
    }
  ]
}"#
        .to_string()
    }

    #[test]
    fn valid_inputs_parse_and_validate() {
        let parsed = parse_package_metadata_abi_v0(
            &valid_metadata_json(),
            &valid_abi_json(),
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect("valid strict package metadata/abi");
        assert_eq!(parsed.packages.len(), 1);
        assert_eq!(parsed.contracts.len(), 1);
        assert_eq!(parsed.packages[0].name, "std::core");
        assert_eq!(parsed.contracts[0].abi_id, "abi:std::core:1.0.0");
    }

    #[test]
    fn schema_v1_metadata_with_signature_and_trust_parses_and_validates() {
        let parsed = parse_package_metadata_abi_v0(
            &valid_metadata_json_v1(),
            &valid_abi_json(),
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect("valid strict package metadata/abi v1");
        assert_eq!(parsed.packages.len(), 1);
        let pkg = &parsed.packages[0];
        let sig = pkg.signature.as_ref().expect("signature metadata");
        assert_eq!(sig.key_id, "k1");
        assert_eq!(pkg.trusted_anchor_ids, vec!["k1"]);
        assert_eq!(pkg.dependencies.len(), 1);
        assert_eq!(pkg.dependencies[0].name, "std::host");
        assert_eq!(pkg.dependencies[0].requirement, "^1.0.0");
    }

    #[test]
    fn missing_metadata_file_reports_c104() {
        let tmp = tempdir().expect("tempdir");
        fs::write(tmp.path().join(STRICT_PACKAGE_ABI_FILE), valid_abi_json())
            .expect("write abi file");
        let err =
            load_required_package_metadata_abi_v0(tmp.path()).expect_err("expected missing file");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains(STRICT_PACKAGE_METADATA_FILE));
    }

    #[test]
    fn malformed_metadata_schema_reports_c104() {
        let bad_metadata = r#"{
  "schema_version": 0,
  "packages": [],
  "extra": true
}"#;
        let err = parse_package_metadata_abi_v0(
            bad_metadata,
            &valid_abi_json(),
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected malformed metadata schema error");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("does not match schema v0"));
    }

    #[test]
    fn metadata_v1_requires_trusted_anchors_when_signature_present() {
        let metadata = r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0",
      "signature": {
        "format": "ed25519",
        "key_id": "k1",
        "signed_at": "2026-01-15T00:00:00Z",
        "signature": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "trust": { "trusted_anchor_ids": [] }
    }
  ]
}"#;
        let err = parse_package_metadata_abi_v0(
            metadata,
            &valid_abi_json(),
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected trusted-anchor validation error");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("at least one trusted anchor id"));
    }

    #[test]
    fn abi_id_missing_from_contracts_reports_c105() {
        let abi = r#"{
  "schema_version": 0,
  "contracts": []
}"#;
        let err = parse_package_metadata_abi_v0(
            &valid_metadata_json(),
            abi,
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected ABI mismatch");
        assert_eq!(err.code(), "C105");
        assert!(err.message().contains("missing from ABI contracts"));
    }

    #[test]
    fn package_version_mismatch_reports_c105() {
        let abi = r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:std::core:1.0.0",
      "package": "std::core",
      "version": "1.0.1",
      "imports": []
    }
  ]
}"#;
        let err = parse_package_metadata_abi_v0(
            &valid_metadata_json(),
            abi,
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected package/version mismatch");
        assert_eq!(err.code(), "C105");
        assert!(err.message().contains("inconsistent package/version"));
    }

    #[test]
    fn duplicate_symbols_report_c104() {
        let abi = r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:std::core:1.0.0",
      "package": "std::core",
      "version": "1.0.0",
      "imports": [
        {
          "symbol": "s",
          "effect": "pure",
          "params": ["Int"],
          "ret": "Int",
          "capability": null
        },
        {
          "symbol": "s",
          "effect": "pure",
          "params": ["Int"],
          "ret": "Int",
          "capability": null
        }
      ]
    }
  ]
}"#;
        let err = parse_package_metadata_abi_v0(
            &valid_metadata_json(),
            abi,
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected duplicate symbol error");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("duplicate symbol"));
    }

    #[test]
    fn duplicate_metadata_abi_id_reports_c104() {
        let metadata = r#"{
  "schema_version": 0,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:shared:1"
    },
    {
      "name": "std::math",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/std-math-1.0.0.wasm" },
      "abi_id": "abi:shared:1"
    }
  ]
}"#;
        let abi = r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:shared:1",
      "package": "std::core",
      "version": "1.0.0",
      "imports": []
    }
  ]
}"#;
        let err = parse_package_metadata_abi_v0(
            metadata,
            abi,
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected duplicate metadata abi_id");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("duplicate abi_id"));
    }

    #[test]
    fn metadata_v1_rejects_duplicate_dependency_names() {
        let metadata = r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0",
      "signature": {
        "format": "ed25519",
        "key_id": "k1",
        "signed_at": "2026-01-15T00:00:00Z",
        "signature": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "trust": { "trusted_anchor_ids": ["k1"] },
      "dependencies": [
        { "name": "std::host", "requirement": "^1.0.0" },
        { "name": "std::host", "requirement": "^1.0.0" }
      ]
    }
  ]
}"#;
        let err = parse_package_metadata_abi_v0(
            metadata,
            &valid_abi_json(),
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected duplicate dependency validation failure");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("duplicate dependency name"));
    }

    #[test]
    fn metadata_rejects_artifact_path_parent_traversal() {
        let metadata = r#"{
  "schema_version": 0,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "../store/std-core.wasm" },
      "abi_id": "abi:std::core:1.0.0"
    }
  ]
}"#;
        let err = parse_package_metadata_abi_v0(
            metadata,
            &valid_abi_json(),
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected traversal artifact path validation failure");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("parent-directory traversal"));
    }
}
