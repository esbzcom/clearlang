use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

pub(super) const STRICT_TRUST_POLICY_FILE: &str = "clg.trust-policy.json";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictTrustPolicyV0 {
    pub(super) trusted_signers: Vec<TrustedSignerV0>,
    pub(super) revoked_key_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TrustedSignerV0 {
    pub(super) key_id: String,
    pub(super) scheme: String,
    pub(super) public_key: String,
    pub(super) not_before: String,
    pub(super) not_after: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictTrustPolicyError {
    code: &'static str,
    message: String,
}

impl StrictTrustPolicyError {
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTrustPolicyV0 {
    schema_version: u32,
    trusted_signers: Vec<RawTrustedSignerV0>,
    revoked_key_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTrustedSignerV0 {
    key_id: String,
    scheme: String,
    public_key: String,
    not_before: String,
    not_after: String,
}

pub(super) fn load_required_trust_policy_v0(
    root: &Path,
) -> Result<StrictTrustPolicyV0, StrictTrustPolicyError> {
    let path = root.join(STRICT_TRUST_POLICY_FILE);
    if !path.exists() {
        return Err(StrictTrustPolicyError::new(
            "C103",
            format!(
                "strict mode requires `{}` at `{}`",
                STRICT_TRUST_POLICY_FILE,
                path.display()
            ),
        ));
    }
    if !path.is_file() {
        return Err(StrictTrustPolicyError::new(
            "C103",
            format!(
                "strict trust policy path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }

    let content = fs::read_to_string(&path).map_err(|err| {
        StrictTrustPolicyError::new(
            "C103",
            format!(
                "failed to read strict trust policy `{}`: {err}",
                path.display()
            ),
        )
    })?;

    parse_trust_policy_v0(&content, &path)
}

fn parse_trust_policy_v0(
    content: &str,
    path: &Path,
) -> Result<StrictTrustPolicyV0, StrictTrustPolicyError> {
    let value: serde_json::Value = serde_json::from_str(content).map_err(|_| {
        StrictTrustPolicyError::new(
            "C103",
            format!(
                "strict trust policy `{}` is not valid JSON (expected schema v0 object)",
                path.display()
            ),
        )
    })?;
    let raw: RawTrustPolicyV0 = serde_json::from_value(value).map_err(|_| {
        StrictTrustPolicyError::new(
            "C103",
            format!(
                "strict trust policy `{}` does not match schema v0 (`schema_version`, `trusted_signers`, `revoked_key_ids`)",
                path.display()
            ),
        )
    })?;
    if raw.schema_version != 0 {
        return Err(StrictTrustPolicyError::new(
            "C103",
            format!(
                "strict trust policy `{}` has unsupported schema_version {}; expected 0",
                path.display(),
                raw.schema_version
            ),
        ));
    }

    let mut signers: Vec<TrustedSignerV0> = raw
        .trusted_signers
        .into_iter()
        .map(|raw_signer| TrustedSignerV0 {
            key_id: raw_signer.key_id,
            scheme: raw_signer.scheme,
            public_key: raw_signer.public_key,
            not_before: raw_signer.not_before,
            not_after: raw_signer.not_after,
        })
        .collect();
    signers.sort_by(|a, b| a.key_id.cmp(&b.key_id));

    let mut signer_ids = HashSet::with_capacity(signers.len());
    let mut signer_dupes = BTreeSet::new();
    for signer in &signers {
        if !signer_ids.insert(signer.key_id.clone()) {
            signer_dupes.insert(signer.key_id.clone());
        }
    }
    if let Some(dupe) = signer_dupes.iter().next() {
        return Err(StrictTrustPolicyError::new(
            "C103",
            format!(
                "strict trust policy `{}` has duplicate trusted signer `key_id` `{}`",
                path.display(),
                dupe
            ),
        ));
    }

    for signer in &signers {
        validate_non_empty("key_id", &signer.key_id).map_err(|msg| {
            StrictTrustPolicyError::new(
                "C103",
                format!(
                    "strict trust policy `{}` signer `{}` is invalid: {msg}",
                    path.display(),
                    signer.key_id
                ),
            )
        })?;
        if signer.scheme != "ed25519" {
            return Err(StrictTrustPolicyError::new(
                "C103",
                format!(
                    "strict trust policy `{}` signer `{}` has unsupported scheme `{}`; expected `ed25519`",
                    path.display(),
                    signer.key_id,
                    signer.scheme
                ),
            ));
        }
        validate_public_key(&signer.public_key).map_err(|msg| {
            StrictTrustPolicyError::new(
                "C103",
                format!(
                    "strict trust policy `{}` signer `{}` has invalid public_key: {msg}",
                    path.display(),
                    signer.key_id
                ),
            )
        })?;
        let not_before = parse_utc_rfc3339("not_before", &signer.not_before).map_err(|msg| {
            StrictTrustPolicyError::new(
                "C103",
                format!(
                    "strict trust policy `{}` signer `{}` has invalid not_before: {msg}",
                    path.display(),
                    signer.key_id
                ),
            )
        })?;
        let not_after = parse_utc_rfc3339("not_after", &signer.not_after).map_err(|msg| {
            StrictTrustPolicyError::new(
                "C103",
                format!(
                    "strict trust policy `{}` signer `{}` has invalid not_after: {msg}",
                    path.display(),
                    signer.key_id
                ),
            )
        })?;
        if not_after <= not_before {
            return Err(StrictTrustPolicyError::new(
                "C103",
                format!(
                    "strict trust policy `{}` signer `{}` must satisfy not_before < not_after",
                    path.display(),
                    signer.key_id
                ),
            ));
        }
    }

    let mut revoked = raw.revoked_key_ids;
    revoked.sort();
    let mut revoked_set = HashSet::with_capacity(revoked.len());
    let mut revoked_dupes = BTreeSet::new();
    for key_id in &revoked {
        if !revoked_set.insert(key_id.clone()) {
            revoked_dupes.insert(key_id.clone());
        }
    }
    if let Some(dupe) = revoked_dupes.iter().next() {
        return Err(StrictTrustPolicyError::new(
            "C103",
            format!(
                "strict trust policy `{}` has duplicate revoked key id `{}`",
                path.display(),
                dupe
            ),
        ));
    }

    let signer_ids: HashSet<&str> = signers.iter().map(|s| s.key_id.as_str()).collect();
    let mut unknown_revoked = BTreeSet::new();
    for key_id in &revoked {
        validate_non_empty("revoked_key_id", key_id).map_err(|msg| {
            StrictTrustPolicyError::new(
                "C103",
                format!(
                    "strict trust policy `{}` has invalid revoked key id `{}`: {msg}",
                    path.display(),
                    key_id
                ),
            )
        })?;
        if !signer_ids.contains(key_id.as_str()) {
            unknown_revoked.insert(key_id.clone());
        }
    }
    if let Some(key_id) = unknown_revoked.iter().next() {
        return Err(StrictTrustPolicyError::new(
            "C103",
            format!(
                "strict trust policy `{}` revoked key id `{}` is not present in trusted_signers",
                path.display(),
                key_id
            ),
        ));
    }

    Ok(StrictTrustPolicyV0 {
        trusted_signers: signers,
        revoked_key_ids: revoked,
    })
}

fn validate_non_empty(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} is empty"));
    }
    Ok(())
}

fn validate_public_key(value: &str) -> Result<(), String> {
    const PREFIX: &str = "hex:";
    if !value.starts_with(PREFIX) {
        return Err("public_key must start with `hex:`".to_string());
    }
    let hex = &value[PREFIX.len()..];
    if hex.len() != 64 {
        return Err("public_key must contain exactly 64 lowercase hex characters".to_string());
    }
    if !hex
        .chars()
        .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return Err("public_key must be lowercase hex (`0-9`, `a-f`)".to_string());
    }
    Ok(())
}

fn parse_utc_rfc3339(field: &str, value: &str) -> Result<(u16, u8, u8, u8, u8, u8), String> {
    let ts = parse_utc_timestamp_components(value)
        .map_err(|msg| format!("{field} must be a valid UTC RFC3339 timestamp: {msg}"))?;
    Ok(ts)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn valid_policy_sorts_signers_and_revocations() {
        let json = r#"
{
  "schema_version": 0,
  "trusted_signers": [
    {
      "key_id": "z",
      "scheme": "ed25519",
      "public_key": "hex:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    },
    {
      "key_id": "a",
      "scheme": "ed25519",
      "public_key": "hex:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    }
  ],
  "revoked_key_ids": ["z", "a"]
}
        "#;
        let parsed = parse_trust_policy_v0(json, Path::new("clg.trust-policy.json"))
            .expect("valid trust policy");
        let signer_ids: Vec<&str> = parsed
            .trusted_signers
            .iter()
            .map(|signer| signer.key_id.as_str())
            .collect();
        assert_eq!(signer_ids, vec!["a", "z"]);
        assert_eq!(parsed.revoked_key_ids, vec!["a", "z"]);
    }

    #[test]
    fn missing_file_fails_with_c103() {
        let tmp = tempdir().expect("tempdir");
        let err = load_required_trust_policy_v0(tmp.path()).expect_err("expected missing-file err");
        assert_eq!(err.code(), "C103");
        assert!(err.message().contains("strict mode requires"));
    }

    #[test]
    fn malformed_schema_reports_c103() {
        let json = r#"
{
  "schema_version": 0,
  "trusted_signers": [],
  "revoked_key_ids": [],
  "extra": true
}
        "#;
        let err = parse_trust_policy_v0(json, Path::new("clg.trust-policy.json"))
            .expect_err("expected schema error");
        assert_eq!(err.code(), "C103");
        assert!(err.message().contains("does not match schema v0"));
    }

    #[test]
    fn duplicate_signers_report_lexicographically_first_key_id() {
        let json = r#"
{
  "schema_version": 0,
  "trusted_signers": [
    {
      "key_id": "z",
      "scheme": "ed25519",
      "public_key": "hex:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    },
    {
      "key_id": "a",
      "scheme": "ed25519",
      "public_key": "hex:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    },
    {
      "key_id": "z",
      "scheme": "ed25519",
      "public_key": "hex:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    },
    {
      "key_id": "a",
      "scheme": "ed25519",
      "public_key": "hex:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    }
  ],
  "revoked_key_ids": []
}
        "#;
        let err = parse_trust_policy_v0(json, Path::new("clg.trust-policy.json"))
            .expect_err("expected duplicate signer error");
        assert_eq!(err.code(), "C103");
        assert!(err.message().contains("`a`"), "message: {}", err.message());
    }

    #[test]
    fn revoked_key_id_must_exist_in_signers() {
        let json = r#"
{
  "schema_version": 0,
  "trusted_signers": [
    {
      "key_id": "a",
      "scheme": "ed25519",
      "public_key": "hex:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    }
  ],
  "revoked_key_ids": ["x"]
}
        "#;
        let err = parse_trust_policy_v0(json, Path::new("clg.trust-policy.json"))
            .expect_err("expected unknown revoked key id error");
        assert_eq!(err.code(), "C103");
        assert!(err.message().contains("not present in trusted_signers"));
    }

    #[test]
    fn signer_time_window_must_be_strictly_increasing() {
        let json = r#"
{
  "schema_version": 0,
  "trusted_signers": [
    {
      "key_id": "a",
      "scheme": "ed25519",
      "public_key": "hex:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "not_before": "2027-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    }
  ],
  "revoked_key_ids": []
}
        "#;
        let err = parse_trust_policy_v0(json, Path::new("clg.trust-policy.json"))
            .expect_err("expected timestamp-order error");
        assert_eq!(err.code(), "C103");
        assert!(err.message().contains("not_before < not_after"));
    }
}
