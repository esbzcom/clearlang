#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use tempfile::tempdir;

    fn package_contract() -> StrictPackageContractV0 {
        StrictPackageContractV0 {
            packages: vec![
                super::super::strict_package_contract::StrictPackageMetadataEntry {
                    name: "std::core".to_string(),
                    version: "1.0.0".to_string(),
                    digest:
                        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    artifact_format: "wasm".to_string(),
                    artifact_path: "store/std-core-1.0.0.wasm".to_string(),
                    abi_id: "abi:std::core:1.0.0".to_string(),
                    dependencies: Vec::new(),
                    signature: None,
                    trusted_anchor_ids: Vec::new(),
                },
            ],
            contracts: Vec::new(),
        }
    }

    fn package_contract_with_metadata_signature(
        key_id: &str,
        signed_at: &str,
        signature_hex: &str,
        trusted_anchor_ids: &[&str],
    ) -> StrictPackageContractV0 {
        StrictPackageContractV0 {
            packages: vec![
                super::super::strict_package_contract::StrictPackageMetadataEntry {
                    name: "std::core".to_string(),
                    version: "1.0.0".to_string(),
                    digest:
                        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    artifact_format: "wasm".to_string(),
                    artifact_path: "store/std-core-1.0.0.wasm".to_string(),
                    abi_id: "abi:std::core:1.0.0".to_string(),
                    dependencies: Vec::new(),
                    signature: Some(
                        super::super::strict_package_contract::StrictPackageMetadataSignature {
                            format: "ed25519".to_string(),
                            key_id: key_id.to_string(),
                            signed_at: signed_at.to_string(),
                            signature: signature_hex.to_string(),
                        },
                    ),
                    trusted_anchor_ids: trusted_anchor_ids
                        .iter()
                        .map(|value| value.to_string())
                        .collect(),
                },
            ],
            contracts: Vec::new(),
        }
    }

    fn trust_policy_for_signing_key(signing: &SigningKey) -> StrictTrustPolicyV0 {
        let public_key = format!("hex:{}", hex::encode(signing.verifying_key().to_bytes()));
        StrictTrustPolicyV0 {
            trusted_signers: vec![TrustedSignerV0 {
                key_id: "k1".to_string(),
                scheme: "ed25519".to_string(),
                public_key,
                not_before: "2026-01-01T00:00:00Z".to_string(),
                not_after: "2027-01-01T00:00:00Z".to_string(),
            }],
            revoked_key_ids: Vec::new(),
            compromised_key_ids: Vec::new(),
            lifecycle: None,
        }
    }

    fn write_signature_file(root: &Path, signature_hex: &str, key_id: &str, signed_at: &str) {
        fs::write(
            root.join(STRICT_PACKAGE_SIGNATURES_FILE),
            format!(
                r#"{{
  "schema_version": 0,
  "signatures": [
    {{
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "key_id": "{key_id}",
      "signed_at": "{signed_at}",
      "signature_format": "ed25519",
      "signature": "{signature_hex}"
    }}
  ]
}}"#
            ),
        )
        .expect("write signature file");
    }

    #[test]
    fn trust_gate_accepts_valid_signature() {
        let tmp = tempdir().expect("tempdir");
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let signed_at = "2026-06-01T00:00:00Z";
        let payload = canonical_payload(
            "std::core",
            "1.0.0",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            signed_at,
        );
        let signature_hex = hex::encode(signing.sign(payload.as_bytes()).to_bytes());
        write_signature_file(tmp.path(), &signature_hex, "k1", signed_at);

        let package_contract = package_contract();
        let trust_policy = trust_policy_for_signing_key(&signing);
        enforce_trust_gate_v0(tmp.path(), &package_contract, &trust_policy)
            .expect("valid signature should pass");
    }

    #[test]
    fn trust_gate_rejects_untrusted_signer() {
        let tmp = tempdir().expect("tempdir");
        write_signature_file(
            tmp.path(),
            &"aa".repeat(64),
            "unknown",
            "2026-06-01T00:00:00Z",
        );
        let package_contract = package_contract();
        let trust_policy = trust_policy_for_signing_key(&SigningKey::from_bytes(&[7u8; 32]));
        let err = enforce_trust_gate_v0(tmp.path(), &package_contract, &trust_policy)
            .expect_err("expected untrusted signer");
        assert_eq!(err.code(), "C103");
        assert!(err.message().contains("not trusted"));
    }

    #[test]
    fn trust_gate_rejects_signature_outside_signer_window() {
        let tmp = tempdir().expect("tempdir");
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let signed_at = "2028-01-01T00:00:00Z";
        let payload = canonical_payload(
            "std::core",
            "1.0.0",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            signed_at,
        );
        let signature_hex = hex::encode(signing.sign(payload.as_bytes()).to_bytes());
        write_signature_file(tmp.path(), &signature_hex, "k1", signed_at);
        let package_contract = package_contract();
        let trust_policy = trust_policy_for_signing_key(&signing);
        let err = enforce_trust_gate_v0(tmp.path(), &package_contract, &trust_policy)
            .expect_err("expected signer window failure");
        assert_eq!(err.code(), "C103");
        assert!(err.message().contains("not valid at signed_at"));
    }

    #[test]
    fn trust_gate_rejects_metadata_signature_mismatch() {
        let tmp = tempdir().expect("tempdir");
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let signed_at = "2026-06-01T00:00:00Z";
        let payload = canonical_payload(
            "std::core",
            "1.0.0",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            signed_at,
        );
        let signature_hex = hex::encode(signing.sign(payload.as_bytes()).to_bytes());
        write_signature_file(tmp.path(), &signature_hex, "k1", signed_at);

        let package_contract =
            package_contract_with_metadata_signature("k1", signed_at, &"aa".repeat(64), &["k1"]);
        let trust_policy = trust_policy_for_signing_key(&signing);
        let err = enforce_trust_gate_v0(tmp.path(), &package_contract, &trust_policy)
            .expect_err("expected metadata/signature mismatch");
        assert_eq!(err.code(), "C103");
        assert!(err.message().contains("metadata signature does not match"));
    }

    #[test]
    fn trust_gate_rejects_missing_metadata_anchor_in_policy() {
        let tmp = tempdir().expect("tempdir");
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let signed_at = "2026-06-01T00:00:00Z";
        let payload = canonical_payload(
            "std::core",
            "1.0.0",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            signed_at,
        );
        let signature_hex = hex::encode(signing.sign(payload.as_bytes()).to_bytes());
        write_signature_file(tmp.path(), &signature_hex, "k1", signed_at);

        let package_contract =
            package_contract_with_metadata_signature("k1", signed_at, &signature_hex, &["k2"]);
        let trust_policy = trust_policy_for_signing_key(&signing);
        let err = enforce_trust_gate_v0(tmp.path(), &package_contract, &trust_policy)
            .expect_err("expected missing anchor in trust policy");
        assert_eq!(err.code(), "C103");
        assert!(err
            .message()
            .contains("not permitted by package trusted anchors"));
    }

    #[test]
    fn trust_gate_rejects_compromised_signer() {
        let tmp = tempdir().expect("tempdir");
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let signed_at = "2026-06-01T00:00:00Z";
        let payload = canonical_payload(
            "std::core",
            "1.0.0",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            signed_at,
        );
        let signature_hex = hex::encode(signing.sign(payload.as_bytes()).to_bytes());
        write_signature_file(tmp.path(), &signature_hex, "k1", signed_at);

        let package_contract = package_contract();
        let mut trust_policy = trust_policy_for_signing_key(&signing);
        trust_policy.compromised_key_ids = vec!["k1".to_string()];
        trust_policy.revoked_key_ids = vec!["k1".to_string()];
        let err = enforce_trust_gate_v0(tmp.path(), &package_contract, &trust_policy)
            .expect_err("expected compromised signer to fail trust gate");
        assert_eq!(err.code(), "C103");
        assert!(err.message().contains("signer `k1` is revoked"));
    }
}
