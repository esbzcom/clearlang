use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};

use super::SignatureFile;

#[allow(dead_code)]
pub fn validate_attestation_payload_file(path: &Path) -> Result<()> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing attestation payload {}", path.display()))?;
    validate_attestation_payload_value(&value)
}

#[allow(dead_code)]
pub fn validate_attestation_payload_value(value: &serde_json::Value) -> Result<()> {
    let root = value
        .as_object()
        .ok_or_else(|| anyhow!("attestation payload must be a JSON object"))?;

    let version = required_str(root, "version")?;
    if version != "1" {
        return Err(anyhow!("attestation payload version must be \"1\""));
    }

    let chain_id = root
        .get("chain_id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow!("attestation payload missing integer chain_id"))?;
    if chain_id == 0 {
        return Err(anyhow!("attestation payload chain_id must be > 0"));
    }

    let schema_version = root
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow!("attestation payload missing integer schema_version"))?;
    if schema_version == 0 || schema_version > u32::MAX as u64 {
        return Err(anyhow!(
            "attestation payload schema_version must be within 1..=4294967295"
        ));
    }

    let registry = required_str(root, "registry")?;
    parse_evm_address(registry)?;

    let signature_value = root
        .get("signature")
        .ok_or_else(|| anyhow!("attestation payload missing signature object"))?
        .clone();
    let signature_file: SignatureFile =
        serde_json::from_value(signature_value).context("invalid signature object")?;

    if signature_file.signature_format.to_lowercase() != "ed25519" {
        return Err(anyhow!(
            "attestation payload signature_format must be ed25519"
        ));
    }
    decode_fixed_hex("attestation signature", &signature_file.signature, 64)?;

    let sig_payload = signature_file
        .payload
        .as_object()
        .ok_or_else(|| anyhow!("attestation signature payload must be a JSON object"))?;

    decode_fixed_hex(
        "signature payload module_hash",
        required_str(sig_payload, "module_hash")?,
        32,
    )?;
    decode_fixed_hex(
        "signature payload proofs_hash",
        required_str(sig_payload, "proofs_hash")?,
        32,
    )?;

    let toolchain = required_str(sig_payload, "toolchain")?;
    if toolchain.trim().is_empty() {
        return Err(anyhow!("signature payload toolchain must be non-empty"));
    }

    let timestamp = required_str(sig_payload, "timestamp")?;
    if timestamp.trim().is_empty() {
        return Err(anyhow!("signature payload timestamp must be non-empty"));
    }

    let payload_scope = required_str(sig_payload, "scope")?;
    if payload_scope != signature_file.scope.as_str() {
        return Err(anyhow!(
            "signature payload scope must match signature scope"
        ));
    }

    Ok(())
}

fn required_str<'a>(
    map: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<&'a str> {
    map.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing string field: {key}"))
}

fn parse_evm_address(addr: &str) -> Result<()> {
    if !addr.starts_with("0x") {
        return Err(anyhow!("registry must start with 0x"));
    }
    decode_fixed_hex("registry address", &addr[2..], 20)?;
    Ok(())
}

fn decode_fixed_hex(label: &str, value: &str, byte_len: usize) -> Result<()> {
    if value.len() != byte_len * 2 {
        return Err(anyhow!("{label} must be {} hex chars", byte_len * 2));
    }
    let decoded = hex::decode(value).with_context(|| format!("invalid {label} hex"))?;
    if decoded.len() != byte_len {
        return Err(anyhow!("{label} must decode to {byte_len} bytes"));
    }
    Ok(())
}
