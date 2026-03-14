use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;

use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;

use super::strict_package_contract::StrictPackageContractV0;
use super::strict_trust_policy::{StrictTrustPolicyV0, TrustedSignerV0};

const STRICT_PACKAGE_SIGNATURES_FILE: &str = "clg.package-signatures.json";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictPackageSignaturesError {
    code: &'static str,
    message: String,
}

impl StrictPackageSignaturesError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(super) fn code(&self) -> &'static str {
        self.code
    }

    pub(super) fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StrictPackageSignatureEntry {
    name: String,
    version: String,
    digest: String,
    key_id: String,
    signed_at: String,
    signature: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSignatureRoot {
    schema_version: u32,
    signatures: Vec<RawSignatureEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSignatureEntry {
    name: String,
    version: String,
    digest: String,
    key_id: String,
    signed_at: String,
    signature_format: String,
    signature: String,
}

pub(super) fn enforce_trust_gate_v0(
    root: &Path,
    package_contract: &StrictPackageContractV0,
    trust_policy: &StrictTrustPolicyV0,
) -> Result<(), StrictPackageSignaturesError> {
    let require_signatures = !package_contract.packages.is_empty();
    let signatures = load_package_signatures_v0(root, require_signatures)?;

    let mut signatures_by_name: HashMap<&str, &StrictPackageSignatureEntry> =
        HashMap::with_capacity(signatures.len());
    for signature in &signatures {
        signatures_by_name.insert(signature.name.as_str(), signature);
    }

    let trust_signers: HashMap<&str, &TrustedSignerV0> = trust_policy
        .trusted_signers
        .iter()
        .map(|signer| (signer.key_id.as_str(), signer))
        .collect();
    let revoked: HashSet<&str> = trust_policy
        .revoked_key_ids
        .iter()
        .map(|key_id| key_id.as_str())
        .collect();

    let mut used_signature_names = HashSet::with_capacity(package_contract.packages.len());
    for package in &package_contract.packages {
        let signature = signatures_by_name
            .get(package.name.as_str())
            .copied()
            .ok_or_else(|| {
                StrictPackageSignaturesError::new(
                    "C103",
                    format!(
                        "strict trust gate failed: package `{}` is missing signature entry in `{}`",
                        package.name, STRICT_PACKAGE_SIGNATURES_FILE
                    ),
                )
            })?;
        used_signature_names.insert(signature.name.as_str());

        if signature.version != package.version {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict trust gate failed for `{}`: signature version `{}` does not match package version `{}`",
                    package.name, signature.version, package.version
                ),
            ));
        }
        if signature.digest != package.digest {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict trust gate failed for `{}`: signature digest `{}` does not match package digest `{}`",
                    package.name, signature.digest, package.digest
                ),
            ));
        }

        let signer = trust_signers
            .get(signature.key_id.as_str())
            .copied()
            .ok_or_else(|| {
                StrictPackageSignaturesError::new(
                    "C103",
                    format!(
                        "strict trust gate failed for `{}`: signer `{}` is not trusted",
                        package.name, signature.key_id
                    ),
                )
            })?;
        if revoked.contains(signature.key_id.as_str()) {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict trust gate failed for `{}`: signer `{}` is revoked",
                    package.name, signature.key_id
                ),
            ));
        }

        let signed_at = parse_rfc3339_utc(&signature.signed_at, "signed_at")?;
        let not_before = parse_rfc3339_utc(&signer.not_before, "not_before")?;
        let not_after = parse_rfc3339_utc(&signer.not_after, "not_after")?;
        if signed_at < not_before || signed_at >= not_after {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict trust gate failed for `{}`: signer `{}` is not valid at signed_at `{}`",
                    package.name, signature.key_id, signature.signed_at
                ),
            ));
        }

        let verifying = verifying_key_from_signer(signer).map_err(|msg| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict trust gate failed for `{}`: signer `{}` public key is invalid: {}",
                    package.name, signer.key_id, msg
                ),
            )
        })?;
        let signature_bytes = decode_signature(&signature.signature).map_err(|msg| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict trust gate failed for `{}`: signature is invalid: {}",
                    package.name, msg
                ),
            )
        })?;
        let payload = canonical_payload(
            package.name.as_str(),
            package.version.as_str(),
            package.digest.as_str(),
            signature.signed_at.as_str(),
        );
        verifying
            .verify_strict(payload.as_bytes(), &signature_bytes)
            .map_err(|_| {
                StrictPackageSignaturesError::new(
                    "C103",
                    format!(
                        "strict trust gate failed for `{}`: signature verification failed for signer `{}`",
                        package.name, signature.key_id
                    ),
                )
            })?;
    }

    if let Some(extra) = signatures
        .iter()
        .find(|entry| !used_signature_names.contains(entry.name.as_str()))
    {
        return Err(StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict trust gate failed: signature entry `{}` is not referenced by package metadata",
                extra.name
            ),
        ));
    }

    Ok(())
}

