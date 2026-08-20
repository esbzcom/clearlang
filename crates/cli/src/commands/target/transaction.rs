use anyhow::{anyhow, bail, Context, Result};
use k256::ecdsa::{Signature, SigningKey};
use serde::Deserialize;
use sha3::{Digest, Keccak256};

const KEY_FORMAT: &str = "clg.evm-signing-key.v1";

#[derive(Deserialize)]
struct KeyFile {
    format: String,
    private_key: String,
    address: String,
}

pub(super) struct EvmSigner {
    key: SigningKey,
    pub address: String,
}

pub(super) struct LegacyTransaction<'a> {
    pub chain_id: u64,
    pub nonce: u64,
    pub gas_price: u64,
    pub gas_limit: u64,
    pub to: Option<&'a str>,
    pub value: u64,
    pub data: &'a [u8],
}

pub(super) fn load_signer(path: &std::path::Path) -> Result<EvmSigner> {
    let bytes =
        std::fs::read(path).with_context(|| format!("read signing key `{}`", path.display()))?;
    let key_file: KeyFile = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse signing key `{}`", path.display()))?;
    if key_file.format != KEY_FORMAT {
        bail!("unsupported EVM signing key format `{}`", key_file.format)
    }
    let private_key =
        decode_hex(&key_file.private_key, 32).context("malformed EVM signing private key")?;
    let key = SigningKey::from_slice(&private_key)
        .map_err(|_| anyhow!("invalid secp256k1 private key"))?;
    let address = derive_address(&key);
    if normalize_address(&key_file.address)? != address {
        bail!("EVM signing key address does not match its private key")
    }
    Ok(EvmSigner { key, address })
}

pub(super) fn sign_legacy(
    transaction: &LegacyTransaction<'_>,
    signer: &EvmSigner,
) -> Result<Vec<u8>> {
    if transaction.chain_id == 0 || transaction.gas_limit == 0 || transaction.gas_price == 0 {
        bail!("EIP-155 transaction requires non-zero chain ID, gas limit, and gas price")
    }
    let to = transaction.to.map(normalize_address).transpose()?;
    let unsigned = rlp_list(&[
        rlp_u64(transaction.nonce),
        rlp_u64(transaction.gas_price),
        rlp_u64(transaction.gas_limit),
        rlp_bytes(
            to.as_deref()
                .map(hex_address_bytes)
                .transpose()?
                .as_deref()
                .unwrap_or_default(),
        ),
        rlp_u64(transaction.value),
        rlp_bytes(transaction.data),
        rlp_u64(transaction.chain_id),
        rlp_u64(0),
        rlp_u64(0),
    ]);
    let digest = Keccak256::digest(&unsigned);
    let (signature, recovery_id) = signer
        .key
        .sign_prehash_recoverable(&digest)
        .map_err(|_| anyhow!("failed to create recoverable secp256k1 signature"))?;
    let (r, s) = signature_parts(&signature);
    let v = transaction
        .chain_id
        .checked_mul(2)
        .and_then(|value| value.checked_add(35 + u64::from(recovery_id.to_byte())))
        .ok_or_else(|| anyhow!("EIP-155 signature v overflow"))?;
    Ok(rlp_list(&[
        rlp_u64(transaction.nonce),
        rlp_u64(transaction.gas_price),
        rlp_u64(transaction.gas_limit),
        rlp_bytes(
            to.as_deref()
                .map(hex_address_bytes)
                .transpose()?
                .as_deref()
                .unwrap_or_default(),
        ),
        rlp_u64(transaction.value),
        rlp_bytes(transaction.data),
        rlp_u64(v),
        rlp_bytes(trim_leading_zeroes(&r)),
        rlp_bytes(trim_leading_zeroes(&s)),
    ]))
}

pub(super) fn transaction_hash(raw_transaction: &[u8]) -> String {
    format!("0x{}", hex::encode(Keccak256::digest(raw_transaction)))
}

