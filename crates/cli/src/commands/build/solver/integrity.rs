fn verify_solver_integrity(solver_bin: PathBuf) -> Option<PathBuf> {
    if !solver_bin.is_file() {
        return None;
    }
    let checksum_path = sidecar_path(solver_bin.as_path(), "sha256");
    let signature_path = sidecar_path(solver_bin.as_path(), "sig");

    let checksum_entry = fs::read_to_string(&checksum_path).ok()?;
    let checksum_entry = checksum_entry
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())?;
    if !is_sha256_prefixed_hex(checksum_entry) {
        return None;
    }

    let bytes = fs::read(&solver_bin).ok()?;
    let computed_digest = format!("sha256:{}", sha256_hex(bytes.as_slice()));
    if checksum_entry != computed_digest {
        return None;
    }

    let signature_entry: SolverBundleSignatureV1 =
        serde_json::from_slice(fs::read(&signature_path).ok()?.as_slice()).ok()?;
    if signature_entry.schema_version != 1 {
        return None;
    }
    if signature_entry.scheme != "ed25519" {
        return None;
    }
    if !is_sha256_prefixed_hex(signature_entry.signed_payload.as_str()) {
        return None;
    }
    if signature_entry.signed_payload != checksum_entry {
        return None;
    }
    let trusted_signer = load_solver_signature_policy()?
        .into_iter()
        .find(|signer| signer.key_id == signature_entry.key_id)?;
    let signature_bytes = decode_signature_hex(signature_entry.signature.as_str())?;
    let signature = ed25519_dalek::Signature::from_bytes(&signature_bytes);
    if ed25519_dalek::Verifier::verify(
        &trusted_signer.verifying_key,
        signature_entry.signed_payload.as_bytes(),
        &signature,
    )
    .is_err()
    {
        return None;
    }

    Some(solver_bin)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SolverBundleSignatureV1 {
    schema_version: u32,
    key_id: String,
    scheme: String,
    signed_payload: String,
    signature: String,
}

#[derive(Debug, Clone)]
struct TrustedSolverSigner {
    key_id: String,
    verifying_key: ed25519_dalek::VerifyingKey,
}

fn load_solver_signature_policy() -> Option<Vec<TrustedSolverSigner>> {
    let lock: serde_json::Value = serde_json::from_str(SOLVER_SUPPLY_CHAIN_LOCK_JSON).ok()?;
    if lock
        .get("schema_version")
        .and_then(|value| value.as_u64())
        .unwrap_or_default()
        != 1
    {
        return None;
    }
    let bundle_integrity = lock.get("bundle_integrity")?.as_object()?;
    if bundle_integrity
        .get("signature_required")
        .and_then(|value| value.as_bool())
        != Some(true)
    {
        return None;
    }
    if bundle_integrity
        .get("signature_mode")
        .and_then(|value| value.as_str())
        != Some("publisher-auth-ed25519-v1")
    {
        return None;
    }
    if bundle_integrity
        .get("signature_schema_version")
        .and_then(|value| value.as_u64())
        != Some(1)
    {
        return None;
    }
    let allowed_statuses = bundle_integrity
        .get("rotation_policy")?
        .get("allowed_statuses")?
        .as_array()?;
    let allowed_statuses = allowed_statuses
        .iter()
        .filter_map(|entry| entry.as_str().map(ToString::to_string))
        .collect::<std::collections::BTreeSet<_>>();
    if allowed_statuses.is_empty() {
        return None;
    }
    let min_signers = bundle_integrity
        .get("rotation_policy")?
        .get("min_trusted_signers")?
        .as_u64()
        .unwrap_or(1);

    let mut trusted = Vec::new();
    for entry in bundle_integrity.get("trusted_signers")?.as_array()? {
        let key_id = entry.get("key_id")?.as_str()?.trim().to_string();
        let scheme = entry.get("scheme")?.as_str()?.trim();
        let status = entry.get("status")?.as_str()?.trim().to_string();
        let public_key = entry.get("public_key")?.as_str()?.trim();
        if key_id.is_empty() || scheme != "ed25519" || !allowed_statuses.contains(status.as_str())
        {
            continue;
        }
        let public_key_bytes: [u8; 32] = decode_hex_prefixed(public_key, 32)?.try_into().ok()?;
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&public_key_bytes).ok()?;
        trusted.push(TrustedSolverSigner {
            key_id,
            verifying_key,
        });
    }
    if trusted.len() < min_signers as usize {
        return None;
    }
    Some(trusted)
}

fn decode_hex_prefixed(value: &str, expected_len: usize) -> Option<Vec<u8>> {
    let hex = value.strip_prefix("hex:")?;
    let bytes = hex::decode(hex).ok()?;
    (bytes.len() == expected_len).then_some(bytes)
}

fn decode_signature_hex(value: &str) -> Option<[u8; 64]> {
    let raw = hex::decode(value).ok()?;
    let bytes: [u8; 64] = raw.try_into().ok()?;
    Some(bytes)
}

fn sidecar_path(solver_bin: &std::path::Path, suffix: &str) -> PathBuf {
    let mut name = solver_bin.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{suffix}"));
    solver_bin
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(name)
}

fn is_sha256_prefixed_hex(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
