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

#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureFile {
    pub key_id: String,
    pub scope: SignScope,
    pub signature_format: String,
    pub signature: String,
    pub payload: serde_json::Value,
}

pub fn sign_bundle(
    package: &ProofPackage,
    module_hash_hex: &str,
    scope: SignScope,
    key_path: &Path,
    key_id: &str,
    sig_out: &Path,
    timestamp: &str,
) -> Result<()> {
    let key_data = fs::read(key_path).with_context(|| format!("reading {}", key_path.display()))?;
    let key_file: SigningKeyFile =
        serde_json::from_slice(&key_data).context("parsing signing key")?;
    if key_file.scheme.to_lowercase() != "ed25519" {
        return Err(anyhow!("unsupported signing scheme: {}", key_file.scheme));
    }
    let key_bytes = hex::decode(&key_file.private_key).context("decoding private key hex")?;
    let signing = SigningKey::from_bytes(
        &key_bytes
            .try_into()
            .map_err(|_| anyhow!("ed25519 private key must be 32 bytes"))?,
    );
    let payload_value = package.build_signing_payload(module_hash_hex, scope, timestamp);
    let canonical_payload = canonical_json_string(&payload_value);
    let signature = signing.sign(canonical_payload.as_bytes());
    let sig_hex = hex::encode(signature.to_bytes());
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

pub fn verify_signature(module_path: &Path, sig_path: &Path, pubkey_path: &Path) -> Result<()> {
    let module_bytes =
        fs::read(module_path).with_context(|| format!("reading {}", module_path.display()))?;
    let signature_bytes =
        fs::read(sig_path).with_context(|| format!("reading {}", sig_path.display()))?;
    let sig_file: SignatureFile =
        serde_json::from_slice(&signature_bytes).context("parsing signature file")?;
    if sig_file.signature_format.to_lowercase() != "ed25519" {
        return Err(anyhow!(
            "unsupported signature format: {}",
            sig_file.signature_format
        ));
    }
    let pub_bytes =
        fs::read(pubkey_path).with_context(|| format!("reading {}", pubkey_path.display()))?;
    let verify_file: VerifyKeyFile =
        serde_json::from_slice(&pub_bytes).context("parsing public key file")?;
    if verify_file.scheme.to_lowercase() != "ed25519" {
        return Err(anyhow!(
            "unsupported public key scheme: {}",
            verify_file.scheme
        ));
    }
    let verifying = VerifyingKey::from_bytes(
        &hex::decode(&verify_file.public_key)
            .context("decoding public key hex")?
            .try_into()
            .map_err(|_| anyhow!("ed25519 public key must be 32 bytes"))?,
    )?;

    let payload_canonical = canonical_json_string(&sig_file.payload);
    let sig_raw = hex::decode(&sig_file.signature).context("decoding signature hex")?;
    let signature = Signature::try_from(sig_raw.as_slice())
        .map_err(|_| anyhow!("ed25519 signature must be 64 bytes"))?;
    verifying
        .verify_strict(payload_canonical.as_bytes(), &signature)
        .map_err(|_| anyhow!("signature verification failed"))?;

    let zeroed_module = module_bytes_with_zeroed_hash(&module_bytes)?;
    let computed_module_hash = sha256_hex(&zeroed_module);

    let payload_module_hash = sig_file
        .payload
        .get("module_hash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("payload missing module_hash"))?;

    if sig_file.scope.includes_module() && payload_module_hash != computed_module_hash {
        return Err(anyhow!("payload module hash mismatch"));
    }

    let proof_bytes = find_proof_section(&module_bytes)?;
    let proof_section = decode_proof_section(&proof_bytes)?;
    if proof_section.module_hash.len() != 32 {
        return Err(anyhow!("proof section module hash must be 32 bytes"));
    }
    let section_module_hash = hex::encode(&proof_section.module_hash);
    if section_module_hash != computed_module_hash {
        return Err(anyhow!("proof section module hash mismatch"));
    }

    let section_proofs_hash = hex::encode(&proof_section.proofs_hash);
    let recomputed_proofs_hash = hex::encode(proofs_hash_from_section(&proof_section)?);
    if section_proofs_hash != recomputed_proofs_hash {
        return Err(anyhow!("proof section proofs hash mismatch"));
    }

    let payload_proofs_hash = sig_file
        .payload
        .get("proofs_hash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("payload missing proofs_hash"))?;

    if sig_file.scope.includes_proofs() && payload_proofs_hash != section_proofs_hash {
        return Err(anyhow!("payload proofs hash mismatch"));
    }

    Ok(())
}

fn find_proof_section(bytes: &[u8]) -> Result<Vec<u8>> {
    for payload in Parser::new(0).parse_all(bytes) {
        if let Payload::CustomSection(section) = payload? {
            if section.name() == "clearlang.proof" {
                return Ok(section.data().to_vec());
            }
        }
    }
    Err(anyhow!("clearlang.proof section not found"))
}