fn load_package_signatures_v0(
    root: &Path,
    required: bool,
) -> Result<Vec<StrictPackageSignatureEntry>, StrictPackageSignaturesError> {
    let path = root.join(STRICT_PACKAGE_SIGNATURES_FILE);
    if !path.exists() {
        if required {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict mode requires `{}` at `{}` when package metadata contains dependencies",
                    STRICT_PACKAGE_SIGNATURES_FILE,
                    path.display()
                ),
            ));
        }
        return Ok(Vec::new());
    }
    if !path.is_file() {
        return Err(StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict package signatures path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }
    let content = fs::read_to_string(&path).map_err(|err| {
        StrictPackageSignaturesError::new(
            "C103",
            format!(
                "failed to read strict package signatures `{}`: {err}",
                path.display()
            ),
        )
    })?;
    parse_package_signatures_v0(&content, &path)
}

fn parse_package_signatures_v0(
    content: &str,
    path: &Path,
) -> Result<Vec<StrictPackageSignatureEntry>, StrictPackageSignaturesError> {
    let value: serde_json::Value = serde_json::from_str(content).map_err(|_| {
        StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict package signatures `{}` are not valid JSON (expected schema v0 object)",
                path.display()
            ),
        )
    })?;
    let raw: RawSignatureRoot = serde_json::from_value(value).map_err(|_| {
        StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict package signatures `{}` do not match schema v0 (`schema_version`, `signatures[]`)",
                path.display()
            ),
        )
    })?;
    if raw.schema_version != 0 {
        return Err(StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict package signatures `{}` have unsupported schema_version {}; expected 0",
                path.display(),
                raw.schema_version
            ),
        ));
    }

    let mut signatures: Vec<StrictPackageSignatureEntry> = raw
        .signatures
        .into_iter()
        .map(|entry| StrictPackageSignatureEntry {
            name: entry.name,
            version: entry.version,
            digest: entry.digest,
            key_id: entry.key_id,
            signed_at: entry.signed_at,
            signature: entry.signature,
        })
        .collect();
    signatures.sort_by(|a, b| a.name.cmp(&b.name));

    let mut seen = HashSet::with_capacity(signatures.len());
    let mut duplicates = BTreeSet::new();
    for signature in &signatures {
        if !seen.insert(signature.name.clone()) {
            duplicates.insert(signature.name.clone());
        }
    }
    if let Some(dupe) = duplicates.iter().next() {
        return Err(StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict package signatures `{}` have duplicate package name `{}`",
                path.display(),
                dupe
            ),
        ));
    }

    for entry in &signatures {
        validate_package_name(entry.name.as_str()).map_err(|msg| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` is invalid: {msg}",
                    path.display(),
                    entry.name
                ),
            )
        })?;
        validate_exact_semver(entry.version.as_str()).map_err(|msg| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` has invalid version `{}`: {msg}",
                    path.display(),
                    entry.name,
                    entry.version
                ),
            )
        })?;
        validate_sha256_digest(entry.digest.as_str()).map_err(|msg| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` has invalid digest `{}`: {msg}",
                    path.display(),
                    entry.name,
                    entry.digest
                ),
            )
        })?;
        if entry.key_id.trim().is_empty() {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` has empty key_id",
                    path.display(),
                    entry.name
                ),
            ));
        }
        parse_rfc3339_utc(entry.signed_at.as_str(), "signed_at").map_err(|_| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` has invalid signed_at `{}`",
                    path.display(),
                    entry.name,
                    entry.signed_at
                ),
            )
        })?;
    }

    // Validate signature_format from raw input while still preserving deny_unknown_fields behavior.
    let raw_again: RawSignatureRoot = serde_json::from_str(content).map_err(|_| {
        StrictPackageSignaturesError::new(
            "C103",
            format!(
                "strict package signatures `{}` are not valid JSON (expected schema v0 object)",
                path.display()
            ),
        )
    })?;
    for entry in &raw_again.signatures {
        if entry.signature_format != "ed25519" {
            return Err(StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` has unsupported signature_format `{}`; expected `ed25519`",
                    path.display(),
                    entry.name,
                    entry.signature_format
                ),
            ));
        }
        decode_signature(entry.signature.as_str()).map_err(|msg| {
            StrictPackageSignaturesError::new(
                "C103",
                format!(
                    "strict package signatures `{}` package `{}` has invalid signature: {msg}",
                    path.display(),
                    entry.name
                ),
            )
        })?;
    }

    Ok(signatures)
}

fn parse_rfc3339_utc(
    value: &str,
    field: &str,
) -> Result<(u16, u8, u8, u8, u8, u8), StrictPackageSignaturesError> {
    parse_utc_timestamp_components(value).map_err(|msg| {
        StrictPackageSignaturesError::new(
            "C103",
            format!("{field} must be a valid UTC RFC3339 timestamp: {msg}"),
        )
    })
}

