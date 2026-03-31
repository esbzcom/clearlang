pub fn load_solver_profile_claim() -> Result<(serde_json::Value, String)> {
    let lock: serde_json::Value = serde_json::from_str(SOLVER_PROFILE_LOCK_JSON)
        .map_err(|err| anyhow!("invalid solver profile lock JSON: {err}"))?;
    if lock
        .get("schema_version")
        .and_then(|value| value.as_u64())
        .unwrap_or_default()
        != 1
    {
        return Err(anyhow!(
            "unsupported solver profile lock schema_version (expected 1)"
        ));
    }
    let solver_profile = lock
        .get("solver_profile")
        .cloned()
        .ok_or_else(|| anyhow!("solver profile lock missing solver_profile object"))?;
    let solver_profile_hash = sha256_prefixed_hex(canonical_json_bytes(&solver_profile).as_slice());
    Ok((solver_profile, solver_profile_hash))
}

pub fn write_proof_artifact_json(
    vcs: &[VerificationCondition],
    path: &Path,
    toolchain: &str,
    compiler_mode: &str,
) -> Result<ProofArtifactEmission> {
    use serde_json::json;

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }

    let (solver_profile, solver_profile_hash) = load_solver_profile_claim()?;

    let mut ordered_vcs: Vec<&VerificationCondition> = vcs.iter().collect();
    ordered_vcs.sort_by(|left, right| {
        left.function
            .cmp(&right.function)
            .then_with(|| left.vc_id.cmp(&right.vc_id))
    });

    let mut proved_count = 0u64;
    let mut failed_count = 0u64;
    let mut unknown_count = 0u64;
    let mut timeout_count = 0u64;
    let mut generated_count = 0u64;
    let mut assumed_vcs = 0u64;
    let mut vc_items = Vec::with_capacity(ordered_vcs.len());

    for vc in ordered_vcs {
        let status = normalize_solver_status(vc.status);
        match status {
            "proved" => proved_count += 1,
            "failed" => failed_count += 1,
            "unknown" => unknown_count += 1,
            "timeout" => timeout_count += 1,
            _ => generated_count += 1,
        }
        let mut assumption_boundaries: Vec<String> = vc
            .assumptions
            .iter()
            .map(|boundary| boundary.id.to_string())
            .collect();
        assumption_boundaries.sort();
        assumption_boundaries.dedup();
        if !assumption_boundaries.is_empty() {
            assumed_vcs += 1;
        }

        let vc_hash_payload = json!({
            "function": vc.function,
            "vc_id": vc.vc_id,
            "pre_smt2": vc.pre.smt2,
            "post_smt2": vc.post.smt2,
            "vc_smt2": vc.vc_smt2,
            "assumption_boundaries": assumption_boundaries,
        });
        let vc_hash = sha256_prefixed_hex(canonical_json_bytes(&vc_hash_payload).as_slice());
        let mut entry = serde_json::Map::new();
        entry.insert("function".to_string(), json!(vc.function));
        entry.insert("vc_id".to_string(), json!(vc.vc_id));
        entry.insert("status".to_string(), json!(status));
        entry.insert(
            "assumption_boundaries".to_string(),
            json!(assumption_boundaries),
        );
        entry.insert("vc_hash".to_string(), json!(vc_hash));
        if status != "proved" {
            entry.insert(
                "counterexample".to_string(),
                json!({
                    "state": "solver_unavailable",
                    "format": "clg.counterexample.v1",
                    "reason": "external solver/model not attached",
                    "bindings": [],
                }),
            );
        }
        vc_items.push(serde_json::Value::Object(entry));
    }

    let artifact = json!({
        "format": "clg.proof_artifact.v1",
        "schema_version": 1,
        "generated_by": toolchain,
        "compiler_mode": compiler_mode,
        "solver_profile": solver_profile.clone(),
        "solver_profile_hash": solver_profile_hash.clone(),
        "summary": {
            "total_vcs": vc_items.len() as u64,
            "proved_count": proved_count,
            "failed_count": failed_count,
            "unknown_count": unknown_count,
            "timeout_count": timeout_count,
            "generated_count": generated_count,
            "assumed_vcs": assumed_vcs,
        },
        "vcs": vc_items,
    });
    let artifact_hash = sha256_prefixed_hex(canonical_json_bytes(&artifact).as_slice());
    let bytes = serde_json::to_vec_pretty(&artifact)?;
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))?;

    Ok(ProofArtifactEmission {
        artifact_hash,
        solver_profile_hash,
        solver_profile,
    })
}

fn proof_assurance_for_tier(tier: &str) -> ProofAssurance {
    ProofAssurance {
        tier: tier.to_string(),
        label: assurance_label_for_tier(tier).to_string(),
        levels: ProofAssuranceLevels {
            l0: assurance_label_for_tier("L0").to_string(),
            l1: assurance_label_for_tier("L1").to_string(),
            l2: assurance_label_for_tier("L2").to_string(),
            l3: assurance_label_for_tier("L3").to_string(),
        },
    }
}

fn assurance_label_for_tier(tier: &str) -> &'static str {
    match tier {
        "L0" => "assumed",
        "L1" => "checked core",
        "L2" => "verified module",
        "L3" => "verified package profile",
        _ => "unknown",
    }
}

fn normalize_solver_status(status: &str) -> &'static str {
    match status {
        "proved" => "proved",
        "failed" => "failed",
        "unknown" => "unknown",
        "timeout" => "timeout",
        "generated" => "generated",
        _ => "generated",
    }
}

fn canonicalize_json_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for key in keys {
                out.insert(key.clone(), canonicalize_json_value(&map[key]));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(canonicalize_json_value).collect())
        }
        _ => value.clone(),
    }
}

fn canonical_json_bytes(value: &serde_json::Value) -> Vec<u8> {
    let canonical = canonicalize_json_value(value);
    serde_json::to_vec(&canonical).expect("canonical json serialization")
}

fn sha256_prefixed_hex(bytes: &[u8]) -> String {
    format!("sha256:{}", sha256_hex(bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut output = String::with_capacity(digest.len() * 2);
    for b in digest {
        write!(&mut output, "{:02x}", b).expect("write hex");
    }
    output
}

fn canonical_name_for(
    emitted_name: &str,
    mangled_name_origins: &HashMap<String, String>,
) -> Option<String> {
    let canonical = mangled_name_origins.get(emitted_name)?;
    if canonical == emitted_name {
        return None;
    }
    Some(canonical.clone())
}

pub fn compute_proofs_hash_from_vcs(vcs: &[(String, ProofVc)]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for (name, vc) in vcs {
        let bytes = to_cbor_bytes(&(name, vc)).expect("serialize vc");
        hasher.update(bytes);
    }
    hasher.finalize().into()
}

pub fn to_cbor_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut serializer = serde_cbor::ser::Serializer::new(&mut buf);
    value.serialize(&mut serializer)?;
    Ok(buf)
}

pub fn decode_proof_section(bytes: &[u8]) -> Result<ProofSection> {
    let section: ProofSection = serde_cbor::from_slice(bytes)?;
    Ok(section)
}

pub fn hash_module(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

