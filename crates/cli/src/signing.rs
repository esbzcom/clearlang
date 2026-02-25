use std::convert::TryFrom;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use wasmparser::{Parser, Payload};

use crate::commands::helpers::{canonical_json_string, sha256_hex};
use crate::proofs::{
    decode_proof_section, module_bytes_with_zeroed_hash, proofs_hash_from_section, ProofPackage,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
#[clap(rename_all = "lowercase")]
pub enum SignScope {
    Module,
    Proofs,
    Both,
}

impl SignScope {
    pub fn includes_module(self) -> bool {
        matches!(self, SignScope::Module | SignScope::Both)
    }

    pub fn includes_proofs(self) -> bool {
        matches!(self, SignScope::Proofs | SignScope::Both)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SignScope::Module => "module",
            SignScope::Proofs => "proofs",
            SignScope::Both => "both",
        }
    }
}

#[derive(Debug, Deserialize)]
struct SigningKeyFile {
    scheme: String,
    private_key: String,
    #[serde(rename = "public_key")]
    _public_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VerifyKeyFile {
    scheme: String,
    public_key: String,
}

#[derive(Debug, Deserialize)]
struct TrustAnchorPolicyFile {
    schema_version: u32,
    trust_anchors: TrustAnchorVersions,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct TrustAnchorVersions {
    lean_checker: String,
    coq_checker: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureFile {
    pub key_id: String,
    pub scope: SignScope,
    pub signature_format: String,
    pub signature: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssuranceManifestSignature {
    pub key_id: String,
    pub signature_format: String,
    pub payload_hash: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssuranceManifestFile {
    pub schema_version: u32,
    pub payload: serde_json::Value,
    pub signature: AssuranceManifestSignature,
}

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

#[allow(dead_code)]
fn required_str<'a>(
    map: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<&'a str> {
    map.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing string field: {key}"))
}

#[allow(dead_code)]
fn parse_evm_address(addr: &str) -> Result<()> {
    if !addr.starts_with("0x") {
        return Err(anyhow!("registry must start with 0x"));
    }
    decode_fixed_hex("registry address", &addr[2..], 20)?;
    Ok(())
}

#[allow(dead_code)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyErrorCode {
    SignatureFailure,
    ProofMissing,
    HashMismatch,
    TrustAnchorFailure,
    PolicyFailure,
}

impl VerifyErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            VerifyErrorCode::SignatureFailure => "V001",
            VerifyErrorCode::ProofMissing => "V002",
            VerifyErrorCode::HashMismatch => "V003",
            VerifyErrorCode::TrustAnchorFailure => "V004",
            VerifyErrorCode::PolicyFailure => "V005",
        }
    }
}

#[derive(Debug)]
pub struct VerifyError {
    code: VerifyErrorCode,
    message: String,
}