fn parse_utc_timestamp_components(value: &str) -> Result<(u16, u8, u8, u8, u8, u8), String> {
    if value.len() != 20 {
        return Err("expected `YYYY-MM-DDTHH:MM:SSZ`".to_string());
    }
    let bytes = value.as_bytes();
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return Err("expected separators in `YYYY-MM-DDTHH:MM:SSZ`".to_string());
    }

    fn parse_u16(s: &str) -> Result<u16, String> {
        if !s.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("`{s}` contains non-digit characters"));
        }
        s.parse::<u16>()
            .map_err(|_| format!("failed to parse `{s}`"))
    }
    fn parse_u8(s: &str) -> Result<u8, String> {
        if !s.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("`{s}` contains non-digit characters"));
        }
        s.parse::<u8>()
            .map_err(|_| format!("failed to parse `{s}`"))
    }

    let year = parse_u16(&value[0..4])?;
    let month = parse_u8(&value[5..7])?;
    let day = parse_u8(&value[8..10])?;
    let hour = parse_u8(&value[11..13])?;
    let minute = parse_u8(&value[14..16])?;
    let second = parse_u8(&value[17..19])?;

    if !(1..=12).contains(&month) {
        return Err(format!("month `{month}` is out of range 1..=12"));
    }
    if !(1..=31).contains(&day) {
        return Err(format!("day `{day}` is out of range 1..=31"));
    }
    let max_day = days_in_month(year, month);
    if day > max_day {
        return Err(format!(
            "day `{day}` is out of range 1..={max_day} for month `{month}`"
        ));
    }
    if hour > 23 {
        return Err(format!("hour `{hour}` is out of range 0..=23"));
    }
    if minute > 59 {
        return Err(format!("minute `{minute}` is out of range 0..=59"));
    }
    if second > 59 {
        return Err(format!("second `{second}` is out of range 0..=59"));
    }

    Ok((year, month, day, hour, minute, second))
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn verifying_key_from_signer(signer: &TrustedSignerV0) -> Result<VerifyingKey, &'static str> {
    const PREFIX: &str = "hex:";
    if !signer.public_key.starts_with(PREFIX) {
        return Err("public_key must start with `hex:`");
    }
    let key_hex = &signer.public_key[PREFIX.len()..];
    let key_raw = hex::decode(key_hex).map_err(|_| "public_key is not valid hex")?;
    let key_bytes: [u8; 32] = key_raw
        .try_into()
        .map_err(|_| "public_key must be exactly 32 bytes")?;
    VerifyingKey::from_bytes(&key_bytes).map_err(|_| "public_key bytes are invalid")
}

fn decode_signature(value: &str) -> Result<Signature, &'static str> {
    let raw = hex::decode(value).map_err(|_| "signature is not valid hex")?;
    Signature::try_from(raw.as_slice()).map_err(|_| "signature must be exactly 64 bytes")
}

fn canonical_payload(name: &str, version: &str, digest: &str, signed_at: &str) -> String {
    format!("clg-package-signature-v0\n{name}\n{version}\n{digest}\n{signed_at}\n")
}

fn validate_package_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("name is empty".to_string());
    }
    for segment in name.split("::") {
        validate_identifier_segment(segment)?;
    }
    Ok(())
}

fn validate_identifier_segment(segment: &str) -> Result<(), String> {
    if segment.is_empty() {
        return Err("contains empty `::` segment".to_string());
    }
    let mut chars = segment.chars();
    let first = chars.next().expect("segment non-empty");
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(format!(
            "segment `{segment}` must start with ASCII letter or `_`"
        ));
    }
    for ch in chars {
        if !(ch == '_' || ch.is_ascii_alphanumeric()) {
            return Err(format!(
                "segment `{segment}` contains invalid character `{ch}`"
            ));
        }
    }
    Ok(())
}

fn validate_exact_semver(version: &str) -> Result<(), String> {
    if version.contains('-') || version.contains('+') {
        return Err(
            "must use exact MAJOR.MINOR.PATCH without pre-release/build metadata".to_string(),
        );
    }
    if !version.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        return Err("must use exact MAJOR.MINOR.PATCH".to_string());
    }
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return Err("must use exact MAJOR.MINOR.PATCH".to_string());
    }
    for part in parts {
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("version segment `{part}` is not numeric"));
        }
    }
    Ok(())
}

fn validate_sha256_digest(digest: &str) -> Result<(), String> {
    const PREFIX: &str = "sha256:";
    if !digest.starts_with(PREFIX) {
        return Err("digest must start with `sha256:`".to_string());
    }
    let hex = &digest[PREFIX.len()..];
    if hex.len() != 64 {
        return Err("digest must have exactly 64 lowercase hex characters".to_string());
    }
    if !hex
        .chars()
        .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return Err("digest must be lowercase hex (`0-9`, `a-f`)".to_string());
    }
    Ok(())
}

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
}
