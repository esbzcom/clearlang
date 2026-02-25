use super::validate_attestation_payload_value;
use serde_json::json;

fn sample_attestation_payload() -> serde_json::Value {
    json!({
        "version": "1",
        "chain_id": 1,
        "schema_version": 1,
        "registry": "0x0000000000000000000000000000000000000001",
        "signature": {
            "key_id": "example-key",
            "scope": "both",
            "signature_format": "ed25519",
            "signature": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "payload": {
                "module_hash": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "proofs_hash": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                "toolchain": "clg/0.1.0",
                "timestamp": "2026-01-31T00:00:00Z",
                "scope": "both"
            }
        }
    })
}

#[test]
fn attestation_payload_accepts_valid_shape() {
    let payload = sample_attestation_payload();
    validate_attestation_payload_value(&payload).expect("expected valid payload");
}

#[test]
fn attestation_payload_rejects_missing_schema_version() {
    let mut payload = sample_attestation_payload();
    payload
        .as_object_mut()
        .expect("object")
        .remove("schema_version");

    let err = validate_attestation_payload_value(&payload).expect_err("missing schema_version");
    assert!(err.to_string().contains("schema_version"));
}

#[test]
fn attestation_payload_rejects_invalid_registry() {
    let mut payload = sample_attestation_payload();
    payload["registry"] = json!("0x1234");

    let err = validate_attestation_payload_value(&payload).expect_err("invalid registry");
    assert!(err.to_string().contains("registry"));
}

#[test]
fn attestation_payload_rejects_signature_scope_mismatch() {
    let mut payload = sample_attestation_payload();
    payload["signature"]["scope"] = json!("module");
    payload["signature"]["payload"]["scope"] = json!("proofs");

    let err = validate_attestation_payload_value(&payload).expect_err("scope mismatch");
    assert!(err.to_string().contains("scope"));
}

#[test]
fn attestation_payload_rejects_bad_hex_lengths() {
    let mut payload = sample_attestation_payload();
    payload["signature"]["signature"] = json!("abcd");
    let err = validate_attestation_payload_value(&payload).expect_err("bad signature length");
    assert!(err.to_string().contains("signature"));

    payload = sample_attestation_payload();
    payload["signature"]["payload"]["module_hash"] = json!("abcd");
    let err = validate_attestation_payload_value(&payload).expect_err("bad module hash length");
    assert!(err.to_string().contains("module_hash"));
}