impl VerifyError {
    pub fn new(code: VerifyErrorCode, message: impl Into<String>) -> Self {
        VerifyError {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        self.code.as_str()
    }
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for VerifyError {}

pub fn sign_bundle(
    package: &ProofPackage,
    module_hash_hex: &str,
    scope: SignScope,
    key_path: &Path,
    key_id: &str,
    sig_out: &Path,
    timestamp: &str,
    lean_checker_version: Option<&str>,
    coq_checker_version: Option<&str>,
) -> Result<()> {
    let signing = load_signing_key(key_path)?;
    let mut payload_value = package.build_signing_payload(module_hash_hex, scope, timestamp);
    if lean_checker_version.is_some() || coq_checker_version.is_some() {
        let Some(lean_checker) = lean_checker_version else {
            return Err(anyhow!(
                "missing --lean-checker-version for trust-anchor payload"
            ));
        };
        let Some(coq_checker) = coq_checker_version else {
            return Err(anyhow!(
                "missing --coq-checker-version for trust-anchor payload"
            ));
        };
        if lean_checker.trim().is_empty() || coq_checker.trim().is_empty() {
            return Err(anyhow!("trust-anchor checker versions must be non-empty"));
        }
        let payload = payload_value
            .as_object_mut()
            .ok_or_else(|| anyhow!("signature payload must be a JSON object"))?;
        payload.insert(
            "trust_anchors".to_string(),
            serde_json::json!({
                "lean_checker": lean_checker,
                "coq_checker": coq_checker,
            }),
        );
    }
    let (sig_hex, _) = sign_payload(&signing, &payload_value);
    let sig_file = SignatureFile {
        key_id: key_id.to_string(),
        scope,
        signature_format: "ed25519".to_string(),
        signature: sig_hex,
        payload: payload_value,
    };
    let output = serde_json::to_vec_pretty(&sig_file)?;
    if let Some(dir) = sig_out.parent() {
        if !dir.as_os_str().is_empty() {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
    }
    fs::write(sig_out, output).with_context(|| format!("writing {}", sig_out.display()))?;
    Ok(())
}

pub fn sign_assurance_manifest(
    payload: serde_json::Value,
    key_path: &Path,
    key_id: &str,
    out: &Path,
) -> Result<()> {
    let signing = load_signing_key(key_path)?;
    let (signature, payload_hash) = sign_payload(&signing, &payload);
    let manifest = AssuranceManifestFile {
        schema_version: 1,
        payload,
        signature: AssuranceManifestSignature {
            key_id: key_id.to_string(),
            signature_format: "ed25519".to_string(),
            payload_hash,
            signature,
        },
    };
    if let Some(dir) = out.parent() {
        if !dir.as_os_str().is_empty() {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
    }
    let output = serde_json::to_vec_pretty(&manifest)?;
    fs::write(out, output).with_context(|| format!("writing {}", out.display()))?;
    Ok(())
}

fn load_signing_key(key_path: &Path) -> Result<SigningKey> {
    let key_data = fs::read(key_path).with_context(|| format!("reading {}", key_path.display()))?;
    let key_file: SigningKeyFile =
        serde_json::from_slice(&key_data).context("parsing signing key")?;
    if key_file.scheme.to_lowercase() != "ed25519" {
        return Err(anyhow!("unsupported signing scheme: {}", key_file.scheme));
    }
    let key_bytes = hex::decode(&key_file.private_key).context("decoding private key hex")?;
    Ok(SigningKey::from_bytes(&key_bytes.try_into().map_err(
        |_| anyhow!("ed25519 private key must be 32 bytes"),
    )?))
}

fn sign_payload(signing: &SigningKey, payload: &serde_json::Value) -> (String, String) {
    let canonical_payload = canonical_json_string(payload);
    let signature = signing.sign(canonical_payload.as_bytes());
    let signature_hex = hex::encode(signature.to_bytes());
    let payload_hash = sha256_hex(canonical_payload.as_bytes());
    (signature_hex, payload_hash)
}

pub fn verify_signature(
    module_path: &Path,
    sig_path: &Path,
    pubkey_path: &Path,
) -> std::result::Result<(), VerifyError> {
    let module_bytes = fs::read(module_path).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!("reading {}: {err}", module_path.display()),
        )
    })?;
    let signature_bytes = fs::read(sig_path).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!("reading {}: {err}", sig_path.display()),
        )
    })?;
    let sig_file: SignatureFile = serde_json::from_slice(&signature_bytes).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!("parsing signature file: {err}"),
        )
    })?;
    if sig_file.signature_format.to_lowercase() != "ed25519" {
        return Err(VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!(
                "unsupported signature format: {}",
                sig_file.signature_format
            ),
        ));
    }
    let pub_bytes = fs::read(pubkey_path).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!("reading {}: {err}", pubkey_path.display()),
        )
    })?;
    let verify_file: VerifyKeyFile = serde_json::from_slice(&pub_bytes).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!("parsing public key file: {err}"),
        )
    })?;
    if verify_file.scheme.to_lowercase() != "ed25519" {
        return Err(VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!("unsupported public key scheme: {}", verify_file.scheme),
        ));
    }
    let pub_key_bytes = hex::decode(&verify_file.public_key).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!("decoding public key hex: {err}"),
        )
    })?;
    let verifying = VerifyingKey::from_bytes(&pub_key_bytes.try_into().map_err(|_| {
        VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            "ed25519 public key must be 32 bytes",
        )
    })?)
    .map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!("verifying key invalid: {err}"),
        )
    })?;

    let payload_canonical = canonical_json_string(&sig_file.payload);
    let sig_raw = hex::decode(&sig_file.signature).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!("decoding signature hex: {err}"),
        )
    })?;
    let signature = Signature::try_from(sig_raw.as_slice()).map_err(|_| {
        VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            "ed25519 signature must be 64 bytes",
        )
    })?;
    verifying
        .verify_strict(payload_canonical.as_bytes(), &signature)
        .map_err(|_| {
            VerifyError::new(
                VerifyErrorCode::SignatureFailure,
                "signature verification failed",
            )
        })?;

    let zeroed_module = module_bytes_with_zeroed_hash(&module_bytes).map_err(|err| {
        let message = err.to_string();
        if message.contains("clearlang.proof section not found") {
            VerifyError::new(VerifyErrorCode::ProofMissing, message)
        } else {
            VerifyError::new(
                VerifyErrorCode::SignatureFailure,
                format!("canonicalizing module hash: {message}"),
            )
        }
    })?;
    let computed_module_hash = sha256_hex(&zeroed_module);

    let payload_module_hash = sig_file
        .payload
        .get("module_hash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            VerifyError::new(
                VerifyErrorCode::SignatureFailure,
                "payload missing module_hash",
            )
        })?;

    if sig_file.scope.includes_module() && payload_module_hash != computed_module_hash {
        return Err(VerifyError::new(
            VerifyErrorCode::HashMismatch,
            "payload module hash mismatch",
        ));
    }

    let proof_bytes = find_proof_section(&module_bytes)?;
    let proof_section = decode_proof_section(&proof_bytes).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!("decoding proof section: {err}"),
        )
    })?;
    if proof_section.module_hash.len() != 32 {
        return Err(VerifyError::new(
            VerifyErrorCode::HashMismatch,
            "proof section module hash must be 32 bytes",
        ));
    }
    let section_module_hash = hex::encode(&proof_section.module_hash);
    if section_module_hash != computed_module_hash {
        return Err(VerifyError::new(
            VerifyErrorCode::HashMismatch,
            "proof section module hash mismatch",
        ));
    }

    let section_proofs_hash = hex::encode(&proof_section.proofs_hash);
    let recomputed_proofs_hash =
        hex::encode(proofs_hash_from_section(&proof_section).map_err(|err| {
            VerifyError::new(
                VerifyErrorCode::SignatureFailure,
                format!("computing proofs hash: {err}"),
            )
        })?);
    if section_proofs_hash != recomputed_proofs_hash {
        return Err(VerifyError::new(
            VerifyErrorCode::HashMismatch,
            "proof section proofs hash mismatch",
        ));
    }

    let payload_proofs_hash = sig_file
        .payload
        .get("proofs_hash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            VerifyError::new(
                VerifyErrorCode::SignatureFailure,
                "payload missing proofs_hash",
            )
        })?;

    if sig_file.scope.includes_proofs() && payload_proofs_hash != section_proofs_hash {
        return Err(VerifyError::new(
            VerifyErrorCode::HashMismatch,
            "payload proofs hash mismatch",
        ));
    }

    Ok(())
}

