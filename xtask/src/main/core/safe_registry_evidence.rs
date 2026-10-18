const SAFE_EVIDENCE_CAPTURE_INPUT_FORMAT: &str = "clg.safe-registry-evidence-capture-input.v1";
const SAFE_EVIDENCE_OBSERVATION_FORMAT: &str = "clg.safe-registry-observation-input.v1";
const SAFE_TARGET_EVIDENCE_FORMAT: &str = "clg.safe-registry-target-evidence.v1";
const SAFE_IDENTITY_ANCHOR_FORMAT: &str = "clg.safe-registry-identity-anchor.v1";
const SAFE_REPLAY_REPORT_FORMAT: &str = "clg.safe-registry-evidence-replay-report.v1";
const SAFE_EVIDENCE_PROFILE: &str = "clg.evm-ethereum-mainnet-safe-registry.v1";

#[derive(Clone, Debug, PartialEq, Eq)]
struct SafeEvidenceCaptureOpts {
    root: PathBuf,
    input: PathBuf,
    out: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SafeEvidenceReplayOpts {
    input: PathBuf,
    anchor: PathBuf,
    report_out: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeEvidenceTarget {
    profile: String,
    chain_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeEvidenceIdentity {
    source_profile_proof_sha256: String,
    state_schema_abi_sha256: String,
    evm_artifact_sha256: String,
    release_bundle_sha256: String,
    policy_sha256: String,
    safe_identity_sha256: String,
    observer_policy_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeEvidenceCaptureRef {
    id: String,
    record_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeEvidenceCaptureInput {
    format: String,
    schema_version: u32,
    target: SafeEvidenceTarget,
    identity: SafeEvidenceIdentity,
    observations: Vec<SafeEvidenceCaptureRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeObserver {
    observer_id: String,
    observer_class: String,
    client_family: String,
    client_version: String,
    exchange_digests: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeEvidenceObservation {
    format: String,
    schema_version: u32,
    id: String,
    kind: String,
    subject_action_id: String,
    captured_at: String,
    primary_observer: SafeObserver,
    independent_observer: SafeObserver,
    block_number: String,
    block_hash: String,
    status: String,
    comparison_result: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeEvidenceCapture {
    source: String,
    secret_free: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeEvidenceSignature {
    status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeTargetEvidence {
    format: String,
    schema_version: u32,
    target: SafeEvidenceTarget,
    identity: SafeEvidenceIdentity,
    capture: SafeEvidenceCapture,
    observations: Vec<SafeEvidenceObservation>,
    correspondence_status: String,
    evidence_signature: SafeEvidenceSignature,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeEvidenceIdentityAnchor {
    format: String,
    schema_version: u32,
    target: SafeEvidenceTarget,
    identity: SafeEvidenceIdentity,
    verified_release_bundle_sha256: String,
    verifier_result_sha256: String,
    verifier_identity_sha256: String,
    trusted_release_key_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeReplayObservationResult {
    id: String,
    kind: String,
    result: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeReplayReport {
    format: String,
    schema_version: u32,
    evidence_sha256: Option<String>,
    identity_anchor_sha256: Option<String>,
    target_profile: Option<String>,
    chain_id: Option<String>,
    status: String,
    diagnostic: String,
    observation_results: Vec<SafeReplayObservationResult>,
    network_contacted: bool,
    production_claim: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SafeEvidenceError {
    code: &'static str,
}

impl SafeEvidenceError {
    fn new(code: &'static str) -> Self {
        Self { code }
    }
}

fn run_safe_evidence_capture(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_safe_evidence_capture_args(raw_args)?;
    let capture_root = resolve_root_path(root, &opts.root)?;
    let input_path = resolve_within_safe_evidence_root(&capture_root, &opts.input)?;
    let output_path = resolve_within_safe_evidence_root(&capture_root, &opts.out)?;
    if output_path.exists() {
        return Err("safe-evidence-capture refuses to overwrite an existing output".to_string());
    }
    let evidence = capture_safe_target_evidence(&capture_root, &input_path)?;
    atomic_write_safe_evidence_json(&output_path, &evidence)
}

fn run_safe_evidence_replay(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_safe_evidence_replay_args(raw_args)?;
    let input_path = resolve_root_path(root, &opts.input)?;
    let anchor_path = resolve_root_path(root, &opts.anchor)?;
    let report_path = resolve_root_path(root, &opts.report_out)?;
    if report_path.exists() {
        return Err("safe-evidence-replay refuses to overwrite an existing report".to_string());
    }
    let outcome = replay_safe_target_evidence(&input_path, &anchor_path);
    let report = match &outcome {
        Ok((digest, anchor_digest, target, results)) => SafeReplayReport {
            format: SAFE_REPLAY_REPORT_FORMAT.to_string(),
            schema_version: 1,
            evidence_sha256: Some(digest.clone()),
            identity_anchor_sha256: Some(anchor_digest.clone()),
            target_profile: Some(target.profile.clone()),
            chain_id: Some(target.chain_id.clone()),
            status: "replayed".to_string(),
            diagnostic: "M6C000".to_string(),
            observation_results: results.clone(),
            network_contacted: false,
            production_claim: "not_established".to_string(),
        },
        Err(error) => SafeReplayReport {
            format: SAFE_REPLAY_REPORT_FORMAT.to_string(),
            schema_version: 1,
            evidence_sha256: None,
            identity_anchor_sha256: None,
            target_profile: None,
            chain_id: None,
            status: if matches!(error.code, "M6C005" | "M6C006" | "M6C007") {
                "unaccepted".to_string()
            } else {
                "rejected".to_string()
            },
            diagnostic: error.code.to_string(),
            observation_results: Vec::new(),
            network_contacted: false,
            production_claim: "not_established".to_string(),
        },
    };
    atomic_write_safe_evidence_json(&report_path, &report)?;
    outcome.map(|_| ()).map_err(|error| format!("safe-evidence-replay rejected evidence with {}", error.code))
}

fn parse_safe_evidence_capture_args(raw_args: Vec<String>) -> Result<SafeEvidenceCaptureOpts, String> {
    let mut root = None;
    let mut input = None;
    let mut out = None;
    let mut index = 0usize;
    while index < raw_args.len() {
        let slot = match raw_args[index].as_str() {
            "--root" => &mut root,
            "--input" => &mut input,
            "--out" => &mut out,
            other => return Err(format!("unknown safe-evidence-capture arg '{other}'")),
        };
        index += 1;
        let value = raw_args.get(index).ok_or_else(|| "missing capture argument value".to_string())?;
        if slot.replace(PathBuf::from(value)).is_some() {
            return Err("safe-evidence-capture arguments may be supplied only once".to_string());
        }
        index += 1;
    }
    Ok(SafeEvidenceCaptureOpts {
        root: root.ok_or_else(|| "missing required --root".to_string())?,
        input: input.ok_or_else(|| "missing required --input".to_string())?,
        out: out.ok_or_else(|| "missing required --out".to_string())?,
    })
}

fn parse_safe_evidence_replay_args(raw_args: Vec<String>) -> Result<SafeEvidenceReplayOpts, String> {
    let mut input = None;
    let mut anchor = None;
    let mut report_out = None;
    let mut index = 0usize;
    while index < raw_args.len() {
        let slot = match raw_args[index].as_str() {
            "--input" => &mut input,
            "--anchor" => &mut anchor,
            "--report-out" => &mut report_out,
            other => return Err(format!("unknown safe-evidence-replay arg '{other}'")),
        };
        index += 1;
        let value = raw_args.get(index).ok_or_else(|| "missing replay argument value".to_string())?;
        if slot.replace(PathBuf::from(value)).is_some() {
            return Err("safe-evidence-replay arguments may be supplied only once".to_string());
        }
        index += 1;
    }
    Ok(SafeEvidenceReplayOpts {
        input: input.ok_or_else(|| "missing required --input".to_string())?,
        anchor: anchor.ok_or_else(|| "missing required --anchor".to_string())?,
        report_out: report_out.ok_or_else(|| "missing required --report-out".to_string())?,
    })
}

fn resolve_within_safe_evidence_root(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let value = relative.to_string_lossy();
    if !is_safe_relative_path(&value) {
        return Err("safe evidence path must be a safe root-relative path".to_string());
    }
    Ok(root.join(relative))
}

fn capture_safe_target_evidence(root: &Path, input_path: &Path) -> Result<SafeTargetEvidence, String> {
    let bytes = fs::read(input_path).map_err(|_| "safe evidence capture input unreadable".to_string())?;
    reject_secret_markers(&bytes).map_err(|error| format!("safe evidence capture {}", error.code))?;
    let input: SafeEvidenceCaptureInput = serde_json::from_slice(&bytes)
        .map_err(|_| "safe evidence capture input malformed".to_string())?;
    if canonical_json_bytes(&input).map_err(|_| "safe evidence capture input malformed".to_string())? != bytes {
        return Err("safe evidence capture input is noncanonical".to_string());
    }
    if input.format != SAFE_EVIDENCE_CAPTURE_INPUT_FORMAT || input.schema_version != 1 {
        return Err("safe evidence capture input has unsupported format or schema".to_string());
    }
    validate_safe_evidence_target(&input.target).map_err(|error| format!("safe evidence capture {}", error.code))?;
    validate_safe_evidence_identity(&input.identity).map_err(|error| format!("safe evidence capture {}", error.code))?;
    if input.observations.is_empty() {
        return Err("safe evidence capture requires observations".to_string());
    }
    let mut observations = Vec::new();
    let mut seen = BTreeSet::new();
    for entry in input.observations {
        if !is_safe_relative_path(&entry.record_path) || entry.id.is_empty() || !seen.insert(entry.id.clone()) {
            return Err("safe evidence capture has unsafe or duplicate observation reference".to_string());
        }
        let bytes = fs::read(root.join(&entry.record_path))
            .map_err(|_| "safe evidence observation unreadable".to_string())?;
        reject_secret_markers(&bytes).map_err(|error| format!("safe evidence capture {}", error.code))?;
        let observation: SafeEvidenceObservation = serde_json::from_slice(&bytes)
            .map_err(|_| "safe evidence observation malformed".to_string())?;
        if canonical_json_bytes(&observation).map_err(|_| "safe evidence observation malformed".to_string())? != bytes
            || observation.id != entry.id
        {
            return Err("safe evidence observation is noncanonical or substituted".to_string());
        }
        validate_safe_evidence_observation(&observation)
            .map_err(|error| format!("safe evidence capture {}", error.code))?;
        observations.push(observation);
    }
    observations.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(SafeTargetEvidence {
        format: SAFE_TARGET_EVIDENCE_FORMAT.to_string(),
        schema_version: 1,
        target: input.target,
        identity: input.identity,
        capture: SafeEvidenceCapture {
            source: "recorded_local_observations".to_string(),
            secret_free: true,
        },
        observations,
        correspondence_status: "captured".to_string(),
        evidence_signature: SafeEvidenceSignature {
            status: "not_authorized".to_string(),
        },
    })
}

fn replay_safe_target_evidence(
    input_path: &Path,
    anchor_path: &Path,
) -> Result<(String, String, SafeEvidenceTarget, Vec<SafeReplayObservationResult>), SafeEvidenceError> {
    let anchor_bytes = fs::read(anchor_path).map_err(|_| SafeEvidenceError::new("M6C001"))?;
    reject_secret_markers(&anchor_bytes).map_err(|_| SafeEvidenceError::new("M6C001"))?;
    let anchor: SafeEvidenceIdentityAnchor =
        serde_json::from_slice(&anchor_bytes).map_err(|_| SafeEvidenceError::new("M6C001"))?;
    if canonical_json_bytes(&anchor)
        .map_err(|_| SafeEvidenceError::new("M6C001"))?
        != anchor_bytes
    {
        return Err(SafeEvidenceError::new("M6C001"));
    }
    validate_safe_evidence_identity_anchor(&anchor)?;
    let bytes = fs::read(input_path).map_err(|_| SafeEvidenceError::new("M6C001"))?;
    reject_secret_markers(&bytes).map_err(|_| SafeEvidenceError::new("M6C001"))?;
    let evidence: SafeTargetEvidence =
        serde_json::from_slice(&bytes).map_err(|_| SafeEvidenceError::new("M6C001"))?;
    if canonical_json_bytes(&evidence)
        .map_err(|_| SafeEvidenceError::new("M6C001"))?
        != bytes
    {
        return Err(SafeEvidenceError::new("M6C001"));
    }
    if evidence.format != SAFE_TARGET_EVIDENCE_FORMAT || evidence.schema_version != 1 {
        return Err(SafeEvidenceError::new("M6C002"));
    }
    validate_safe_evidence_target(&evidence.target)?;
    validate_safe_evidence_identity(&evidence.identity)?;
    if evidence.target != anchor.target || evidence.identity != anchor.identity {
        return Err(SafeEvidenceError::new("M6C003"));
    }
    if evidence.capture.source != "recorded_local_observations"
        || !evidence.capture.secret_free
        || evidence.correspondence_status != "captured"
        || evidence.evidence_signature.status != "not_authorized"
        || evidence.observations.is_empty()
    {
        return Err(SafeEvidenceError::new("M6C001"));
    }
    let mut previous = None;
    let mut action_kinds = BTreeMap::new();
    let mut observation_kinds_by_subject = BTreeMap::<String, BTreeSet<String>>::new();
    let mut last_captured_at_by_subject = BTreeMap::new();
    let mut results = Vec::new();
    for observation in &evidence.observations {
        if previous.as_ref().is_some_and(|last: &String| last >= &observation.id) {
            return Err(SafeEvidenceError::new("M6C001"));
        }
        previous = Some(observation.id.clone());
        validate_safe_evidence_observation(observation)?;
        if last_captured_at_by_subject
            .get(&observation.subject_action_id)
            .is_some_and(|last: &String| last > &observation.captured_at)
        {
            return Err(SafeEvidenceError::new("M6C005"));
        }
        last_captured_at_by_subject.insert(
            observation.subject_action_id.clone(),
            observation.captured_at.clone(),
        );
        observation_kinds_by_subject
            .entry(observation.subject_action_id.clone())
            .or_default()
            .insert(observation.kind.clone());
        if matches!(observation.kind.as_str(), "deployment" | "safe_authorized_revision")
            && action_kinds
                .insert(
                    observation.subject_action_id.clone(),
                    observation.kind.clone(),
                )
                .is_some()
        {
            return Err(SafeEvidenceError::new("M6C007"));
        }
        match observation.status.as_str() {
            "qualified_finality_observation" => results.push(SafeReplayObservationResult {
                id: observation.id.clone(),
                kind: observation.kind.clone(),
                result: "replayed".to_string(),
            }),
            "missing_receipt" | "pending_receipt" | "reverted_receipt" | "replaced_or_conflicting" => {
                return Err(SafeEvidenceError::new("M6C005"))
            }
            "observer_disagreement_or_reorg" => return Err(SafeEvidenceError::new("M6C006")),
            _ => return Err(SafeEvidenceError::new("M6C004")),
        }
    }
    if action_kinds.is_empty() {
        return Err(SafeEvidenceError::new("M6C007"));
    }
    for (subject_action_id, _) in action_kinds {
        let kinds = observation_kinds_by_subject
            .get(&subject_action_id)
            .expect("action observation must have a kind set");
        if !kinds.contains("confirmation") || !kinds.contains("reorganization_observation") {
            return Err(SafeEvidenceError::new("M6C004"));
        }
    }
    Ok((
        safe_policy_sha256_hex(&bytes),
        safe_policy_sha256_hex(&anchor_bytes),
        evidence.target,
        results,
    ))
}

fn validate_safe_evidence_identity_anchor(anchor: &SafeEvidenceIdentityAnchor) -> Result<(), SafeEvidenceError> {
    if anchor.format != SAFE_IDENTITY_ANCHOR_FORMAT || anchor.schema_version != 1 {
        return Err(SafeEvidenceError::new("M6C002"));
    }
    validate_safe_evidence_target(&anchor.target)?;
    validate_safe_evidence_identity(&anchor.identity)?;
    if !is_sha256(&anchor.verified_release_bundle_sha256)
        || !is_sha256(&anchor.verifier_result_sha256)
        || !is_sha256(&anchor.verifier_identity_sha256)
        || anchor.trusted_release_key_id.is_empty()
        || anchor.verified_release_bundle_sha256 != anchor.identity.release_bundle_sha256
    {
        return Err(SafeEvidenceError::new("M6C003"));
    }
    Ok(())
}

fn validate_safe_evidence_target(target: &SafeEvidenceTarget) -> Result<(), SafeEvidenceError> {
    if target.profile != SAFE_EVIDENCE_PROFILE || target.chain_id != "1" {
        Err(SafeEvidenceError::new("M6C002"))
    } else {
        Ok(())
    }
}

fn validate_safe_evidence_identity(identity: &SafeEvidenceIdentity) -> Result<(), SafeEvidenceError> {
    if !is_sha256(&identity.source_profile_proof_sha256)
        || !is_sha256(&identity.state_schema_abi_sha256)
        || !is_sha256(&identity.evm_artifact_sha256)
        || !is_sha256(&identity.release_bundle_sha256)
        || !is_sha256(&identity.policy_sha256)
        || !is_sha256(&identity.safe_identity_sha256)
        || identity.observer_policy_version != "1"
    {
        Err(SafeEvidenceError::new("M6C003"))
    } else {
        Ok(())
    }
}

fn validate_safe_evidence_observation(observation: &SafeEvidenceObservation) -> Result<(), SafeEvidenceError> {
    if observation.format != SAFE_EVIDENCE_OBSERVATION_FORMAT || observation.schema_version != 1 {
        return Err(SafeEvidenceError::new("M6C002"));
    }
    if observation.id.is_empty()
        || observation.subject_action_id.is_empty()
        || !is_utc_timestamp(&observation.captured_at)
        || !is_quantity(&observation.block_number)
        || !is_prefixed_hex(&observation.block_hash, 32)
        || !matches!(observation.kind.as_str(), "deployment" | "safe_authorized_revision" | "expected_unauthorized_invocation" | "confirmation" | "reorganization_observation")
        || !matches!(observation.status.as_str(), "qualified_finality_observation" | "missing_receipt" | "pending_receipt" | "reverted_receipt" | "replaced_or_conflicting" | "observer_disagreement_or_reorg")
    {
        return Err(SafeEvidenceError::new("M6C004"));
    }
    if observation.kind == "expected_unauthorized_invocation" {
        if observation.comparison_result != "expected_no_transition" {
            return Err(SafeEvidenceError::new("M6C007"));
        }
    } else if observation.comparison_result != "match" {
        return Err(SafeEvidenceError::new("M6C004"));
    }
    validate_safe_observer(&observation.primary_observer, "primary")?;
    validate_safe_observer(&observation.independent_observer, "independent")?;
    if observation.primary_observer.observer_id == observation.independent_observer.observer_id
        || observation.primary_observer.client_family == observation.independent_observer.client_family
    {
        return Err(SafeEvidenceError::new("M6C006"));
    }
    Ok(())
}

fn validate_safe_observer(observer: &SafeObserver, expected_class: &str) -> Result<(), SafeEvidenceError> {
    if observer.observer_id.is_empty()
        || observer.observer_class != expected_class
        || observer.client_family.is_empty()
        || observer.client_version.is_empty()
        || observer.exchange_digests.is_empty()
        || observer.exchange_digests.iter().any(|digest| !is_sha256(digest))
    {
        Err(SafeEvidenceError::new("M6C002"))
    } else {
        Ok(())
    }
}

fn atomic_write_safe_evidence_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "safe evidence output has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("create safe evidence output directory: {error}"))?;
    let bytes = serde_json::to_vec(value).map_err(|error| format!("serialize safe evidence output: {error}"))?;
    let temporary = path.with_extension("tmp");
    if temporary.exists() {
        return Err("safe evidence temporary output already exists".to_string());
    }
    fs::write(&temporary, bytes).map_err(|error| format!("write safe evidence temporary output: {error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("commit safe evidence output: {error}"))
}

