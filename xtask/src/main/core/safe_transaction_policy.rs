const SAFE_POLICY_REQUEST_FORMAT: &str = "clg.safe-transaction-request.v1";
const SAFE_POLICY_EVIDENCE_FORMAT: &str = "clg.safe-transaction-policy-evidence.v1";
const SAFE_POLICY_REPORT_FORMAT: &str = "clg.safe-transaction-policy-report.v1";
const SAFE_POLICY_APPROVAL_FORMAT: &str = "clg.safe-owner-approval.v1";
const SAFE_SIGNATURE_BUNDLE_FORMAT: &str = "clg.safe-signature-bundle.v1";
const SAFE_POLICY_PROFILE: &str = "clg.evm-ethereum-mainnet-safe-registry.v1";

#[derive(Clone, Debug, PartialEq, Eq)]
struct SafePolicyValidateOpts {
    request: PathBuf,
    evidence: PathBuf,
    artifact_root: PathBuf,
    report: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafePolicyTarget {
    profile: String,
    chain_id: String,
    submission_endpoint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafePolicyRelease {
    bundle_path: String,
    bundle_sha256: String,
    verifier_result_path: String,
    verifier_result_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafePolicyBinding {
    operator_policy_path: String,
    operator_policy_sha256: String,
    safe_owner_amendment_path: String,
    safe_owner_amendment_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeFee {
    kind: String,
    gas_price_wei: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeIdentity {
    safe_address: String,
    safe_version: String,
    proxy_runtime_code_sha256: String,
    singleton_address: String,
    singleton_runtime_code_sha256: String,
    owners: Vec<String>,
    threshold: String,
    safe_nonce: String,
    modules: Vec<String>,
    guard: Option<String>,
    fallback_handler: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RegistryConstructor {
    owner_safe_address: String,
    initial_value_u64: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RegistryDeployAction {
    deployment_sender: String,
    deployment_nonce: String,
    value_wei: String,
    gas_limit: String,
    fee: SafeFee,
    creation_calldata: String,
    creation_calldata_sha256: String,
    constructor: RegistryConstructor,
    safe_identity: SafeIdentity,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeTransaction {
    safe_nonce: String,
    to: String,
    value_wei: String,
    operation: String,
    set_value_next_u64: String,
    set_value_calldata: String,
    set_value_calldata_sha256: String,
    safe_tx_digest: String,
    signature_bundle_path: String,
    signature_bundle_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeRegistryRevisionAction {
    executor_sender: String,
    executor_nonce: String,
    outer_value_wei: String,
    outer_gas_limit: String,
    outer_fee: SafeFee,
    safe_identity: SafeIdentity,
    safe_transaction: SafeTransaction,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind")]
enum SafePolicyAction {
    #[serde(rename = "registry_deploy")]
    RegistryDeploy(RegistryDeployAction),
    #[serde(rename = "safe_registry_set_value")]
    SafeRegistrySetValue(SafeRegistryRevisionAction),
}

impl SafePolicyAction {
    fn kind(&self) -> &'static str {
        match self {
            Self::RegistryDeploy(_) => "registry_deploy",
            Self::SafeRegistrySetValue(_) => "safe_registry_set_value",
        }
    }

    fn safe_identity(&self) -> &SafeIdentity {
        match self {
            Self::RegistryDeploy(action) => &action.safe_identity,
            Self::SafeRegistrySetValue(action) => &action.safe_identity,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafePolicyRequest {
    format: String,
    schema_version: u32,
    request_id: String,
    environment: String,
    target: SafePolicyTarget,
    created_at: String,
    expires_at: String,
    release: SafePolicyRelease,
    policy: SafePolicyBinding,
    action: SafePolicyAction,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeOwnerApproval {
    format: String,
    schema_version: u32,
    transaction_request_sha256: String,
    environment: String,
    target_profile: String,
    chain_id: String,
    release_bundle_sha256: String,
    operator_policy_sha256: String,
    action_kind: String,
    owner_address: String,
    approved_at: String,
    expires_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeEvidenceApproval {
    owner_address: String,
    approval_record_path: String,
    approval_record_sha256: String,
    approved_at: String,
    expires_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafePolicyEvidence {
    format: String,
    schema_version: u32,
    transaction_request_sha256: String,
    environment: String,
    target_profile: String,
    chain_id: String,
    release_bundle_sha256: String,
    operator_policy_sha256: String,
    approval_kind: String,
    required_owner_count: String,
    frozen_owner_set_sha256: String,
    approvals: Vec<SafeEvidenceApproval>,
    review_disposition: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafeSignatureBundle {
    format: String,
    schema_version: u32,
    safe_tx_digest: String,
    signer_addresses: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SafePolicyValidationReport {
    format: String,
    schema_version: u32,
    status: String,
    request_sha256: Option<String>,
    evidence_sha256: Option<String>,
    action_kind: Option<String>,
    rejection_code: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SafePolicyError {
    code: &'static str,
}

impl SafePolicyError {
    fn new(code: &'static str) -> Self {
        Self { code }
    }
}

fn run_safe_policy_validate(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_safe_policy_validate_args(raw_args)?;
    let request_path = resolve_root_path(root, &opts.request)?;
    let evidence_path = resolve_root_path(root, &opts.evidence)?;
    let artifact_root = resolve_root_path(root, &opts.artifact_root)?;
    let report_path = resolve_root_path(root, &opts.report)?;
    if report_path.exists() {
        return Err(format!(
            "safe-policy-validate refuses to overwrite existing report '{}'",
            opts.report.display()
        ));
    }
    let outcome = validate_safe_policy_files(
        request_path.as_path(),
        evidence_path.as_path(),
        artifact_root.as_path(),
    );
    let report = match &outcome {
        Ok((request_digest, evidence_digest, action_kind)) => SafePolicyValidationReport {
            format: SAFE_POLICY_REPORT_FORMAT.to_string(),
            schema_version: 1,
            status: "accepted".to_string(),
            request_sha256: Some(request_digest.clone()),
            evidence_sha256: Some(evidence_digest.clone()),
            action_kind: Some((*action_kind).to_string()),
            rejection_code: None,
        },
        Err(error) => SafePolicyValidationReport {
            format: SAFE_POLICY_REPORT_FORMAT.to_string(),
            schema_version: 1,
            status: "rejected".to_string(),
            request_sha256: None,
            evidence_sha256: None,
            action_kind: None,
            rejection_code: Some(error.code.to_string()),
        },
    };
    write_safe_policy_report(report_path.as_path(), &report)?;
    match outcome {
        Ok(_) => Ok(()),
        Err(error) => Err(format!(
            "safe-policy-validate rejected record with {}",
            error.code
        )),
    }
}

fn parse_safe_policy_validate_args(raw_args: Vec<String>) -> Result<SafePolicyValidateOpts, String> {
    let mut request = None;
    let mut evidence = None;
    let mut artifact_root = None;
    let mut report = None;
    let mut index = 0usize;
    while index < raw_args.len() {
        let slot = match raw_args[index].as_str() {
            "--request" => &mut request,
            "--evidence" => &mut evidence,
            "--artifact-root" => &mut artifact_root,
            "--report" => &mut report,
            other => return Err(format!("unknown safe-policy-validate arg '{other}'")),
        };
        index += 1;
        let value = raw_args
            .get(index)
            .ok_or_else(|| "missing value for safe-policy-validate argument".to_string())?;
        if slot.replace(PathBuf::from(value)).is_some() {
            return Err("safe-policy-validate arguments may be supplied only once".to_string());
        }
        index += 1;
    }
    Ok(SafePolicyValidateOpts {
        request: request.ok_or_else(|| "missing required --request".to_string())?,
        evidence: evidence.ok_or_else(|| "missing required --evidence".to_string())?,
        artifact_root: artifact_root.ok_or_else(|| "missing required --artifact-root".to_string())?,
        report: report.ok_or_else(|| "missing required --report".to_string())?,
    })
}

fn resolve_root_path(root: &Path, value: &Path) -> Result<PathBuf, String> {
    if value.as_os_str().is_empty() {
        return Err("safe-policy-validate path must not be empty".to_string());
    }
    Ok(if value.is_absolute() { value.to_path_buf() } else { root.join(value) })
}

fn write_safe_policy_report(path: &Path, report: &SafePolicyValidationReport) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "safe-policy-validate report path has no parent".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("create report directory '{}': {error}", parent.display()))?;
    let bytes = pretty_json_bytes(report)?;
    fs::write(path, bytes).map_err(|error| format!("write report '{}': {error}", path.display()))
}

fn validate_safe_policy_files(
    request_path: &Path,
    evidence_path: &Path,
    artifact_root: &Path,
) -> Result<(String, String, &'static str), SafePolicyError> {
    let request_bytes = fs::read(request_path).map_err(|_| SafePolicyError::new("M6B001"))?;
    let evidence_bytes = fs::read(evidence_path).map_err(|_| SafePolicyError::new("M6B001"))?;
    reject_secret_markers(&request_bytes)?;
    reject_secret_markers(&evidence_bytes)?;
    let request: SafePolicyRequest =
        serde_json::from_slice(&request_bytes).map_err(|_| SafePolicyError::new("M6B001"))?;
    let evidence: SafePolicyEvidence =
        serde_json::from_slice(&evidence_bytes).map_err(|_| SafePolicyError::new("M6B001"))?;
    if canonical_json_bytes(&request)? != request_bytes || canonical_json_bytes(&evidence)? != evidence_bytes {
        return Err(SafePolicyError::new("M6B001"));
    }
    let request_digest = safe_policy_sha256_hex(&request_bytes);
    let evidence_digest = safe_policy_sha256_hex(&evidence_bytes);
    validate_safe_policy_request(&request, artifact_root)?;
    validate_safe_policy_evidence(&request, &evidence, request_digest.as_str(), artifact_root)?;
    Ok((request_digest, evidence_digest, request.action.kind()))
}

fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, SafePolicyError> {
    serde_json::to_vec(value).map_err(|_| SafePolicyError::new("M6B001"))
}

fn safe_policy_sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn validate_safe_policy_request(request: &SafePolicyRequest, artifact_root: &Path) -> Result<(), SafePolicyError> {
    if request.format != SAFE_POLICY_REQUEST_FORMAT || request.schema_version != 1 {
        return Err(SafePolicyError::new("M6B001"));
    }
    if !is_request_id(&request.request_id) || request.environment != "production" {
        return Err(SafePolicyError::new("M6B002"));
    }
    if request.target.profile != SAFE_POLICY_PROFILE || request.target.chain_id != "1" {
        return Err(SafePolicyError::new("M6B002"));
    }
    if !is_submission_endpoint(&request.target.submission_endpoint) {
        return Err(SafePolicyError::new("M6B003"));
    }
    if !is_utc_timestamp(&request.created_at) || !is_utc_timestamp(&request.expires_at) || request.expires_at <= request.created_at {
        return Err(SafePolicyError::new("M6B004"));
    }
    verify_referenced_digest(artifact_root, &request.release.bundle_path, &request.release.bundle_sha256)?;
    verify_referenced_digest(artifact_root, &request.release.verifier_result_path, &request.release.verifier_result_sha256)?;
    verify_referenced_digest(artifact_root, &request.policy.operator_policy_path, &request.policy.operator_policy_sha256)?;
    verify_referenced_digest(artifact_root, &request.policy.safe_owner_amendment_path, &request.policy.safe_owner_amendment_sha256)?;
    match &request.action {
        SafePolicyAction::RegistryDeploy(action) => validate_registry_deploy(action),
        SafePolicyAction::SafeRegistrySetValue(action) => validate_safe_registry_revision(action, artifact_root),
    }
}

fn validate_safe_policy_evidence(
    request: &SafePolicyRequest,
    evidence: &SafePolicyEvidence,
    request_digest: &str,
    artifact_root: &Path,
) -> Result<(), SafePolicyError> {
    if evidence.format != SAFE_POLICY_EVIDENCE_FORMAT || evidence.schema_version != 1 {
        return Err(SafePolicyError::new("M6B001"));
    }
    if evidence.transaction_request_sha256 != request_digest
        || evidence.environment != request.environment
        || evidence.target_profile != request.target.profile
        || evidence.chain_id != request.target.chain_id
        || evidence.release_bundle_sha256 != request.release.bundle_sha256
        || evidence.operator_policy_sha256 != request.policy.operator_policy_sha256
    {
        return Err(SafePolicyError::new("M6B002"));
    }
    let approval_kind = match request.action {
        SafePolicyAction::RegistryDeploy(_) => "safe_owner_deployment_consent",
        SafePolicyAction::SafeRegistrySetValue(_) => "safe_exec_transaction",
    };
    if evidence.approval_kind != approval_kind || evidence.required_owner_count != "2" || evidence.review_disposition != "approved" {
        return Err(SafePolicyError::new("M6B004"));
    }
    let identity = request.action.safe_identity();
    let owner_set_digest = safe_policy_sha256_hex(&serde_json::to_vec(&identity.owners).map_err(|_| SafePolicyError::new("M6B001"))?);
    if evidence.frozen_owner_set_sha256 != owner_set_digest || evidence.approvals.len() < 2 {
        return Err(SafePolicyError::new("M6B005"));
    }
    let mut seen = BTreeSet::new();
    let mut previous: Option<&String> = None;
    for approval in &evidence.approvals {
        if !seen.insert(&approval.owner_address)
            || previous.is_some_and(|last| last >= &approval.owner_address)
            || !identity.owners.contains(&approval.owner_address)
            || !is_utc_timestamp(&approval.approved_at)
            || !is_utc_timestamp(&approval.expires_at)
            || approval.expires_at > request.expires_at
        {
            return Err(SafePolicyError::new("M6B004"));
        }
        previous = Some(&approval.owner_address);
        let bytes = read_artifact_path(artifact_root, &approval.approval_record_path)?;
        if safe_policy_sha256_hex(&bytes) != approval.approval_record_sha256 {
            return Err(SafePolicyError::new("M6B004"));
        }
        reject_secret_markers(&bytes)?;
        let record: SafeOwnerApproval = serde_json::from_slice(&bytes).map_err(|_| SafePolicyError::new("M6B004"))?;
        if canonical_json_bytes(&record)? != bytes
            || record.format != SAFE_POLICY_APPROVAL_FORMAT
            || record.schema_version != 1
            || record.transaction_request_sha256 != request_digest
            || record.environment != request.environment
            || record.target_profile != request.target.profile
            || record.chain_id != request.target.chain_id
            || record.release_bundle_sha256 != request.release.bundle_sha256
            || record.operator_policy_sha256 != request.policy.operator_policy_sha256
            || record.action_kind != request.action.kind()
            || record.owner_address != approval.owner_address
            || record.approved_at != approval.approved_at
            || record.expires_at != approval.expires_at
        {
            return Err(SafePolicyError::new("M6B004"));
        }
    }
    Ok(())
}

fn validate_registry_deploy(action: &RegistryDeployAction) -> Result<(), SafePolicyError> {
    validate_safe_identity(&action.safe_identity)?;
    if !is_address(&action.deployment_sender)
        || action.safe_identity.owners.contains(&action.deployment_sender)
        || action.value_wei != "0"
        || !is_nonzero_quantity(&action.gas_limit)
        || !valid_legacy_fee(&action.fee)
        || !is_quantity(&action.deployment_nonce)
        || !is_quantity(&action.constructor.initial_value_u64)
        || action.constructor.owner_safe_address != action.safe_identity.safe_address
    {
        return Err(SafePolicyError::new("M6B006"));
    }
    let bytes = decode_hex(&action.creation_calldata).ok_or_else(|| SafePolicyError::new("M6B006"))?;
    if bytes.len() < 64 || safe_policy_sha256_hex(&bytes) != action.creation_calldata_sha256 || !constructor_tail_matches(&bytes, &action.constructor) {
        return Err(SafePolicyError::new("M6B006"));
    }
    Ok(())
}

fn validate_safe_registry_revision(action: &SafeRegistryRevisionAction, artifact_root: &Path) -> Result<(), SafePolicyError> {
    validate_safe_identity(&action.safe_identity)?;
    if !is_address(&action.executor_sender)
        || action.safe_identity.owners.contains(&action.executor_sender)
        || action.outer_value_wei != "0"
        || !is_nonzero_quantity(&action.outer_gas_limit)
        || !valid_legacy_fee(&action.outer_fee)
        || !is_quantity(&action.executor_nonce)
    {
        return Err(SafePolicyError::new("M6B007"));
    }
    let transaction = &action.safe_transaction;
    if transaction.safe_nonce != action.safe_identity.safe_nonce
        || !is_address(&transaction.to)
        || transaction.value_wei != "0"
        || transaction.operation != "CALL"
        || !is_quantity(&transaction.set_value_next_u64)
        || !is_prefixed_hex(&transaction.safe_tx_digest, 32)
    {
        return Err(SafePolicyError::new("M6B007"));
    }
    let calldata = decode_hex(&transaction.set_value_calldata).ok_or_else(|| SafePolicyError::new("M6B007"))?;
    if safe_policy_sha256_hex(&calldata) != transaction.set_value_calldata_sha256 || !is_canonical_set_value_calldata(&calldata, &transaction.set_value_next_u64) {
        return Err(SafePolicyError::new("M6B007"));
    }
    let bundle_bytes = read_artifact_path(artifact_root, &transaction.signature_bundle_path)?;
    if safe_policy_sha256_hex(&bundle_bytes) != transaction.signature_bundle_sha256 {
        return Err(SafePolicyError::new("M6B004"));
    }
    reject_secret_markers(&bundle_bytes)?;
    let bundle: SafeSignatureBundle = serde_json::from_slice(&bundle_bytes).map_err(|_| SafePolicyError::new("M6B007"))?;
    if canonical_json_bytes(&bundle)? != bundle_bytes
        || bundle.format != SAFE_SIGNATURE_BUNDLE_FORMAT
        || bundle.schema_version != 1
        || bundle.safe_tx_digest != transaction.safe_tx_digest
        || bundle.signer_addresses.len() < 2
        || !is_sorted_unique_addresses(&bundle.signer_addresses)
        || bundle.signer_addresses.iter().any(|address| !action.safe_identity.owners.contains(address))
    {
        return Err(SafePolicyError::new("M6B007"));
    }
    Ok(())
}

fn validate_safe_identity(identity: &SafeIdentity) -> Result<(), SafePolicyError> {
    if !is_address(&identity.safe_address)
        || identity.safe_version != "1.4.1"
        || !is_sha256(&identity.proxy_runtime_code_sha256)
        || !is_address(&identity.singleton_address)
        || !is_sha256(&identity.singleton_runtime_code_sha256)
        || identity.owners.len() != 3
        || !is_sorted_unique_addresses(&identity.owners)
        || identity.threshold != "2"
        || !is_quantity(&identity.safe_nonce)
        || !identity.modules.is_empty()
        || identity.guard.is_some()
        || identity.fallback_handler.is_some()
    {
        return Err(SafePolicyError::new("M6B005"));
    }
    Ok(())
}

fn verify_referenced_digest(artifact_root: &Path, relative_path: &str, expected_digest: &str) -> Result<(), SafePolicyError> {
    if !is_sha256(expected_digest) {
        return Err(SafePolicyError::new("M6B001"));
    }
    let bytes = read_artifact_path(artifact_root, relative_path)?;
    if safe_policy_sha256_hex(&bytes) != expected_digest {
        return Err(SafePolicyError::new("M6B004"));
    }
    reject_secret_markers(&bytes)
}

fn read_artifact_path(root: &Path, relative_path: &str) -> Result<Vec<u8>, SafePolicyError> {
    if !is_safe_relative_path(relative_path) {
        return Err(SafePolicyError::new("M6B003"));
    }
    fs::read(root.join(relative_path)).map_err(|_| SafePolicyError::new("M6B004"))
}

fn reject_secret_markers(bytes: &[u8]) -> Result<(), SafePolicyError> {
    let text = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    if ["private_key", "seed_phrase", "keystore", "rpc_credential", "begin private"].iter().any(|marker| text.contains(marker)) {
        Err(SafePolicyError::new("M6B003"))
    } else {
        Ok(())
    }
}

fn valid_legacy_fee(fee: &SafeFee) -> bool {
    fee.kind == "legacy-eip155-gas-price" && is_nonzero_quantity(&fee.gas_price_wei)
}

fn is_request_id(value: &str) -> bool {
    value.len() == 71 && value.starts_with("clgreq-") && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_quantity(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) && (value == "0" || !value.starts_with('0'))
}

fn is_nonzero_quantity(value: &str) -> bool {
    is_quantity(value) && value != "0"
}

fn is_address(value: &str) -> bool {
    is_prefixed_hex(value, 20)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_prefixed_hex(value: &str, bytes: usize) -> bool {
    value.len() == 2 + bytes * 2 && value.starts_with("0x") && value[2..].bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_sorted_unique_addresses(values: &[String]) -> bool {
    values.iter().all(|value| is_address(value)) && values.windows(2).all(|window| window[0] < window[1])
}

fn is_safe_relative_path(value: &str) -> bool {
    !value.is_empty() && !value.contains('\\') && !value.starts_with('/') && !value.contains(':')
        && value.split('/').all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn is_submission_endpoint(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://") else { return false; };
    !rest.is_empty() && !rest.contains('@') && !rest.contains('?') && !rest.contains('#')
}

fn is_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 20
        && bytes[4] == b'-' && bytes[7] == b'-' && bytes[10] == b'T'
        && bytes[13] == b':' && bytes[16] == b':' && bytes[19] == b'Z'
        && bytes.iter().enumerate().filter(|(index, _)| ![4, 7, 10, 13, 16, 19].contains(index)).all(|(_, byte)| byte.is_ascii_digit())
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if !value.starts_with("0x") || value.len() % 2 != 0 || !value[2..].bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()) {
        return None;
    }
    hex::decode(&value[2..]).ok()
}

fn constructor_tail_matches(bytes: &[u8], constructor: &RegistryConstructor) -> bool {
    let owner = match decode_hex(&constructor.owner_safe_address) { Some(value) => value, None => return false };
    let initial = match constructor.initial_value_u64.parse::<u64>() { Ok(value) => value, Err(_) => return false };
    let tail = &bytes[bytes.len() - 64..];
    tail[..12].iter().all(|byte| *byte == 0) && tail[12..32] == owner && tail[32..56].iter().all(|byte| *byte == 0) && tail[56..64] == initial.to_be_bytes()
}

fn is_canonical_set_value_calldata(bytes: &[u8], next: &str) -> bool {
    let next = match next.parse::<u64>() { Ok(value) => value, Err(_) => return false };
    bytes.len() == 36 && bytes[..4] == [0x13, 0x76, 0x58, 0x38]
        && bytes[4..28].iter().all(|byte| *byte == 0) && bytes[28..36] == next.to_be_bytes()
}