pub fn verify_signature_with_trust_policy(
    module_path: &Path,
    sig_path: &Path,
    pubkey_path: &Path,
    trust_policy_path: &Path,
) -> std::result::Result<(), VerifyError> {
    verify_signature(module_path, sig_path, pubkey_path)?;
    let policy = read_trust_anchor_policy(trust_policy_path)?;
    let payload = read_signature_payload(sig_path)?;
    let payload_versions = extract_payload_trust_anchors(&payload)?;
    if payload_versions != policy {
        return Err(VerifyError::new(
            VerifyErrorCode::TrustAnchorFailure,
            format!(
                "trust-anchor checker version mismatch: expected lean={} coq={}, got lean={} coq={}",
                policy.lean_checker,
                policy.coq_checker,
                payload_versions.lean_checker,
                payload_versions.coq_checker
            ),
        ));
    }
    Ok(())
}

pub fn verify_assurance_manifest(
    manifest_path: &Path,
    pubkey_path: &Path,
) -> std::result::Result<AssuranceManifestFile, VerifyError> {
    let bytes = fs::read(manifest_path).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!(
                "reading assurance manifest {}: {err}",
                manifest_path.display()
            ),
        )
    })?;
    let manifest: AssuranceManifestFile = serde_json::from_slice(&bytes).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!(
                "parsing assurance manifest {}: {err}",
                manifest_path.display()
            ),
        )
    })?;
    if manifest.schema_version != 1 {
        return Err(VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!(
                "unsupported assurance manifest schema_version {} (expected 1)",
                manifest.schema_version
            ),
        ));
    }
    if manifest.signature.signature_format.to_lowercase() != "ed25519" {
        return Err(VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            format!(
                "unsupported assurance manifest signature format: {}",
                manifest.signature.signature_format
            ),
        ));
    }
    let payload_canonical = canonical_json_string(&manifest.payload);
    let computed_payload_hash = sha256_hex(payload_canonical.as_bytes());
    if computed_payload_hash != manifest.signature.payload_hash {
        return Err(VerifyError::new(
            VerifyErrorCode::SignatureFailure,
            "assurance manifest payload_hash mismatch",
        ));
    }

    let verifying = read_verifying_key(pubkey_path, VerifyErrorCode::SignatureFailure)?;
    let signature = parse_signature_hex(
        &manifest.signature.signature,
        VerifyErrorCode::SignatureFailure,
        "assurance manifest signature",
    )?;
    verifying
        .verify_strict(payload_canonical.as_bytes(), &signature)
        .map_err(|_| {
            VerifyError::new(
                VerifyErrorCode::SignatureFailure,
                "assurance manifest signature verification failed",
            )
        })?;

    Ok(manifest)
}