fn derive_address(key: &SigningKey) -> String {
    let point = key.verifying_key().to_encoded_point(false);
    let hash = Keccak256::digest(&point.as_bytes()[1..]);
    format!("0x{}", hex::encode(&hash[12..]))
}

fn normalize_address(value: &str) -> Result<String> {
    Ok(format!("0x{}", hex::encode(decode_hex(value, 20)?)))
}
fn hex_address_bytes(value: &str) -> Result<Vec<u8>> {
    decode_hex(value, 20)
}
fn decode_hex(value: &str, expected: usize) -> Result<Vec<u8>> {
    let bytes = hex::decode(
        value
            .strip_prefix("0x")
            .ok_or_else(|| anyhow!("hex value must be 0x-prefixed"))?,
    )?;
    if bytes.len() != expected {
        bail!("expected {expected} bytes")
    }
    Ok(bytes)
}

fn rlp_u64(value: u64) -> Vec<u8> {
    if value == 0 {
        rlp_bytes(&[])
    } else {
        rlp_bytes(trim_leading_zeroes(&value.to_be_bytes()))
    }
}
fn rlp_bytes(value: &[u8]) -> Vec<u8> {
    if value.len() == 1 && value[0] < 0x80 {
        return value.to_vec();
    }
    rlp_prefix(0x80, value)
}
fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload = items.concat();
    rlp_prefix(0xc0, &payload)
}
fn rlp_prefix(base: u8, payload: &[u8]) -> Vec<u8> {
    if payload.len() <= 55 {
        let mut out = vec![base + payload.len() as u8];
        out.extend(payload);
        out
    } else {
        let length_bytes = (payload.len() as u64).to_be_bytes();
        let length = trim_leading_zeroes(&length_bytes);
        let mut out = vec![base + 55 + length.len() as u8];
        out.extend(length);
        out.extend(payload);
        out
    }
}
fn trim_leading_zeroes(value: &[u8]) -> &[u8] {
    value
        .iter()
        .position(|byte| *byte != 0)
        .map(|index| &value[index..])
        .unwrap_or(&[])
}
fn signature_parts(signature: &Signature) -> ([u8; 32], [u8; 32]) {
    let bytes = signature.to_bytes();
    let mut r = [0; 32];
    let mut s = [0; 32];
    r.copy_from_slice(&bytes[..32]);
    s.copy_from_slice(&bytes[32..]);
    (r, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signer_derives_and_validates_its_evm_address() {
        let key = SigningKey::from_slice(&[1; 32]).expect("valid key");
        assert_eq!(
            derive_address(&key),
            "0x1a642f0e3c3af545e7acbd38b07251b3990914f1"
        );
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("signer.json");
        std::fs::write(
            &path,
            r#"{"format":"clg.evm-signing-key.v1","private_key":"0x0101010101010101010101010101010101010101010101010101010101010101","address":"0x1a642f0e3c3af545e7acbd38b07251b3990914f1"}"#,
        )
        .expect("write signer");
        assert_eq!(
            load_signer(&path).expect("load signer").address,
            derive_address(&key)
        );
    }

    #[test]
    fn legacy_eip155_signing_is_deterministic_and_changes_with_transaction_data() {
        let signer = EvmSigner {
            key: SigningKey::from_slice(&[1; 32]).expect("valid key"),
            address: "0x1a642f0e3c3af545e7acbd38b07251b3990914f1".to_string(),
        };
        let transaction = LegacyTransaction {
            chain_id: 1,
            nonce: 7,
            gas_price: 2,
            gas_limit: 21_000,
            to: Some("0x1111111111111111111111111111111111111111"),
            value: 3,
            data: &[0xaa],
        };
        let first = sign_legacy(&transaction, &signer).expect("sign first");
        let second = sign_legacy(&transaction, &signer).expect("sign second");
        assert_eq!(first, second);
        assert!(first.len() > 32);
        let changed = sign_legacy(
            &LegacyTransaction {
                data: &[0xbb],
                ..transaction
            },
            &signer,
        )
        .expect("sign changed");
        assert_ne!(first, changed);
    }
}