fn read_trust_anchor_policy(path: &Path) -> std::result::Result<TrustAnchorVersions, VerifyError> {
    let bytes = fs::read(path).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::TrustAnchorFailure,
            format!("reading trust policy {}: {err}", path.display()),
        )
    })?;
    let policy: TrustAnchorPolicyFile = serde_json::from_slice(&bytes).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::TrustAnchorFailure,
            format!("parsing trust policy {}: {err}", path.display()),
        )
    })?;
    if policy.schema_version != 1 {
        return Err(VerifyError::new(
            VerifyErrorCode::TrustAnchorFailure,
            format!(
                "unsupported trust policy schema_version {} (expected 1)",
                policy.schema_version
            ),
        ));
    }
    if policy.trust_anchors.lean_checker.trim().is_empty()
        || policy.trust_anchors.coq_checker.trim().is_empty()
    {
        return Err(VerifyError::new(
            VerifyErrorCode::TrustAnchorFailure,
            "trust policy checker versions must be non-empty",
        ));
    }
    Ok(policy.trust_anchors)
}

fn read_signature_payload(path: &Path) -> std::result::Result<serde_json::Value, VerifyError> {
    let bytes = fs::read(path).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::TrustAnchorFailure,
            format!("reading signature file {}: {err}", path.display()),
        )
    })?;
    let sig_file: SignatureFile = serde_json::from_slice(&bytes).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::TrustAnchorFailure,
            format!("parsing signature file {}: {err}", path.display()),
        )
    })?;
    Ok(sig_file.payload)
}

fn extract_payload_trust_anchors(
    payload: &serde_json::Value,
) -> std::result::Result<TrustAnchorVersions, VerifyError> {
    let trust = payload
        .get("trust_anchors")
        .ok_or_else(|| {
            VerifyError::new(
                VerifyErrorCode::TrustAnchorFailure,
                "signature payload missing trust_anchors",
            )
        })?
        .clone();
    let trust: TrustAnchorVersions = serde_json::from_value(trust).map_err(|err| {
        VerifyError::new(
            VerifyErrorCode::TrustAnchorFailure,
            format!("invalid signature payload trust_anchors: {err}"),
        )
    })?;
    if trust.lean_checker.trim().is_empty() || trust.coq_checker.trim().is_empty() {
        return Err(VerifyError::new(
            VerifyErrorCode::TrustAnchorFailure,
            "signature payload trust_anchors checker versions must be non-empty",
        ));
    }
    Ok(trust)
}

fn find_proof_section(bytes: &[u8]) -> std::result::Result<Vec<u8>, VerifyError> {
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|err| {
            VerifyError::new(
                VerifyErrorCode::SignatureFailure,
                format!("parsing module: {err}"),
            )
        })?;
        if let Payload::CustomSection(section) = payload {
            if section.name() == "clearlang.proof" {
                return Ok(section.data().to_vec());
            }
        }
    }
    Err(VerifyError::new(
        VerifyErrorCode::ProofMissing,
        "clearlang.proof section not found",
    ))
}

fn read_verifying_key(
    pubkey_path: &Path,
    code: VerifyErrorCode,
) -> std::result::Result<VerifyingKey, VerifyError> {
    let pub_bytes = fs::read(pubkey_path).map_err(|err| {
        VerifyError::new(code, format!("reading {}: {err}", pubkey_path.display()))
    })?;
    let verify_file: VerifyKeyFile = serde_json::from_slice(&pub_bytes)
        .map_err(|err| VerifyError::new(code, format!("parsing public key file: {err}")))?;
    if verify_file.scheme.to_lowercase() != "ed25519" {
        return Err(VerifyError::new(
            code,
            format!("unsupported public key scheme: {}", verify_file.scheme),
        ));
    }
    let pub_key_bytes = hex::decode(&verify_file.public_key)
        .map_err(|err| VerifyError::new(code, format!("decoding public key hex: {err}")))?;
    VerifyingKey::from_bytes(
        &pub_key_bytes
            .try_into()
            .map_err(|_| VerifyError::new(code, "ed25519 public key must be 32 bytes"))?,
    )
    .map_err(|err| VerifyError::new(code, format!("verifying key invalid: {err}")))
}

fn parse_signature_hex(
    signature_hex: &str,
    code: VerifyErrorCode,
    label: &str,
) -> std::result::Result<Signature, VerifyError> {
    let sig_raw = hex::decode(signature_hex)
        .map_err(|err| VerifyError::new(code, format!("decoding {label} hex: {err}")))?;
    Signature::try_from(sig_raw.as_slice())
        .map_err(|_| VerifyError::new(code, format!("{label} must be 64 bytes")))
}

#[cfg(test)]
mod tests {
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
}
