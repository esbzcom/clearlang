//! Explicit, narrow EVM-compatible target adapter.
//!
//! The adapter never discovers an endpoint, account, or network.  `call` is operational for the
//! first static scalar ABI profile. Deploy and invoke submit only explicit, locally signed
//! EIP-155 transactions and write receipts only after confirmed on-chain success.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::commands::helpers::{canonical_json_bytes, sha256_hex};

mod transaction;

const TARGET_PROFILE: &str = "clg.evm-sepolia-pilot.v1";
const TARGET_CHAIN_ID: u64 = 11_155_111;
const WIRE_ABI_FORMAT: &str = "clg.evm-wire-abi.v1";
const RECEIPT_FORMAT: &str = "clg.target-receipt.v1";
const PILOT_INPUT_FORMAT: &str = "clg.pilot-exercise-input.v1";
const PILOT_PREFLIGHT_FORMAT: &str = "clg.pilot-exercise-preflight.v1";
const PILOT_CAPTURE_INPUT_FORMAT: &str = "clg.pilot-evidence-capture-input.v1";
const PILOT_EVIDENCE_FORMAT: &str = "clg.pilot-evidence.v1";
const PILOT_RPC_OBSERVATIONS_FORMAT: &str = "clg.pilot-rpc-observations.v1";
const PILOT_VERIFIER_RESULT_FORMAT: &str = "clg.pilot-release-verifier-result.v1";
const RECEIPT_POLL_ATTEMPTS: usize = 10;
const RECEIPT_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub struct DeployArgs {
    pub target_profile: String,
    pub rpc_url: String,
    pub chain_id: u64,
    pub sender: String,
    pub nonce: u64,
    pub signing_key: PathBuf,
    pub value: u64,
    pub gas_limit: u64,
    pub gas_price: u64,
    pub artifact: PathBuf,
    pub abi: PathBuf,
    pub contract_state_schema: PathBuf,
    pub proof_artifact: PathBuf,
    pub signed_bundle: PathBuf,
    pub args: Option<PathBuf>,
    pub receipt_out: PathBuf,
}

#[derive(Debug)]
pub struct CallArgs {
    pub target_profile: String,
    pub rpc_url: String,
    pub chain_id: u64,
    pub artifact: PathBuf,
    pub abi: PathBuf,
    pub contract_state_schema: PathBuf,
    pub proof_artifact: PathBuf,
    pub signed_bundle: PathBuf,
    pub contract_address: String,
    pub function: String,
    pub args: PathBuf,
    pub receipt_out: PathBuf,
}

#[derive(Debug)]
pub struct InvokeArgs {
    pub target_profile: String,
    pub rpc_url: String,
    pub chain_id: u64,
    pub artifact: PathBuf,
    pub abi: PathBuf,
    pub contract_state_schema: PathBuf,
    pub proof_artifact: PathBuf,
    pub signed_bundle: PathBuf,
    pub contract_address: String,
    pub function: String,
    pub args: PathBuf,
    pub sender: String,
    pub nonce: u64,
    pub signing_key: PathBuf,
    pub value: u64,
    pub gas_limit: u64,
    pub gas_price: u64,
    pub receipt_out: PathBuf,
}

/// Arguments for the offline pilot operator review step.
///
/// This intentionally only reads `input` and writes `report_out`. In particular, it neither
/// dereferences the signer reference nor contacts the declared endpoint.
#[derive(Debug)]
pub struct PilotPreflightArgs {
    pub input: PathBuf,
    pub report_out: PathBuf,
}

/// Arguments for offline pilot-evidence capture.
#[derive(Debug)]
pub struct CapturePilotEvidenceArgs {
    pub root: PathBuf,
    pub input: String,
    pub out: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PilotExerciseInput {
    format: String,
    schema_version: u64,
    target_profile: String,
    rpc_url: String,
    chain_id: u64,
    signer_reference: String,
    sender: String,
    deploy: PilotDeployInput,
    invocation: PilotInvocationInput,
    release_bundle: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PilotDeployInput {
    nonce: u64,
    value: u64,
    gas_limit: u64,
    fee: PilotFeeInput,
    constructor_arguments: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PilotInvocationInput {
    contract_address: String,
    function_selector: String,
    arguments: String,
    nonce: u64,
    value: u64,
    gas_limit: u64,
    fee: PilotFeeInput,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PilotFeeInput {
    kind: String,
    gas_price: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PilotEvidenceCaptureInput {
    format: String,
    schema_version: u64,
    submission_intent: String,
    release_bundle: String,
    verifier_result: String,
    observations: Vec<PilotEvidenceObservationInput>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PilotEvidenceObservationInput {
    kind: String,
    caller: String,
    rpc_observations: String,
    receipt: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PilotRpcObservations {
    format: String,
    schema_version: u64,
    target_profile: String,
    chain_id: u64,
    exchanges: Vec<PilotRpcExchange>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PilotRpcExchange {
    request: PilotRpcRequest,
    response: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PilotRpcRequest {
    method: String,
    params: Value,
}

/// Validate an operator-supplied testnet plan and emit the exact values requiring review.
///
/// This is deliberately a non-submitting command. A later explicit operator action must still
/// use the reviewed values when it signs and submits the deploy or invocation.
pub fn pilot_preflight(args: PilotPreflightArgs) -> Result<()> {
    let bytes = fs::read(&args.input)
        .with_context(|| format!("read pilot exercise input `{}`", args.input.display()))?;
    let input: PilotExerciseInput = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse pilot exercise input `{}`", args.input.display()))?;

    if input.format != PILOT_INPUT_FORMAT {
        bail!(
            "unsupported pilot exercise input format `{}`; expected `{PILOT_INPUT_FORMAT}`",
            input.format
        );
    }
    if input.schema_version != 1 {
        bail!(
            "unsupported pilot exercise input schema version `{}`; expected `1`",
            input.schema_version
        );
    }
    let endpoint = preflight(&input.target_profile, &input.rpc_url, input.chain_id)?;
    let signer_reference = safe_relative_path(&input.signer_reference, "signer_reference")?;
    let release_bundle = safe_relative_path(&input.release_bundle, "release_bundle")?;
    let constructor_arguments = safe_relative_path(
        &input.deploy.constructor_arguments,
        "deploy.constructor_arguments",
    )?;
    let invocation_arguments =
        safe_relative_path(&input.invocation.arguments, "invocation.arguments")?;
    let sender = canonical_address(&input.sender).context("invalid pilot sender")?;
    let contract_address = canonical_address(&input.invocation.contract_address)
        .context("invalid pilot invocation contract_address")?;
    let function_selector = canonical_selector(&input.invocation.function_selector)?;
    validate_pilot_transaction(input.deploy.gas_limit, &input.deploy.fee, "deploy")?;
    validate_pilot_transaction(
        input.invocation.gas_limit,
        &input.invocation.fee,
        "invocation",
    )?;

    let report = json!({
        "format": PILOT_PREFLIGHT_FORMAT,
        "schema_version": 1,
        "network": {
            "chain_id": input.chain_id,
            "rpc_url": endpoint,
            "target_profile": TARGET_PROFILE,
        },
        "signer": {
            "reference": signer_reference,
            "sender": sender,
        },
        "release_bundle": release_bundle,
        "deploy": {
            "constructor_arguments": constructor_arguments,
            "transaction": pilot_transaction_value(
                input.deploy.nonce,
                input.deploy.value,
                input.deploy.gas_limit,
                &input.deploy.fee,
            ),
        },
        "invocation": {
            "arguments": invocation_arguments,
            "contract_address": contract_address,
            "function_selector": function_selector,
            "transaction": pilot_transaction_value(
                input.invocation.nonce,
                input.invocation.value,
                input.invocation.gas_limit,
                &input.invocation.fee,
            ),
        },
        "safety": {
            "network_contacted": false,
            "signer_material_accessed": false,
            "transaction_signed": false,
            "transaction_submitted": false,
        },
    });
    let report_bytes = canonical_json_bytes(&report);
    fs::write(&args.report_out, &report_bytes).with_context(|| {
        format!(
            "write pilot preflight report `{}`",
            args.report_out.display()
        )
    })?;
    println!(
        "{}",
        String::from_utf8(report_bytes).expect("canonical JSON is valid UTF-8")
    );
    Ok(())
}

fn pilot_transaction_value(nonce: u64, value: u64, gas_limit: u64, fee: &PilotFeeInput) -> Value {
    json!({
        "fee": { "gas_price": fee.gas_price, "kind": fee.kind },
        "gas_limit": gas_limit,
        "nonce": nonce,
        "value": value,
    })
}

fn validate_pilot_transaction(gas_limit: u64, fee: &PilotFeeInput, label: &str) -> Result<()> {
    if gas_limit == 0 {
        bail!("pilot {label} gas_limit must be greater than zero")
    }
    if fee.kind != "legacy-eip155-gas-price" {
        bail!(
            "unsupported pilot {label} fee kind `{}`; expected `legacy-eip155-gas-price`",
            fee.kind
        )
    }
    if fee.gas_price == 0 {
        bail!("pilot {label} fee gas_price must be greater than zero")
    }
    Ok(())
}

fn canonical_selector(selector: &str) -> Result<String> {
    let bytes = valid_hex_bytes(selector).context("pilot invocation function_selector")?;
    if bytes.len() != 4 {
        bail!("pilot invocation function_selector must contain exactly four bytes")
    }
    Ok(format!("0x{}", hex::encode(bytes)))
}

fn safe_relative_path(value: &str, field: &str) -> Result<String> {
    if value.is_empty()
        || value.contains('\\')
        || value.contains(':')
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        bail!("pilot {field} must be a non-empty canonical relative path")
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        })
    {
        bail!("pilot {field} must be a non-empty canonical relative path")
    }
    Ok(value.to_string())
}

/// Capture ordered, offline testnet observations into a path-free evidence record.
///
/// Capture deliberately has no signing flags and never calls `rpc`: it only validates locally
/// recorded files under `root`. Existing release signing policy does not authorize a signature
/// over this operator evidence, so the output is intentionally unsigned.
pub fn capture_pilot_evidence(args: CapturePilotEvidenceArgs) -> Result<()> {
    let root = fs::canonicalize(&args.root)
        .with_context(|| format!("resolve pilot evidence root `{}`", args.root.display()))?;
    if !fs::metadata(&root)
        .with_context(|| format!("inspect pilot evidence root `{}`", root.display()))?
        .is_dir()
    {
        bail!("pilot evidence root must be a directory")
    }
    let (capture_bytes, capture_value) =
        load_pilot_evidence_file(&root, &args.input, "capture input", false)?;
    let capture: PilotEvidenceCaptureInput =
        serde_json::from_value(capture_value).context("parse pilot evidence capture input")?;
    let _ = capture_bytes;
    if capture.format != PILOT_CAPTURE_INPUT_FORMAT || capture.schema_version != 1 {
        bail!("pilot evidence capture input has an unsupported format or schema version")
    }
    let expected_kinds = ["deploy", "authorized-invoke", "unauthorized-invoke"];
    if capture.observations.len() != expected_kinds.len()
        || capture
            .observations
            .iter()
            .map(|observation| observation.kind.as_str())
            .collect::<Vec<_>>()
            != expected_kinds
    {
        bail!("pilot evidence observations must be ordered as deploy, authorized-invoke, unauthorized-invoke")
    }

    let (intent_bytes, intent) =
        load_pilot_evidence_file(&root, &capture.submission_intent, "submission intent", true)?;
    let intent_context = validate_pilot_submission_intent(&intent)?;
    let (bundle_bytes, bundle) =
        load_pilot_evidence_file(&root, &capture.release_bundle, "release bundle", true)?;
    let bundle_context = validate_pilot_release_bundle(&bundle)?;
    if bundle_context.target_profile != intent_context.target_profile {
        bail!("pilot release bundle target profile does not match submission intent")
    }
    let bundle_digest = sha256_digest(&bundle_bytes);
    let (_verifier_bytes, verifier_result) = load_pilot_evidence_file(
        &root,
        &capture.verifier_result,
        "release verifier result",
        true,
    )?;
    validate_pilot_verifier_result(&verifier_result, &bundle_digest)?;

    let mut observations = Vec::with_capacity(capture.observations.len());
    for observation in &capture.observations {
        observations.push(capture_pilot_observation(
            &root,
            observation,
            &intent_context,
            &bundle_context,
        )?);
    }
    let record = json!({
        "format": PILOT_EVIDENCE_FORMAT,
        "schema_version": 1,
        "submission_intent": {
            "chain_id": intent_context.chain_id,
            "digest": sha256_digest(&intent_bytes),
            "sender": intent_context.sender,
            "target_profile": intent_context.target_profile,
        },
        "release_bundle": {
            "digest": bundle_digest,
            "signature_digest": bundle_context.signature_digest,
            "target_profile": bundle_context.target_profile,
        },
        "verifier_result": {
            "release_bundle_digest": bundle_digest,
            "status": "verified",
        },
        "observations": observations,
        "evidence_signature": {
            "status": "not_authorized",
            "reason": "existing release signing policy does not authorize pilot-evidence signing",
        },
    });
    let bytes = canonical_json_bytes(&record);
    fs::write(&args.out, &bytes)
        .with_context(|| format!("write pilot evidence `{}`", args.out.display()))?;
    println!(
        "{}",
        String::from_utf8(bytes).expect("canonical JSON is valid UTF-8")
    );
    Ok(())
}

#[derive(Debug)]
struct PilotIntentContext {
    target_profile: String,
    chain_id: u64,
    sender: String,
    deploy: Value,
    invocation: Value,
}

#[derive(Debug)]
struct PilotBundleContext {
    target_profile: String,
    signature_digest: String,
}

fn load_pilot_evidence_file(
    root: &Path,
    relative: &str,
    description: &str,
    require_canonical_json: bool,
) -> Result<(Vec<u8>, Value)> {
    let relative = safe_relative_path(relative, description)?;
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path)
        .with_context(|| format!("inspect pilot {description} `{}`", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("pilot {description} must be a regular root-relative file")
    }
    let resolved = fs::canonicalize(&path)
        .with_context(|| format!("resolve pilot {description} `{}`", path.display()))?;
    if !resolved.starts_with(root) {
        bail!("pilot {description} resolves outside the evidence root")
    }
    let bytes = fs::read(&resolved)
        .with_context(|| format!("read pilot {description} `{}`", resolved.display()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse pilot {description} `{}`", resolved.display()))?;
    if require_canonical_json && bytes != canonical_json_bytes(&value) {
        bail!("pilot {description} must use canonical JSON encoding")
    }
    Ok((bytes, value))
}

fn validate_pilot_submission_intent(value: &Value) -> Result<PilotIntentContext> {
    if value["format"].as_str() != Some(PILOT_PREFLIGHT_FORMAT)
        || value["schema_version"].as_u64() != Some(1)
        || value["safety"]["network_contacted"].as_bool() != Some(false)
        || value["safety"]["signer_material_accessed"].as_bool() != Some(false)
        || value["safety"]["transaction_signed"].as_bool() != Some(false)
        || value["safety"]["transaction_submitted"].as_bool() != Some(false)
    {
        bail!("pilot submission intent is not a valid offline preflight record")
    }
    let target_profile = value["network"]["target_profile"]
        .as_str()
        .filter(|profile| *profile == TARGET_PROFILE)
        .ok_or_else(|| anyhow!("pilot submission intent has an unsupported target profile"))?
        .to_string();
    let chain_id = value["network"]["chain_id"]
        .as_u64()
        .filter(|chain_id| *chain_id == TARGET_CHAIN_ID)
        .ok_or_else(|| anyhow!("pilot submission intent has an unsupported chain ID"))?;
    let sender = value["signer"]["sender"]
        .as_str()
        .ok_or_else(|| anyhow!("pilot submission intent is missing its sender"))
        .and_then(canonical_address)?;
    for name in ["deploy", "invocation"] {
        if !value[name]["transaction"].is_object() {
            bail!("pilot submission intent is missing its {name} transaction")
        }
    }
    Ok(PilotIntentContext {
        target_profile,
        chain_id,
        sender,
        deploy: value["deploy"].clone(),
        invocation: value["invocation"].clone(),
    })
}

fn validate_pilot_release_bundle(value: &Value) -> Result<PilotBundleContext> {
    if value["format"].as_str() != Some("clg.contract-release-bundle.v1")
        || value["schema_version"].as_u64() != Some(1)
        || value["target_profile"].as_str() != Some(TARGET_PROFILE)
    {
        bail!("pilot release bundle has an unsupported format, schema version, or target profile")
    }
    let signature_digest = digest_value(
        &value["artifacts"]["signature"]["sha256"],
        "pilot release bundle signature",
    )?;
    Ok(PilotBundleContext {
        target_profile: TARGET_PROFILE.to_string(),
        signature_digest,
    })
}

fn validate_pilot_verifier_result(value: &Value, bundle_digest: &str) -> Result<()> {
    let keys = value
        .as_object()
        .map(|object| object.keys().map(String::as_str).collect::<Vec<_>>())
        .ok_or_else(|| anyhow!("pilot release verifier result must be an object"))?;
    if keys
        != [
            "format",
            "release_bundle_digest",
            "schema_version",
            "status",
        ]
        || value["format"].as_str() != Some(PILOT_VERIFIER_RESULT_FORMAT)
        || value["schema_version"].as_u64() != Some(1)
        || value["status"].as_str() != Some("verified")
        || value["release_bundle_digest"].as_str() != Some(bundle_digest)
    {
        bail!("pilot release verifier result does not verify this release bundle")
    }
    Ok(())
}

fn capture_pilot_observation(
    root: &Path,
    observation: &PilotEvidenceObservationInput,
    intent: &PilotIntentContext,
    bundle: &PilotBundleContext,
) -> Result<Value> {
    let caller = canonical_address(&observation.caller).context("pilot observation caller")?;
    let (rpc_bytes, rpc_value) = load_pilot_evidence_file(
        root,
        &observation.rpc_observations,
        "RPC observations",
        true,
    )?;
    let transaction_digest = validate_pilot_rpc_observations(
        &rpc_value,
        &observation.kind,
        intent.chain_id,
        &intent.target_profile,
    )?;
    let receipt = match observation.kind.as_str() {
        "deploy" | "authorized-invoke" => {
            if caller != intent.sender {
                bail!(
                    "pilot {} caller does not match the reviewed sender",
                    observation.kind
                )
            }
            let receipt_path = observation.receipt.as_deref().ok_or_else(|| {
                anyhow!(
                    "pilot {} observation requires a canonical target receipt",
                    observation.kind
                )
            })?;
            let (receipt_bytes, receipt) =
                load_pilot_evidence_file(root, receipt_path, "target receipt", true)?;
            validate_pilot_receipt(
                &receipt,
                &observation.kind,
                &caller,
                &transaction_digest,
                intent,
                bundle,
            )?;
            json!({
                "digest": sha256_digest(&receipt_bytes),
                "request_digest": receipt["request_digest"],
                "transaction_digest": transaction_digest,
            })
        }
        "unauthorized-invoke" => {
            if caller == intent.sender {
                bail!("pilot unauthorized-invoke caller must differ from the reviewed sender")
            }
            if observation.receipt.is_some() {
                bail!("pilot unauthorized-invoke must not claim a successful target receipt")
            }
            Value::Null
        }
        _ => bail!("unsupported pilot observation kind `{}`", observation.kind),
    };
    Ok(json!({
        "caller": caller,
        "kind": observation.kind,
        "receipt": receipt,
        "rpc_observations": {
            "digest": sha256_digest(&rpc_bytes),
            "exchanges": rpc_value["exchanges"],
        },
        "transaction_digest": transaction_digest,
    }))
}

fn validate_pilot_rpc_observations(
    value: &Value,
    kind: &str,
    chain_id: u64,
    target_profile: &str,
) -> Result<String> {
    reject_secret_fields(value, "RPC observations")?;
    let observations: PilotRpcObservations =
        serde_json::from_value(value.clone()).context("parse pilot RPC observations")?;
    if observations.format != PILOT_RPC_OBSERVATIONS_FORMAT
        || observations.schema_version != 1
        || observations.chain_id != chain_id
        || observations.target_profile != target_profile
    {
        bail!("pilot RPC observations do not match the reviewed target")
    }
    if observations.exchanges.len() < 3
        || observations.exchanges[0].request.method != "eth_chainId"
        || observations.exchanges[1].request.method != "eth_sendRawTransaction"
        || observations.exchanges[2..]
            .iter()
            .any(|exchange| exchange.request.method != "eth_getTransactionReceipt")
        || observations
            .exchanges
            .iter()
            .any(|exchange| !exchange.request.params.is_array())
    {
        bail!("pilot RPC observations must be ordered eth_chainId, eth_sendRawTransaction, eth_getTransactionReceipt")
    }
    let observed_chain = observations.exchanges[0].response["result"]
        .as_str()
        .ok_or_else(|| anyhow!("pilot RPC chain-id response is missing a result"))?;
    if parse_quantity(&Value::String(observed_chain.to_string()))? != chain_id {
        bail!("pilot RPC chain-id response does not match the reviewed chain")
    }
    let transaction_digest = observations.exchanges[1].response["result"]
        .as_str()
        .ok_or_else(|| anyhow!("pilot RPC submission response is missing a transaction digest"))
        .and_then(|digest| canonical_hash(digest, "pilot RPC transaction digest"))?;
    let receipt_status = observations
        .exchanges
        .last()
        .and_then(|exchange| exchange.response["result"]["status"].as_str());
    let expected_status = if kind == "unauthorized-invoke" {
        "0x0"
    } else {
        "0x1"
    };
    if receipt_status != Some(expected_status) {
        bail!("pilot RPC observations have an unexpected final receipt status")
    }
    Ok(transaction_digest)
}

fn validate_pilot_receipt(
    receipt: &Value,
    kind: &str,
    caller: &str,
    transaction_digest: &str,
    intent: &PilotIntentContext,
    bundle: &PilotBundleContext,
) -> Result<()> {
    validate_transaction_receipt(receipt)?;
    if receipt["target_profile"].as_str() != Some(bundle.target_profile.as_str())
        || receipt["chain_id"].as_u64() != Some(intent.chain_id)
        || receipt["caller"].as_str() != Some(caller)
        || receipt["transaction_hash"].as_str() != Some(transaction_digest)
        || receipt["evidence"]["signed_bundle_digest"].as_str()
            != Some(bundle.signature_digest.as_str())
    {
        bail!("pilot target receipt does not bind the reviewed identity evidence")
    }
    let request = &receipt["request"];
    let planned = if kind == "deploy" {
        &intent.deploy["transaction"]
    } else {
        &intent.invocation["transaction"]
    };
    for (request_key, planned_key) in [
        ("nonce", "nonce"),
        ("value", "value"),
        ("gas_limit", "gas_limit"),
    ] {
        if request[request_key] != planned[planned_key] {
            bail!("pilot target receipt {request_key} does not match submission intent")
        }
    }
    if request["gas_price"] != planned["fee"]["gas_price"] {
        bail!("pilot target receipt gas_price does not match submission intent")
    }
    if kind == "deploy" {
        if !request["to"].is_null() {
            bail!("pilot deploy receipt must not have a target address")
        }
    } else {
        let address = intent.invocation["contract_address"]
            .as_str()
            .ok_or_else(|| {
                anyhow!("pilot submission intent is missing invocation contract_address")
            })?;
        let selector = intent.invocation["function_selector"]
            .as_str()
            .ok_or_else(|| {
                anyhow!("pilot submission intent is missing invocation function_selector")
            })?;
        if request["to"].as_str() != Some(address)
            || !request["data"]
                .as_str()
                .is_some_and(|data| data.starts_with(selector))
        {
            bail!("pilot invocation receipt does not match the reviewed target or selector")
        }
    }
    Ok(())
}

fn reject_secret_fields(value: &Value, description: &str) -> Result<()> {
    let forbidden = [
        "authorization",
        "endpoint",
        "password",
        "private_key",
        "rpc_url",
        "secret",
        "signer_reference",
        "token",
    ];
    match value {
        Value::Array(values) => {
            for value in values {
                reject_secret_fields(value, description)?;
            }
        }
        Value::Object(values) => {
            for (key, value) in values {
                if forbidden.contains(&key.to_ascii_lowercase().as_str()) {
                    bail!("pilot {description} contains forbidden secret or endpoint field `{key}`")
                }
                reject_secret_fields(value, description)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn sha256_digest(bytes: &[u8]) -> String {
    format!("sha256:{}", sha256_hex(bytes))
}

pub fn deploy(args: DeployArgs) -> Result<()> {
    let endpoint = preflight(&args.target_profile, &args.rpc_url, args.chain_id)?;
    let artifact = load_artifact(&args.artifact, &args.target_profile)?;
    let abi = load_wire_abi(&args.abi, &args.target_profile)?;
    let evidence = load_evidence(
        &args.contract_state_schema,
        &args.proof_artifact,
        &args.signed_bundle,
        &artifact,
        &abi,
    )?;
    let constructor_args = args
        .args
        .as_ref()
        .map(|path| read_json(path, "constructor arguments"))
        .transpose()?
        .unwrap_or_else(|| json!([]));
    let constructor_data = encode_constructor_args(&abi.value, &constructor_args)?;
    let mut deploy_data = artifact.bytecode.clone();
    deploy_data.extend(constructor_data);
    let signer = transaction::load_signer(&args.signing_key)?;
    let sender = canonical_address(&args.sender)?;
    if sender != signer.address {
        bail!("explicit sender does not match the EVM signing key address")
    }

    verify_chain(&endpoint, args.chain_id)?;
    let raw_transaction = transaction::sign_legacy(
        &transaction::LegacyTransaction {
            chain_id: args.chain_id,
            nonce: args.nonce,
            gas_price: args.gas_price,
            gas_limit: args.gas_limit,
            to: None,
            value: args.value,
            data: &deploy_data,
        },
        &signer,
    )?;
    let transaction_hash = submit_transaction(&endpoint, &raw_transaction)?;
    let onchain = wait_for_successful_receipt(&endpoint, &transaction_hash)?;
    let contract_address = onchain
        .contract_address
        .clone()
        .ok_or_else(|| anyhow!("malformed deploy receipt: missing contractAddress"))?;
    let request = json!({
        "chain_id": args.chain_id,
        "data": format!("0x{}", hex::encode(&deploy_data)),
        "from": sender,
        "gas_limit": args.gas_limit,
        "gas_price": args.gas_price,
        "nonce": args.nonce,
        "to": Value::Null,
        "value": args.value,
    });
    write_transaction_receipt(
        &args.receipt_out,
        &artifact,
        &abi,
        "deploy",
        &endpoint,
        args.chain_id,
        request,
        transaction_hash,
        onchain,
        Some(contract_address),
        None,
        evidence,
    )
}

pub fn call(args: CallArgs) -> Result<()> {
    let endpoint = preflight(&args.target_profile, &args.rpc_url, args.chain_id)?;
    let artifact = load_artifact(&args.artifact, &args.target_profile)?;
    let abi = load_wire_abi(&args.abi, &args.target_profile)?;
    let evidence = load_evidence(
        &args.contract_state_schema,
        &args.proof_artifact,
        &args.signed_bundle,
        &artifact,
        &abi,
    )?;
    let input = read_json(&args.args, "call arguments")?;
    let calldata = encode_calldata(&abi.value, &args.function, &input)?;

    verify_chain(&endpoint, args.chain_id)?;
    let request = json!({
        "data": calldata,
        "to": canonical_address(&args.contract_address)?,
    });
    let result = rpc(&endpoint, "eth_call", json!([request, "latest"]))?;
    let return_data = result
        .as_str()
        .filter(|value| value.starts_with("0x"))
        .ok_or_else(|| anyhow!("malformed eth_call result; expected 0x-prefixed bytes"))?;
    let call_request = json!({
        "chain_id": args.chain_id,
        "data": calldata,
        "function": args.function,
        "to": canonical_address(&args.contract_address)?,
    });
    let receipt = json!({
        "artifact_digest": artifact.digest,
        "block_reference": "latest",
        "call_identifier": format!("sha256:{}", sha256_hex(&canonical_json_bytes(&call_request))),
        "chain_id": args.chain_id,
        "command": "call",
        "contract_address": call_request["to"].clone(),
        "evidence": {
            "compiler": evidence.compiler,
            "proof_artifact_digest": evidence.proof_artifact_digest,
            "schema_digest": evidence.schema_digest,
            "signed_bundle_digest": evidence.signed_bundle_digest,
            "source_graph": evidence.source_graph,
        },
        "function": args.function,
        "request_digest": format!("sha256:{}", sha256_hex(&canonical_json_bytes(&call_request))),
        "result": { "return_data": return_data, "status": "success" },
        "rpc_endpoint": endpoint,
        "schema_version": 1,
        "target_profile": TARGET_PROFILE,
        "wire_abi_digest": abi.digest,
        "format": RECEIPT_FORMAT,
    });
    fs::write(&args.receipt_out, canonical_json_bytes(&receipt))
        .with_context(|| format!("write target receipt `{}`", args.receipt_out.display()))
}

pub fn invoke(args: InvokeArgs) -> Result<()> {
    let endpoint = preflight(&args.target_profile, &args.rpc_url, args.chain_id)?;
    let artifact = load_artifact(&args.artifact, &args.target_profile)?;
    let abi = load_wire_abi(&args.abi, &args.target_profile)?;
    let evidence = load_evidence(
        &args.contract_state_schema,
        &args.proof_artifact,
        &args.signed_bundle,
        &artifact,
        &abi,
    )?;
    let input = read_json(&args.args, "invoke arguments")?;
    let calldata = encode_calldata(&abi.value, &args.function, &input)?;
    let signer = transaction::load_signer(&args.signing_key)?;
    let sender = canonical_address(&args.sender)?;
    if sender != signer.address {
        bail!("explicit sender does not match the EVM signing key address")
    }
    let contract_address = canonical_address(&args.contract_address)?;
    let calldata_bytes = valid_hex_bytes(&calldata)?;

    verify_chain(&endpoint, args.chain_id)?;
    let raw_transaction = transaction::sign_legacy(
        &transaction::LegacyTransaction {
            chain_id: args.chain_id,
            nonce: args.nonce,
            gas_price: args.gas_price,
            gas_limit: args.gas_limit,
            to: Some(&contract_address),
            value: args.value,
            data: &calldata_bytes,
        },
        &signer,
    )?;
    let transaction_hash = submit_transaction(&endpoint, &raw_transaction)?;
    let onchain = wait_for_successful_receipt(&endpoint, &transaction_hash)?;
    let request = json!({
        "chain_id": args.chain_id,
        "data": calldata,
        "from": sender,
        "gas_limit": args.gas_limit,
        "gas_price": args.gas_price,
        "nonce": args.nonce,
        "to": contract_address,
        "value": args.value,
    });
    write_transaction_receipt(
        &args.receipt_out,
        &artifact,
        &abi,
        "invoke",
        &endpoint,
        args.chain_id,
        request,
        transaction_hash,
        onchain,
        Some(contract_address),
        Some(args.function),
        evidence,
    )
}

struct LoadedArtifact {
    digest: String,
    bytecode: Vec<u8>,
    value: Value,
    compiler: Value,
    contract: Value,
    source_graph: Value,
    state_schema_digest: Option<String>,
    wire_abi_digest: String,
}

struct LoadedWireAbi {
    digest: String,
    value: Value,
    compiler: Value,
    contract: Value,
    source_graph: Value,
}

#[derive(Debug)]
struct Evidence {
    compiler: Value,
    proof_artifact_digest: String,
    schema_digest: String,
    signed_bundle_digest: String,
    source_graph: Value,
}

fn load_artifact(path: &PathBuf, target_profile: &str) -> Result<LoadedArtifact> {
    let bytes =
        fs::read(path).with_context(|| format!("read target artifact `{}`", path.display()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse target artifact `{}`", path.display()))?;
    if value["target"]["profile"].as_str() != Some(target_profile) {
        bail!("target artifact does not declare profile `{target_profile}`");
    }
    let bytecode = value["bytecode"]
        .as_str()
        .or_else(|| value["target"]["bytecode"].as_str())
        .ok_or_else(|| anyhow!("target artifact is missing EVM bytecode"))?;
    let bytecode =
        valid_hex_bytes(bytecode).context("target artifact has malformed EVM bytecode")?;
    if bytecode.is_empty() {
        bail!("target artifact has empty EVM bytecode");
    }
    let compiler = identity_value(&value, "compiler", "target artifact")?;
    let contract = identity_value(&value, "contract", "target artifact")?;
    let source_graph = source_graph_value(&value, "target artifact")?;
    let wire_abi_digest = digest_value(&value["wire_abi"]["digest"], "target artifact wire ABI")?;
    let state_schema_digest = match &value["state_schema"]["digest"] {
        Value::Null => None,
        value => Some(digest_value(value, "target artifact state schema")?),
    };
    Ok(LoadedArtifact {
        digest: format!("sha256:{}", sha256_hex(&bytes)),
        bytecode,
        value,
        compiler,
        contract,
        source_graph,
        state_schema_digest,
        wire_abi_digest,
    })
}

fn load_wire_abi(path: &PathBuf, target_profile: &str) -> Result<LoadedWireAbi> {
    let bytes = fs::read(path).with_context(|| format!("read wire ABI `{}`", path.display()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse wire ABI `{}`", path.display()))?;
    if value["wire_abi"]["format"].as_str() != Some(WIRE_ABI_FORMAT) {
        bail!("ABI is not a `{WIRE_ABI_FORMAT}` artifact");
    }
    if value["target"]["profile"].as_str() != Some(target_profile) {
        bail!("wire ABI does not declare profile `{target_profile}`");
    }
    let digest = value["wire_abi"]["digest"]
        .as_str()
        .filter(|value| value.starts_with("sha256:"))
        .ok_or_else(|| anyhow!("wire ABI is missing its canonical digest"))?
        .to_string();
    Ok(LoadedWireAbi {
        digest,
        compiler: identity_value(&value, "compiler", "wire ABI")?,
        contract: identity_value(&value, "contract", "wire ABI")?,
        source_graph: source_graph_value(&value, "wire ABI")?,
        value,
    })
}

fn load_evidence(
    schema_path: &PathBuf,
    proof_path: &PathBuf,
    signed_bundle_path: &PathBuf,
    artifact: &LoadedArtifact,
    abi: &LoadedWireAbi,
) -> Result<Evidence> {
    if artifact.wire_abi_digest != abi.digest {
        bail!("target artifact wire-ABI digest does not match the supplied wire ABI")
    }
    ensure_same_identity(
        "target artifact",
        &artifact.compiler,
        "wire ABI",
        &abi.compiler,
    )?;
    ensure_same_identity(
        "target artifact",
        &artifact.source_graph,
        "wire ABI",
        &abi.source_graph,
    )?;
    ensure_same_identity(
        "target artifact",
        &artifact.contract,
        "wire ABI",
        &abi.contract,
    )?;

    let (schema_bytes, schema) = read_json_with_bytes(schema_path, "contract-state schema")?;
    if schema["schema"]["format"].as_str() != Some("clg.contract-state-schema.v1") {
        bail!("contract-state schema has an unsupported format")
    }
    let schema_digest = digest_value(&schema["schema"]["digest"], "contract-state schema")?;
    ensure_same_identity(
        "target artifact",
        &artifact.compiler,
        "contract-state schema",
        &identity_value(&schema, "compiler", "contract-state schema")?,
    )?;
    ensure_same_identity(
        "target artifact",
        &artifact.source_graph,
        "contract-state schema",
        &source_graph_value(&schema, "contract-state schema")?,
    )?;
    ensure_same_identity(
        "target artifact",
        &artifact.contract,
        "contract-state schema",
        &identity_value(&schema, "contract", "contract-state schema")?,
    )?;
    if let Some(expected) = &artifact.state_schema_digest {
        if expected != &schema_digest {
            bail!("target artifact state-schema digest does not match the supplied contract-state schema")
        }
    }
    validate_caller_authorization_binding(&schema, &artifact.value, &abi.value)?;

    let (_proof_bytes, proof) = read_json_with_bytes(proof_path, "proof artifact")?;
    if proof["format"].as_str() != Some("clg.proof_artifact.v1") {
        bail!("proof artifact has an unsupported format")
    }
    ensure_same_identity(
        "target artifact",
        &artifact.compiler,
        "proof artifact",
        &identity_value(&proof, "compiler", "proof artifact")?,
    )?;
    ensure_same_identity(
        "target artifact",
        &artifact.source_graph,
        "proof artifact",
        &source_graph_value(&proof, "proof artifact")?,
    )?;
    let proof_artifact_digest = format!("sha256:{}", sha256_hex(&canonical_json_bytes(&proof)));

    let (signed_bundle_bytes, signed_bundle) =
        read_json_with_bytes(signed_bundle_path, "signed bundle")?;
    let payload = signed_bundle["payload"]
        .as_object()
        .ok_or_else(|| anyhow!("signed bundle is missing its payload object"))?;
    if signed_bundle["signature_format"].as_str() != Some("ed25519")
        || signed_bundle["signature"].as_str().is_none()
        || signed_bundle["key_id"].as_str().is_none()
    {
        bail!("signed bundle has malformed signing metadata")
    }
    let payload = Value::Object(payload.clone());
    ensure_same_identity(
        "target artifact",
        &artifact.compiler,
        "signed bundle",
        &identity_value(&payload, "compiler", "signed bundle")?,
    )?;
    ensure_same_identity(
        "target artifact",
        &artifact.source_graph,
        "signed bundle",
        &source_graph_value(&payload, "signed bundle")?,
    )?;
    if digest_value(
        &payload["proof_artifact_hash"],
        "signed bundle proof artifact",
    )? != proof_artifact_digest
    {
        bail!("signed bundle proof-artifact hash does not match the supplied proof artifact")
    }
    validate_caller_authorization_evidence(&schema, &proof, &payload)?;

    let _ = schema_bytes;
    Ok(Evidence {
        compiler: artifact.compiler.clone(),
        proof_artifact_digest,
        schema_digest,
        signed_bundle_digest: format!("sha256:{}", sha256_hex(&signed_bundle_bytes)),
        source_graph: artifact.source_graph.clone(),
    })
}

fn validate_caller_authorization_evidence(
    schema: &Value,
    proof: &Value,
    signed_payload: &Value,
) -> Result<()> {
    let expected = &schema["caller_authorization"];
    for (name, actual) in [
        ("proof artifact", &proof["caller_authorization"]),
        ("signed bundle", &signed_payload["caller_authorization"]),
    ] {
        if canonical_json_bytes(actual) != canonical_json_bytes(expected) {
            bail!("caller-authorization drift: contract-state schema does not match {name}");
        }
    }
    Ok(())
}

fn validate_caller_authorization_binding(
    schema: &Value,
    artifact: &Value,
    abi: &Value,
) -> Result<()> {
    let authorization = &schema["caller_authorization"];
    if authorization.is_null() {
        for (name, actual) in [
            (
                "target artifact",
                &artifact["state_mapping"]["caller_authorization"],
            ),
            ("wire ABI", &abi["caller_authorization"]),
        ] {
            if !actual.is_null() {
                bail!("caller-authorization drift: contract-state schema does not match {name}");
            }
        }
        return Ok(());
    }
    let authorization = authorization.as_object().ok_or_else(|| {
        anyhow!("contract-state schema caller authorization must be an object or null")
    })?;
    if authorization.get("entry_symbol").and_then(Value::as_str) != Some("clg.caller.entry")
        || authorization
            .get("identity")
            .and_then(|identity| identity.get("encoding"))
            .and_then(Value::as_str)
            != Some("evm-address-20-byte")
        || authorization
            .get("identity")
            .and_then(|identity| identity.get("width_bits"))
            .and_then(Value::as_u64)
            != Some(160)
        || authorization
            .get("owner_field_id")
            .and_then(Value::as_str)
            .is_none()
        || authorization
            .get("transitions")
            .and_then(Value::as_array)
            .is_none()
    {
        bail!("contract-state schema has an invalid caller-authorization descriptor");
    }
    let expected = Value::Object(authorization.clone());
    for (name, actual) in [
        (
            "target artifact",
            &artifact["state_mapping"]["caller_authorization"],
        ),
        ("wire ABI", &abi["caller_authorization"]),
    ] {
        if canonical_json_bytes(actual) != canonical_json_bytes(&expected) {
            bail!("caller-authorization drift: contract-state schema does not match {name}");
        }
    }
    Ok(())
}

fn read_json_with_bytes(path: &PathBuf, description: &str) -> Result<(Vec<u8>, Value)> {
    let bytes =
        fs::read(path).with_context(|| format!("read {description} `{}`", path.display()))?;
    let value = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse {description} `{}`", path.display()))?;
    Ok((bytes, value))
}

fn identity_value(value: &Value, key: &str, description: &str) -> Result<Value> {
    let value = value[key].clone();
    if !value.is_object() {
        bail!("{description} is missing its `{key}` identity object")
    }
    Ok(value)
}

fn source_graph_value(value: &Value, description: &str) -> Result<Value> {
    let source_graph = identity_value(value, "source_graph", description)?;
    if source_graph["algorithm"].as_str() != Some("clg.loaded-source-graph.v1") {
        bail!("{description} has an unsupported loaded-source graph")
    }
    let digest = digest_value(&source_graph["digest"], "loaded-source graph")?;
    let files = source_graph["files"]
        .as_array()
        .ok_or_else(|| anyhow!("{description} loaded-source graph is missing files"))?;
    let identity = json!({ "algorithm": source_graph["algorithm"], "files": files });
    if digest != format!("sha256:{}", sha256_hex(&canonical_json_bytes(&identity))) {
        bail!("{description} loaded-source graph digest is invalid")
    }
    Ok(source_graph)
}

fn digest_value(value: &Value, description: &str) -> Result<String> {
    let digest = value
        .as_str()
        .filter(|digest| {
            digest.len() == "sha256:".len() + 64
                && digest.starts_with("sha256:")
                && digest["sha256:".len()..]
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
        .ok_or_else(|| anyhow!("{description} is missing a canonical sha256 digest"))?;
    Ok(digest.to_string())
}

fn ensure_same_identity(
    left_name: &str,
    left: &Value,
    right_name: &str,
    right: &Value,
) -> Result<()> {
    if canonical_json_bytes(left) != canonical_json_bytes(right) {
        bail!("cross-artifact identity drift: {left_name} does not match {right_name}")
    }
    Ok(())
}

fn read_json(path: &PathBuf, description: &str) -> Result<Value> {
    let bytes =
        fs::read(path).with_context(|| format!("read {description} `{}`", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parse {description} `{}`", path.display()))
}

fn encode_calldata(abi: &Value, function: &str, args: &Value) -> Result<String> {
    let function = abi["functions"]
        .as_array()
        .and_then(|functions| {
            functions
                .iter()
                .find(|item| item["selector"].as_str() == Some(function))
        })
        .ok_or_else(|| anyhow!("wire ABI does not contain function selector `{function}`"))?;
    let inputs = function["inputs"]
        .as_array()
        .ok_or_else(|| anyhow!("malformed wire ABI function inputs"))?;
    let values = abi_values(inputs, args, "call")?;
    let mut encoded = valid_hex_bytes(
        function["selector"]
            .as_str()
            .ok_or_else(|| anyhow!("malformed function selector"))?,
    )?;
    for (input, value) in inputs.iter().zip(values.iter()) {
        let ty = input["evm_type"]
            .as_str()
            .ok_or_else(|| anyhow!("malformed wire ABI input type"))?;
        encoded.extend(encode_static_word(ty, value)?);
    }
    Ok(format!("0x{}", hex::encode(encoded)))
}

fn encode_constructor_args(abi: &Value, args: &Value) -> Result<Vec<u8>> {
    let inputs = match &abi["constructor"] {
        Value::Null => {
            let values = abi_values(&[], args, "constructor")?;
            if !values.is_empty() {
                bail!("wire ABI does not declare constructor arguments")
            }
            return Ok(Vec::new());
        }
        constructor => constructor["inputs"]
            .as_array()
            .ok_or_else(|| anyhow!("malformed wire ABI constructor inputs"))?,
    };
    let values = abi_values(inputs, args, "constructor")?;
    let mut encoded = Vec::with_capacity(inputs.len() * 32);
    for (input, value) in inputs.iter().zip(values.iter()) {
        let ty = input["evm_type"]
            .as_str()
            .ok_or_else(|| anyhow!("malformed wire ABI constructor input type"))?;
        encoded.extend(encode_static_word(ty, value)?);
    }
    Ok(encoded)
}

fn abi_values<'a>(inputs: &'a [Value], args: &'a Value, label: &str) -> Result<Vec<&'a Value>> {
    let values = match args {
        Value::Array(values) => values.iter().collect(),
        Value::Object(object) => inputs
            .iter()
            .map(|input| {
                let name = input["name"]
                    .as_str()
                    .ok_or_else(|| anyhow!("malformed wire ABI input"))?;
                object
                    .get(name)
                    .ok_or_else(|| anyhow!("missing {label} argument `{name}`"))
            })
            .collect::<Result<Vec<_>>>()?,
        _ => bail!("{label} arguments must be a JSON array or object"),
    };
    if values.len() != inputs.len() {
        bail!("{label} argument count does not match wire ABI");
    }
    Ok(values)
}

fn encode_static_word(ty: &str, value: &Value) -> Result<[u8; 32]> {
    let mut word = [0_u8; 32];
    match ty {
        "bool" => word[31] = u8::from(value.as_bool().ok_or_else(|| anyhow!("expected bool ABI argument"))?),
        "uint8" | "uint64" => {
            let number = parse_unsigned(value, ty)?;
            let width = if ty == "uint8" { 8 } else { 64 };
            if width == 8 && number > u8::MAX as u64 {
                bail!("value does not fit `{ty}`");
            }
            word[24..].copy_from_slice(&number.to_be_bytes());
        }
        "uint128" => {
            let number = parse_unsigned_u128(value, ty)?;
            word[16..].copy_from_slice(&number.to_be_bytes());
        }
        "int256" => {
            let number = parse_signed(value)?;
            if number < 0 { word.fill(0xff); }
            word[24..].copy_from_slice(&number.to_be_bytes());
        }
        "address" => {
            let address = value
                .as_str()
                .ok_or_else(|| anyhow!("expected canonical address ABI argument"))?;
            let address = canonical_address(address)
                .context("expected canonical address ABI argument")?;
            let bytes = hex::decode(address.trim_start_matches("0x"))
                .expect("canonical address has already been validated");
            word[12..].copy_from_slice(&bytes);
        }
        _ => bail!("EVM calldata encoder does not support dynamic or 256-bit type `{ty}` in the first target profile"),
    }
    Ok(word)
}

fn parse_unsigned(value: &Value, ty: &str) -> Result<u64> {
    match value {
        Value::String(value) if !value.starts_with('-') => value
            .parse::<u64>()
            .map_err(|_| anyhow!("value does not fit `{ty}`")),
        Value::Number(value) => value
            .as_u64()
            .ok_or_else(|| anyhow!("expected unsigned `{ty}` ABI argument")),
        _ => bail!("expected unsigned `{ty}` ABI argument"),
    }
}

fn parse_unsigned_u128(value: &Value, ty: &str) -> Result<u128> {
    match value {
        Value::String(value) if !value.starts_with('-') => value
            .parse::<u128>()
            .map_err(|_| anyhow!("value does not fit `{ty}`")),
        Value::Number(value) => value
            .as_u64()
            .map(u128::from)
            .ok_or_else(|| anyhow!("expected unsigned `{ty}` ABI argument")),
        _ => bail!("expected unsigned `{ty}` ABI argument"),
    }
}

fn parse_signed(value: &Value) -> Result<i64> {
    match value {
        Value::String(value) => value
            .parse()
            .map_err(|_| anyhow!("expected signed int256 ABI argument")),
        Value::Number(value) => value
            .as_i64()
            .ok_or_else(|| anyhow!("expected signed int256 ABI argument")),
        _ => bail!("expected signed int256 ABI argument"),
    }
}

#[derive(Debug)]
struct OnchainReceipt {
    block_hash: String,
    block_number: u64,
    contract_address: Option<String>,
}

fn verify_chain(endpoint: &str, chain_id: u64) -> Result<()> {
    let observed_chain = rpc(endpoint, "eth_chainId", json!([]))?;
    let observed_chain =
        parse_quantity(&observed_chain).context("malformed eth_chainId response")?;
    if observed_chain != chain_id {
        bail!("RPC chain-ID mismatch: requested {chain_id}, endpoint reported {observed_chain}",);
    }
    Ok(())
}

fn submit_transaction(endpoint: &str, raw_transaction: &[u8]) -> Result<String> {
    let expected = transaction::transaction_hash(raw_transaction);
    let submitted = rpc(
        endpoint,
        "eth_sendRawTransaction",
        json!([format!("0x{}", hex::encode(raw_transaction))]),
    )?;
    let submitted = submitted.as_str().ok_or_else(|| {
        anyhow!("malformed eth_sendRawTransaction response; expected transaction hash")
    })?;
    let submitted = canonical_hash(submitted, "transaction hash")?;
    if submitted != expected {
        bail!("eth_sendRawTransaction returned a hash that does not match the signed transaction")
    }
    Ok(submitted)
}

fn wait_for_successful_receipt(endpoint: &str, transaction_hash: &str) -> Result<OnchainReceipt> {
    for attempt in 0..RECEIPT_POLL_ATTEMPTS {
        let value = rpc(
            endpoint,
            "eth_getTransactionReceipt",
            json!([transaction_hash]),
        )?;
        if value.is_null() {
            if attempt + 1 < RECEIPT_POLL_ATTEMPTS {
                thread::sleep(RECEIPT_POLL_INTERVAL);
                continue;
            }
            break;
        }
        let receipt = value
            .as_object()
            .ok_or_else(|| anyhow!("malformed transaction receipt; expected object or null"))?;
        let observed_hash = receipt
            .get("transactionHash")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("malformed transaction receipt: missing transactionHash"))?;
        if canonical_hash(observed_hash, "receipt transaction hash")? != transaction_hash {
            bail!("transaction receipt hash does not match submitted transaction")
        }
        match receipt.get("status").and_then(Value::as_str) {
            Some("0x1") => {}
            Some("0x0") => bail!("transaction reverted; no successful target receipt was written"),
            _ => bail!("malformed transaction receipt: status must be 0x1 or 0x0"),
        }
        let block_number = receipt
            .get("blockNumber")
            .ok_or_else(|| anyhow!("malformed transaction receipt: missing blockNumber"))
            .and_then(parse_quantity)
            .context("malformed transaction receipt blockNumber")?;
        let block_hash = receipt
            .get("blockHash")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("malformed transaction receipt: missing blockHash"))?;
        let contract_address = match receipt.get("contractAddress") {
            None | Some(Value::Null) => None,
            Some(Value::String(address)) => Some(
                canonical_address(address)
                    .context("malformed transaction receipt contractAddress")?,
            ),
            _ => bail!("malformed transaction receipt: contractAddress must be a string or null"),
        };
        return Ok(OnchainReceipt {
            block_hash: canonical_hash(block_hash, "receipt block hash")?,
            block_number,
            contract_address,
        });
    }
    bail!("transaction receipt was not available after {RECEIPT_POLL_ATTEMPTS} explicit polls")
}

#[allow(clippy::too_many_arguments)]
fn write_transaction_receipt(
    output: &PathBuf,
    artifact: &LoadedArtifact,
    abi: &LoadedWireAbi,
    command: &str,
    endpoint: &str,
    chain_id: u64,
    request: Value,
    transaction_hash: String,
    onchain: OnchainReceipt,
    contract_address: Option<String>,
    function: Option<String>,
    evidence: Evidence,
) -> Result<()> {
    let request_digest = format!("sha256:{}", sha256_hex(&canonical_json_bytes(&request)));
    let caller = request.get("from").cloned().unwrap_or(Value::Null);
    let mut receipt = json!({
        "artifact_digest": artifact.digest,
        "block": { "hash": onchain.block_hash, "number": onchain.block_number },
        "caller": caller,
        "chain_id": chain_id,
        "command": command,
        "evidence": {
            "compiler": evidence.compiler,
            "proof_artifact_digest": evidence.proof_artifact_digest,
            "schema_digest": evidence.schema_digest,
            "signed_bundle_digest": evidence.signed_bundle_digest,
            "source_graph": evidence.source_graph,
        },
        "format": RECEIPT_FORMAT,
        "request": request,
        "request_digest": request_digest,
        "result": { "status": "success" },
        "rpc_endpoint": endpoint,
        "schema_version": 1,
        "target_profile": TARGET_PROFILE,
        "transaction_hash": transaction_hash,
        "wire_abi_digest": abi.digest,
    });
    if let Some(contract_address) = contract_address {
        receipt["contract_address"] = Value::String(contract_address);
    }
    if let Some(function) = function {
        receipt["function"] = Value::String(function);
    }
    validate_transaction_receipt(&receipt)?;
    fs::write(output, canonical_json_bytes(&receipt))
        .with_context(|| format!("write target receipt `{}`", output.display()))
}

fn validate_transaction_receipt(receipt: &Value) -> Result<()> {
    if receipt["format"].as_str() != Some(RECEIPT_FORMAT)
        || receipt["result"]["status"].as_str() != Some("success")
    {
        bail!("target receipt is not successful canonical evidence");
    }
    let request = receipt["request"]
        .as_object()
        .ok_or_else(|| anyhow!("target receipt is missing its canonical request object"))?;
    let request = Value::Object(request.clone());
    let expected_digest = format!("sha256:{}", sha256_hex(&canonical_json_bytes(&request)));
    if receipt["request_digest"].as_str() != Some(expected_digest.as_str()) {
        bail!("target receipt request digest does not match its canonical request");
    }
    match request.get("from") {
        Some(Value::String(sender)) => {
            let sender = canonical_address(sender)
                .context("target receipt request has a malformed caller address")?;
            if receipt["caller"].as_str() != Some(sender.as_str()) {
                bail!("target receipt caller does not match the signed request sender");
            }
        }
        None | Some(Value::Null) if receipt["caller"].is_null() => {}
        None | Some(Value::Null) => {
            bail!("target receipt must not declare a caller when its request has no sender");
        }
        _ => bail!("target receipt request sender must be a canonical address string or null"),
    }
    Ok(())
}

fn canonical_hash(value: &str, label: &str) -> Result<String> {
    let bytes = valid_hex_bytes(value).with_context(|| format!("malformed {label}"))?;
    if bytes.len() != 32 {
        bail!("malformed {label}: expected exactly 32 bytes")
    }
    Ok(format!("0x{}", hex::encode(bytes)))
}

fn preflight(target_profile: &str, rpc_url: &str, chain_id: u64) -> Result<String> {
    if target_profile != TARGET_PROFILE {
        bail!(
            "unsupported target profile `{target_profile}`; expected explicit `{TARGET_PROFILE}`"
        );
    }
    if !(rpc_url.starts_with("http://") || rpc_url.starts_with("https://")) {
        bail!("RPC endpoint must be an explicit http:// or https:// URL")
    }
    if rpc_url.split_once("://").is_some_and(|(_, tail)| {
        tail.split('/')
            .next()
            .is_some_and(|host| host.contains('@'))
    }) {
        bail!("RPC endpoint credentials are unsupported; use an endpoint without embedded credentials")
    }
    if chain_id != TARGET_CHAIN_ID {
        bail!(
            "target profile `{TARGET_PROFILE}` requires explicit Sepolia chain ID `{TARGET_CHAIN_ID}`; received `{chain_id}`"
        )
    }
    Ok(rpc_url.trim_end_matches('/').to_string())
}

fn rpc(endpoint: &str, method: &str, params: Value) -> Result<Value> {
    let request = json!({ "id": 1, "jsonrpc": "2.0", "method": method, "params": params });
    let body = String::from_utf8(canonical_json_bytes(&request)).expect("canonical JSON is UTF-8");
    let response = ureq::post(endpoint)
        .set("content-type", "application/json")
        .set("accept", "application/json")
        .send_string(&body)
        .map_err(|error| anyhow!("RPC request `{method}` failed: {error}"))?;
    let text = response
        .into_string()
        .map_err(|error| anyhow!("read RPC response `{method}`: {error}"))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| anyhow!("malformed RPC response `{method}`: {error}"))?;
    if value["jsonrpc"].as_str() != Some("2.0") || value["id"] != 1 {
        bail!("malformed RPC response `{method}`: JSON-RPC identity mismatch");
    }
    if !value["error"].is_null() {
        bail!("RPC `{method}` returned an error: {}", value["error"]);
    }
    value
        .get("result")
        .cloned()
        .ok_or_else(|| anyhow!("malformed RPC response `{method}`: missing result"))
}

fn parse_quantity(value: &Value) -> Result<u64> {
    match value {
        Value::String(value) => u64::from_str_radix(
            value
                .strip_prefix("0x")
                .ok_or_else(|| anyhow!("quantity must be 0x-prefixed"))?,
            16,
        )
        .map_err(Into::into),
        Value::Number(value) => value
            .as_u64()
            .ok_or_else(|| anyhow!("quantity must be unsigned")),
        _ => bail!("quantity must be a JSON string or number"),
    }
}

fn canonical_address(address: &str) -> Result<String> {
    let bytes = valid_hex_bytes(address)?;
    if bytes.len() != 20 {
        bail!("contract address must contain exactly 20 bytes");
    }
    Ok(format!("0x{}", hex::encode(bytes)))
}

fn valid_hex_bytes(value: &str) -> Result<Vec<u8>> {
    let value = value
        .strip_prefix("0x")
        .ok_or_else(|| anyhow!("hex bytes must be 0x-prefixed"))?;
    if value.len() % 2 != 0 {
        bail!("hex bytes must contain an even number of digits");
    }
    hex::decode(value).map_err(|error| anyhow!("invalid hexadecimal bytes: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use tempfile::tempdir;

    fn abi() -> Value {
        json!({
            "functions": [{
                "inputs": [{"name": "amount", "evm_type": "uint64"}, {"name": "ok", "evm_type": "bool"}],
                "selector": "0x13765838"
            }]
        })
    }

    fn mock_rpc<F>(request_count: usize, respond: F) -> (String, thread::JoinHandle<Vec<Value>>)
    where
        F: Fn(&Value) -> Value + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock RPC");
        let address = listener.local_addr().expect("mock RPC address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for _ in 0..request_count {
                let (mut stream, _) = listener.accept().expect("accept RPC request");
                let request = read_http_json(&mut stream);
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": respond(&request),
                });
                let body = canonical_json_bytes(&response);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .expect("write RPC headers");
                stream.write_all(&body).expect("write RPC body");
                requests.push(request);
            }
            requests
        });
        (format!("http://{address}"), server)
    }

    fn sepolia_fixture_set() -> Value {
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/fixtures/phase-42.0.3/sepolia-rpc-fixtures.json"
        )))
        .expect("parse Sepolia RPC fixtures");
        assert_eq!(fixture["format"], "clg.sepolia-rpc-fixture-set.v1");
        assert_eq!(fixture["schema_version"], 1);
        assert_eq!(fixture["target_profile"], TARGET_PROFILE);
        assert_eq!(fixture["chain_id"], TARGET_CHAIN_ID);
        fixture
    }

    fn sepolia_fixture_case(id: &str) -> Value {
        sepolia_fixture_set()["cases"]
            .as_array()
            .expect("fixture cases")
            .iter()
            .find(|case| case["id"] == id)
            .unwrap_or_else(|| panic!("missing fixture case `{id}`"))
            .clone()
    }

    fn sepolia_fixture_steps(case: &Value) -> Vec<Value> {
        case["responses"]
            .as_array()
            .expect("fixture responses")
            .iter()
            .flat_map(|step| {
                let repeat = step["repeat"].as_u64().unwrap_or(1);
                assert!((1..=RECEIPT_POLL_ATTEMPTS as u64).contains(&repeat));
                let mut step = step.clone();
                step.as_object_mut()
                    .expect("fixture response object")
                    .remove("repeat");
                std::iter::repeat_n(step, repeat as usize)
            })
            .collect()
    }

    fn resolve_fixture_transaction_hash(value: &Value, transaction_hash: &str) -> Value {
        match value {
            Value::String(value) if value == "$transaction_hash" => {
                Value::String(transaction_hash.to_string())
            }
            Value::Array(values) => Value::Array(
                values
                    .iter()
                    .map(|value| resolve_fixture_transaction_hash(value, transaction_hash))
                    .collect(),
            ),
            Value::Object(values) => Value::Object(
                values
                    .iter()
                    .map(|(key, value)| {
                        (
                            key.clone(),
                            resolve_fixture_transaction_hash(value, transaction_hash),
                        )
                    })
                    .collect(),
            ),
            value => value.clone(),
        }
    }

    fn mock_sepolia_fixture(case: Value) -> (String, thread::JoinHandle<Vec<Value>>) {
        let steps = sepolia_fixture_steps(&case);
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture RPC");
        let address = listener.local_addr().expect("fixture RPC address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            let mut transaction_hash = None;
            for step in steps {
                let (mut stream, _) = listener.accept().expect("accept fixture RPC request");
                let request = read_http_json(&mut stream);
                assert_eq!(request["method"], step["method"], "fixture request method");
                if request["method"] == "eth_sendRawTransaction" {
                    let raw = valid_hex_bytes(
                        request["params"][0]
                            .as_str()
                            .expect("fixture raw transaction"),
                    )
                    .expect("fixture raw transaction bytes");
                    transaction_hash = Some(transaction::transaction_hash(&raw));
                }
                let request_hash = request["params"]
                    .get(0)
                    .and_then(Value::as_str)
                    .filter(|value| value.starts_with("0x"));
                let token_hash = transaction_hash.as_deref().or(request_hash).unwrap_or(
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                );
                let mut response = json!({ "id": 1, "jsonrpc": "2.0" });
                if let Some(error) = step.get("error") {
                    response["error"] = error.clone();
                } else {
                    response["result"] = resolve_fixture_transaction_hash(
                        step.get("result").expect("fixture result"),
                        token_hash,
                    );
                }
                let body = canonical_json_bytes(&response);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .expect("write fixture RPC headers");
                stream.write_all(&body).expect("write fixture RPC body");
                requests.push(request);
            }
            requests
        });
        (format!("http://{address}"), server)
    }

    fn read_http_json(stream: &mut std::net::TcpStream) -> Value {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = stream.read(&mut buffer).expect("read RPC request");
            request.extend_from_slice(&buffer[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let header_end = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("header delimiter")
            + 4;
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .expect("content length")
            .parse::<usize>()
            .expect("numeric content length");
        while request.len() - header_end < content_length {
            let read = stream.read(&mut buffer).expect("read RPC body");
            request.extend_from_slice(&buffer[..read]);
        }
        serde_json::from_slice(&request[header_end..header_end + content_length])
            .expect("parse RPC request")
    }

    fn write_signer(dir: &tempfile::TempDir) -> PathBuf {
        let path = dir.path().join("signer.json");
        fs::write(
            &path,
            r#"{"format":"clg.evm-signing-key.v1","private_key":"0x0101010101010101010101010101010101010101010101010101010101010101","address":"0x1a642f0e3c3af545e7acbd38b07251b3990914f1"}"#,
        )
        .expect("write signing key");
        path
    }

    fn write_target_inputs(
        dir: &tempfile::TempDir,
        constructor: Value,
        functions: Value,
    ) -> (PathBuf, PathBuf) {
        let artifact = dir.path().join("contract.evm.json");
        let abi = dir.path().join("contract.abi.json");
        let compiler = json!({ "name": "clg-cli", "version": env!("CARGO_PKG_VERSION") });
        let source_graph = test_source_graph();
        let contract = json!({ "name": "Vault", "version": 1 });
        fs::write(
            &artifact,
            canonical_json_bytes(&json!({
                "bytecode": "0x6000",
                "compiler": compiler,
                "contract": contract,
                "source_graph": source_graph,
                "target": { "profile": TARGET_PROFILE },
                "wire_abi": { "digest": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd", "format": WIRE_ABI_FORMAT },
            })),
        )
        .expect("write artifact");
        fs::write(
            &abi,
            canonical_json_bytes(&json!({
                "constructor": constructor,
                "compiler": compiler,
                "contract": contract,
                "functions": functions,
                "source_graph": source_graph,
                "target": { "profile": TARGET_PROFILE },
                "wire_abi": { "digest": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd", "format": WIRE_ABI_FORMAT },
            })),
        )
        .expect("write ABI");
        (artifact, abi)
    }

    fn test_source_graph() -> Value {
        let identity = json!({ "algorithm": "clg.loaded-source-graph.v1", "files": [] });
        json!({
            "algorithm": identity["algorithm"],
            "digest": format!("sha256:{}", sha256_hex(&canonical_json_bytes(&identity))),
            "files": identity["files"],
        })
    }

    fn write_evidence(dir: &tempfile::TempDir) -> (PathBuf, PathBuf, PathBuf) {
        let schema = dir.path().join("schema.json");
        let proof = dir.path().join("proof.json");
        let signed_bundle = dir.path().join("bundle.sig.json");
        let compiler = json!({ "name": "clg-cli", "version": env!("CARGO_PKG_VERSION") });
        let source_graph = test_source_graph();
        let contract = json!({ "name": "Vault", "version": 1 });
        fs::write(
            &schema,
            canonical_json_bytes(&json!({
                "compiler": compiler,
                "contract": contract,
                "schema": { "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc", "format": "clg.contract-state-schema.v1" },
                "source_graph": source_graph,
            })),
        )
        .expect("write schema evidence");
        let proof_value = json!({
            "compiler": compiler,
            "format": "clg.proof_artifact.v1",
            "source_graph": source_graph,
        });
        let proof_digest = format!("sha256:{}", sha256_hex(&canonical_json_bytes(&proof_value)));
        fs::write(&proof, canonical_json_bytes(&proof_value)).expect("write proof evidence");
        fs::write(
            &signed_bundle,
            canonical_json_bytes(&json!({
                "key_id": "test-key",
                "payload": {
                    "compiler": compiler,
                    "proof_artifact_hash": proof_digest,
                    "source_graph": source_graph,
                },
                "signature": "00",
                "signature_format": "ed25519",
            })),
        )
        .expect("write signed-bundle evidence");
        (schema, proof, signed_bundle)
    }

    #[test]
    fn calldata_is_canonical_for_static_scalar_inputs() {
        assert_eq!(
            encode_calldata(&abi(), "0x13765838", &json!({"amount": 7, "ok": true})).expect("calldata"),
            "0x1376583800000000000000000000000000000000000000000000000000000000000000070000000000000000000000000000000000000000000000000000000000000001"
        );
    }

    #[test]
    fn preflight_requires_the_locked_profile_http_endpoint_and_chain() {
        assert!(preflight(
            TARGET_PROFILE,
            "https://rpc.example.invalid",
            TARGET_CHAIN_ID
        )
        .is_ok());
        assert!(
            preflight("other", "https://rpc.example.invalid", TARGET_CHAIN_ID)
                .unwrap_err()
                .to_string()
                .contains("unsupported target profile")
        );
        assert!(
            preflight(TARGET_PROFILE, "file:///tmp/rpc", TARGET_CHAIN_ID)
                .unwrap_err()
                .to_string()
                .contains("http:// or https://")
        );
        assert!(preflight(TARGET_PROFILE, "https://rpc.example.invalid", 0)
            .unwrap_err()
            .to_string()
            .contains("requires explicit Sepolia chain ID"));
        assert!(preflight(TARGET_PROFILE, "https://rpc.example.invalid", 1)
            .unwrap_err()
            .to_string()
            .contains("requires explicit Sepolia chain ID"));
    }

    #[test]
    fn sepolia_offline_fixture_set_is_sanitized_and_covers_the_locked_surface() {
        let fixtures = sepolia_fixture_set();
        let allowed_methods = [
            "eth_chainId",
            "eth_call",
            "eth_sendRawTransaction",
            "eth_getTransactionReceipt",
        ];
        let cases = fixtures["cases"].as_array().expect("fixture cases");
        assert_eq!(cases.len(), 10, "fixture matrix must remain complete");
        for case in cases {
            assert!(matches!(
                case["outcome"].as_str(),
                Some("success" | "failure")
            ));
            for step in sepolia_fixture_steps(case) {
                assert!(allowed_methods.contains(&step["method"].as_str().expect("method")));
                assert!(step.get("result").is_some() ^ step.get("error").is_some());
            }
        }
        let serialized = serde_json::to_string(&fixtures).expect("serialize fixtures");
        assert!(!serialized.contains("http://"));
        assert!(!serialized.contains("https://"));
        assert!(!serialized.contains("private_key"));
        assert!(!serialized.contains("authorization"));
    }

    #[test]
    fn sepolia_offline_fixtures_accept_success_paths_and_reject_failures() {
        let dir = tempdir().expect("tempdir");
        let (artifact, abi_path) = write_target_inputs(
            &dir,
            json!({"inputs": [{"name":"initial", "evm_type":"uint64"}]}),
            json!([{"inputs": [], "selector": "0x13765838"}]),
        );
        let (contract_state_schema, proof_artifact, signed_bundle) = write_evidence(&dir);
        let call_args = dir.path().join("call-args.json");
        let malformed_call_args = dir.path().join("call-malformed-args.json");
        let constructor_args = dir.path().join("constructor-args.json");
        let invoke_args = dir.path().join("invoke-args.json");
        fs::write(&call_args, b"[]").expect("write call arguments");
        fs::write(&malformed_call_args, b"[]").expect("write malformed call arguments");
        fs::write(&constructor_args, br#"{"initial":7}"#).expect("write constructor arguments");
        fs::write(&invoke_args, b"[]").expect("write invoke arguments");

        let (endpoint, server) = mock_sepolia_fixture(sepolia_fixture_case("call-success"));
        call(CallArgs {
            target_profile: TARGET_PROFILE.to_string(),
            rpc_url: endpoint,
            chain_id: TARGET_CHAIN_ID,
            artifact: artifact.clone(),
            abi: abi_path.clone(),
            contract_state_schema: contract_state_schema.clone(),
            proof_artifact: proof_artifact.clone(),
            signed_bundle: signed_bundle.clone(),
            contract_address: "0x1111111111111111111111111111111111111111".to_string(),
            function: "0x13765838".to_string(),
            args: call_args,
            receipt_out: dir.path().join("call-receipt.json"),
        })
        .expect("fixture call succeeds");
        assert_eq!(server.join().expect("join call fixture").len(), 2);

        let (endpoint, server) = mock_sepolia_fixture(sepolia_fixture_case("deploy-success"));
        deploy(DeployArgs {
            target_profile: TARGET_PROFILE.to_string(),
            rpc_url: endpoint,
            chain_id: TARGET_CHAIN_ID,
            sender: "0x1a642f0e3c3af545e7acbd38b07251b3990914f1".to_string(),
            nonce: 0,
            signing_key: write_signer(&dir),
            value: 0,
            gas_limit: 100_000,
            gas_price: 1,
            artifact: artifact.clone(),
            abi: abi_path.clone(),
            contract_state_schema: contract_state_schema.clone(),
            proof_artifact: proof_artifact.clone(),
            signed_bundle: signed_bundle.clone(),
            args: Some(constructor_args),
            receipt_out: dir.path().join("deploy-receipt.json"),
        })
        .expect("fixture deploy succeeds");
        assert_eq!(server.join().expect("join deploy fixture").len(), 3);

        let (endpoint, server) = mock_sepolia_fixture(sepolia_fixture_case("invoke-success"));
        invoke(InvokeArgs {
            target_profile: TARGET_PROFILE.to_string(),
            rpc_url: endpoint,
            chain_id: TARGET_CHAIN_ID,
            artifact: artifact.clone(),
            abi: abi_path.clone(),
            contract_state_schema: contract_state_schema.clone(),
            proof_artifact: proof_artifact.clone(),
            signed_bundle: signed_bundle.clone(),
            contract_address: "0x1111111111111111111111111111111111111111".to_string(),
            function: "0x13765838".to_string(),
            args: invoke_args,
            sender: "0x1a642f0e3c3af545e7acbd38b07251b3990914f1".to_string(),
            nonce: 1,
            signing_key: write_signer(&dir),
            value: 0,
            gas_limit: 100_000,
            gas_price: 1,
            receipt_out: dir.path().join("invoke-receipt.json"),
        })
        .expect("fixture invoke succeeds");
        assert_eq!(server.join().expect("join invoke fixture").len(), 3);

        let transaction_hash = "0xcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
        for id in [
            "wrong-chain",
            "substituted-transaction-hash",
            "reverted-receipt",
            "malformed-receipt",
        ] {
            let (endpoint, server) = mock_sepolia_fixture(sepolia_fixture_case(id));
            let result = match id {
                "wrong-chain" => verify_chain(&endpoint, TARGET_CHAIN_ID).map(|_| ()),
                "substituted-transaction-hash" => submit_transaction(&endpoint, &[1]).map(|_| ()),
                "reverted-receipt" | "malformed-receipt" => {
                    wait_for_successful_receipt(&endpoint, transaction_hash).map(|_| ())
                }
                _ => unreachable!("fixture list is exhaustive"),
            };
            assert!(result.is_err(), "fixture `{id}` must fail closed");
            assert_eq!(server.join().expect("join failure fixture").len(), 1);
        }

        let (endpoint, server) = mock_sepolia_fixture(sepolia_fixture_case("unavailable-receipt"));
        assert!(wait_for_successful_receipt(&endpoint, transaction_hash).is_err());
        assert_eq!(
            server.join().expect("join unavailable fixture").len(),
            RECEIPT_POLL_ATTEMPTS
        );

        let (endpoint, server) =
            mock_sepolia_fixture(sepolia_fixture_case("malformed-call-result"));
        let call_error = call(CallArgs {
            target_profile: TARGET_PROFILE.to_string(),
            rpc_url: endpoint,
            chain_id: TARGET_CHAIN_ID,
            artifact,
            abi: abi_path,
            contract_state_schema,
            proof_artifact,
            signed_bundle,
            contract_address: "0x1111111111111111111111111111111111111111".to_string(),
            function: "0x13765838".to_string(),
            args: malformed_call_args,
            receipt_out: dir.path().join("malformed-call-receipt.json"),
        })
        .expect_err("malformed call fixture must fail");
        assert!(call_error.to_string().contains("malformed eth_call result"));
        assert_eq!(server.join().expect("join malformed call fixture").len(), 2);

        let (endpoint, server) =
            mock_sepolia_fixture(sepolia_fixture_case("required-method-rpc-error"));
        assert!(verify_chain(&endpoint, TARGET_CHAIN_ID).is_err());
        assert_eq!(server.join().expect("join RPC error fixture").len(), 1);
    }

    #[test]
    fn call_checks_chain_id_submits_calldata_and_writes_a_canonical_receipt() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock RPC");
        let address = listener.local_addr().expect("mock RPC address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [
                json!({"jsonrpc":"2.0", "id":1, "result":"0xaa36a7"}),
                json!({"jsonrpc":"2.0", "id":1, "result":"0x"}),
            ] {
                let (mut stream, _) = listener.accept().expect("accept RPC request");
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1024];
                loop {
                    let read = stream.read(&mut buffer).expect("read RPC request");
                    request.extend_from_slice(&buffer[..read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let header_end = request
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .expect("header delimiter")
                    + 4;
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .expect("content length")
                    .parse::<usize>()
                    .expect("numeric content length");
                while request.len() - header_end < content_length {
                    let read = stream.read(&mut buffer).expect("read RPC body");
                    request.extend_from_slice(&buffer[..read]);
                }
                requests.push(
                    serde_json::from_slice::<Value>(
                        &request[header_end..header_end + content_length],
                    )
                    .expect("parse RPC request"),
                );
                let body = canonical_json_bytes(&response);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .expect("write RPC headers");
                stream.write_all(&body).expect("write RPC body");
            }
            requests
        });
        let dir = tempdir().expect("tempdir");
        let artifact = dir.path().join("contract.evm.json");
        let abi_path = dir.path().join("contract.abi.json");
        let args_path = dir.path().join("args.json");
        let receipt_path = dir.path().join("receipt.json");
        let (contract_state_schema, proof_artifact, signed_bundle) = write_evidence(&dir);
        let compiler = json!({ "name": "clg-cli", "version": env!("CARGO_PKG_VERSION") });
        let contract = json!({ "name": "Vault", "version": 1 });
        let source_graph = test_source_graph();
        fs::write(
            &artifact,
            canonical_json_bytes(&json!({
                "bytecode": "0x6000",
                "compiler": compiler,
                "contract": contract,
                "source_graph": source_graph,
                "target": { "profile": TARGET_PROFILE },
                "wire_abi": { "digest": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd", "format": WIRE_ABI_FORMAT },
            })),
        )
        .expect("write artifact");
        fs::write(
            &abi_path,
            canonical_json_bytes(&json!({
                "functions": [{
                    "inputs": [{"name": "amount", "evm_type": "uint64"}],
                    "selector": "0x13765838",
                }],
                "compiler": compiler,
                "contract": contract,
                "source_graph": source_graph,
                "target": { "profile": TARGET_PROFILE },
                "wire_abi": { "digest": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd", "format": WIRE_ABI_FORMAT },
            })),
        )
        .expect("write ABI");
        fs::write(&args_path, br#"{"amount":7}"#).expect("write args");

        call(CallArgs {
            target_profile: TARGET_PROFILE.to_string(),
            rpc_url: format!("http://{address}"),
            chain_id: TARGET_CHAIN_ID,
            artifact,
            abi: abi_path,
            contract_state_schema,
            proof_artifact,
            signed_bundle,
            contract_address: "0x1111111111111111111111111111111111111111".to_string(),
            function: "0x13765838".to_string(),
            args: args_path,
            receipt_out: receipt_path.clone(),
        })
        .expect("successful target call");

        let receipt: Value =
            serde_json::from_slice(&fs::read(&receipt_path).expect("read receipt"))
                .expect("parse receipt");
        assert_eq!(receipt["format"], RECEIPT_FORMAT);
        assert_eq!(receipt["command"], "call");
        assert_eq!(receipt["chain_id"], TARGET_CHAIN_ID);
        assert_eq!(receipt["result"]["status"], "success");
        let requests = server.join().expect("join mock RPC");
        assert_eq!(requests[0]["method"], "eth_chainId");
        assert_eq!(requests[1]["method"], "eth_call");
        assert_eq!(
            requests[1]["params"][0]["data"],
            "0x137658380000000000000000000000000000000000000000000000000000000000000007"
        );
    }

    #[test]
    fn deploy_signs_submits_constructor_data_and_writes_confirmed_receipt() {
        let (endpoint, server) = mock_rpc(3, |request| match request["method"].as_str() {
            Some("eth_chainId") => json!("0xaa36a7"),
            Some("eth_sendRawTransaction") => {
                let raw = valid_hex_bytes(request["params"][0].as_str().expect("raw transaction"))
                    .expect("valid raw transaction");
                json!(transaction::transaction_hash(&raw))
            }
            Some("eth_getTransactionReceipt") => json!({
                "blockHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "blockNumber": "0x2",
                "contractAddress": "0x2222222222222222222222222222222222222222",
                "status": "0x1",
                "transactionHash": request["params"][0].clone(),
            }),
            other => panic!("unexpected RPC method {other:?}"),
        });
        let dir = tempdir().expect("tempdir");
        let (artifact, abi) = write_target_inputs(
            &dir,
            json!({"inputs": [{"name":"initial", "evm_type":"uint64"}]}),
            json!([]),
        );
        let args = dir.path().join("constructor.json");
        let receipt = dir.path().join("receipt.json");
        let (contract_state_schema, proof_artifact, signed_bundle) = write_evidence(&dir);
        fs::write(&args, br#"{"initial":7}"#).expect("write arguments");

        deploy(DeployArgs {
            target_profile: TARGET_PROFILE.to_string(),
            rpc_url: endpoint,
            chain_id: TARGET_CHAIN_ID,
            sender: "0x1a642f0e3c3af545e7acbd38b07251b3990914f1".to_string(),
            nonce: 0,
            signing_key: write_signer(&dir),
            value: 0,
            gas_limit: 100_000,
            gas_price: 1,
            artifact,
            abi,
            contract_state_schema,
            proof_artifact,
            signed_bundle,
            args: Some(args),
            receipt_out: receipt.clone(),
        })
        .expect("successful deploy");

        let output: Value = serde_json::from_slice(&fs::read(receipt).expect("read receipt"))
            .expect("parse receipt");
        assert_eq!(output["command"], "deploy");
        assert_eq!(
            output["contract_address"],
            "0x2222222222222222222222222222222222222222"
        );
        assert_eq!(output["block"]["number"], 2);
        let requests = server.join().expect("join mock RPC");
        assert_eq!(
            requests
                .iter()
                .map(|request| request["method"].as_str())
                .collect::<Vec<_>>(),
            [
                Some("eth_chainId"),
                Some("eth_sendRawTransaction"),
                Some("eth_getTransactionReceipt")
            ]
        );
    }

    #[test]
    fn invoke_signs_submits_and_writes_confirmed_receipt() {
        let (endpoint, server) = mock_rpc(3, |request| match request["method"].as_str() {
            Some("eth_chainId") => json!("0xaa36a7"),
            Some("eth_sendRawTransaction") => {
                let raw = valid_hex_bytes(request["params"][0].as_str().expect("raw transaction"))
                    .expect("valid raw transaction");
                json!(transaction::transaction_hash(&raw))
            }
            Some("eth_getTransactionReceipt") => json!({
                "blockHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "blockNumber": "0x2",
                "contractAddress": null,
                "status": "0x1",
                "transactionHash": request["params"][0].clone(),
            }),
            other => panic!("unexpected RPC method {other:?}"),
        });
        let dir = tempdir().expect("tempdir");
        let (artifact, abi) = write_target_inputs(
            &dir,
            Value::Null,
            json!([{"inputs": [], "selector": "0x13765838"}]),
        );
        let args = dir.path().join("arguments.json");
        let receipt = dir.path().join("receipt.json");
        let (contract_state_schema, proof_artifact, signed_bundle) = write_evidence(&dir);
        fs::write(&args, b"[]").expect("write arguments");

        invoke(InvokeArgs {
            target_profile: TARGET_PROFILE.to_string(),
            rpc_url: endpoint,
            chain_id: TARGET_CHAIN_ID,
            artifact,
            abi,
            contract_state_schema,
            proof_artifact,
            signed_bundle,
            contract_address: "0x1111111111111111111111111111111111111111".to_string(),
            function: "0x13765838".to_string(),
            args,
            sender: "0x1a642f0e3c3af545e7acbd38b07251b3990914f1".to_string(),
            nonce: 0,
            signing_key: write_signer(&dir),
            value: 0,
            gas_limit: 100_000,
            gas_price: 1,
            receipt_out: receipt.clone(),
        })
        .expect("successful invoke");
        let output: Value = serde_json::from_slice(&fs::read(receipt).expect("read receipt"))
            .expect("parse receipt");
        assert_eq!(output["command"], "invoke");
        assert_eq!(
            output["caller"],
            "0x1a642f0e3c3af545e7acbd38b07251b3990914f1"
        );
        assert_eq!(
            output["contract_address"],
            "0x1111111111111111111111111111111111111111"
        );
        assert_eq!(output["result"]["status"], "success");
        let requests = server.join().expect("join mock RPC");
        assert_eq!(requests.len(), 3);
    }

    #[test]
    fn reverted_receipt_never_becomes_success_evidence() {
        let transaction_hash = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let (endpoint, server) = mock_rpc(1, move |request| {
            assert_eq!(request["method"], "eth_getTransactionReceipt");
            json!({
                "blockHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "blockNumber": "0x2",
                "contractAddress": null,
                "status": "0x0",
                "transactionHash": transaction_hash,
            })
        });
        let error = wait_for_successful_receipt(&endpoint, transaction_hash)
            .expect_err("reverted transaction must fail");
        assert!(error.to_string().contains("transaction reverted"));
        let requests = server.join().expect("join mock RPC");
        assert_eq!(requests.len(), 1);
    }

    #[test]
    fn address_arguments_are_encoded_as_a_canonical_20_byte_abi_word() {
        let word = encode_static_word(
            "address",
            &json!("0x1111111111111111111111111111111111111111"),
        )
        .expect("encode address");
        assert_eq!(&word[..12], &[0; 12]);
        assert_eq!(&word[12..], &[0x11; 20]);
        assert!(encode_static_word(
            "address",
            &json!("0x111111111111111111111111111111111111111G"),
        )
        .is_err());
    }

    #[test]
    fn target_receipt_rejects_a_substituted_caller() {
        let request = json!({
            "from": "0x1111111111111111111111111111111111111111",
            "to": "0x2222222222222222222222222222222222222222",
        });
        let request_digest = format!("sha256:{}", sha256_hex(&canonical_json_bytes(&request)));
        let mut receipt = json!({
            "caller": request["from"].clone(),
            "format": RECEIPT_FORMAT,
            "request": request.clone(),
            "request_digest": request_digest,
            "result": { "status": "success" },
        });
        validate_transaction_receipt(&receipt).expect("canonical receipt");

        receipt["caller"] = json!("0x3333333333333333333333333333333333333333");
        let error = validate_transaction_receipt(&receipt)
            .expect_err("substituted caller must fail closed");
        assert!(error
            .to_string()
            .contains("caller does not match the signed request sender"));
    }

    #[test]
    fn cross_artifact_compiler_drift_is_rejected_before_submission() {
        let dir = tempdir().expect("tempdir");
        let (artifact_path, abi_path) = write_target_inputs(&dir, Value::Null, json!([]));
        let (schema_path, proof_path, signed_bundle_path) = write_evidence(&dir);
        let mut signed_bundle: Value =
            serde_json::from_slice(&fs::read(&signed_bundle_path).expect("read signed bundle"))
                .expect("parse signed bundle");
        signed_bundle["payload"]["compiler"]["version"] = json!("drifted");
        fs::write(&signed_bundle_path, canonical_json_bytes(&signed_bundle))
            .expect("write drifted signed bundle");

        let artifact = load_artifact(&artifact_path, TARGET_PROFILE).expect("load artifact");
        let abi = load_wire_abi(&abi_path, TARGET_PROFILE).expect("load ABI");
        let error = load_evidence(
            &schema_path,
            &proof_path,
            &signed_bundle_path,
            &artifact,
            &abi,
        )
        .expect_err("identity drift must fail closed");
        assert!(error.to_string().contains("cross-artifact identity drift"));
    }

    #[test]
    fn caller_authorization_descriptor_drift_is_rejected_before_submission() {
        let authorization = json!({
            "entry_symbol": "clg.caller.entry",
            "identity": { "encoding": "evm-address-20-byte", "width_bits": 160 },
            "owner_field_id": "sha256:owner",
            "profile": "clg.evm-stateful-scalar.v1",
            "transitions": [{ "function": "set_value", "guard": "caller_equals_state_owner" }],
        });
        let schema = json!({ "caller_authorization": authorization });
        let artifact = json!({
            "state_mapping": { "caller_authorization": schema["caller_authorization"] },
        });
        let abi = json!({ "caller_authorization": schema["caller_authorization"] });
        validate_caller_authorization_binding(&schema, &artifact, &abi)
            .expect("matching caller descriptor");
        let proof = json!({ "caller_authorization": schema["caller_authorization"] });
        let signed_payload = json!({ "caller_authorization": schema["caller_authorization"] });
        validate_caller_authorization_evidence(&schema, &proof, &signed_payload)
            .expect("matching caller evidence");

        let mut drifted_artifact = artifact.clone();
        drifted_artifact["state_mapping"]["caller_authorization"]["owner_field_id"] =
            json!("sha256:other-owner");
        let error = validate_caller_authorization_binding(&schema, &drifted_artifact, &abi)
            .expect_err("artifact caller drift must fail closed");
        assert!(error.to_string().contains("caller-authorization drift"));

        let mut drifted_abi = abi.clone();
        drifted_abi["caller_authorization"]["identity"]["width_bits"] = json!(256);
        let error = validate_caller_authorization_binding(&schema, &artifact, &drifted_abi)
            .expect_err("wire ABI caller drift must fail closed");
        assert!(error.to_string().contains("caller-authorization drift"));

        let mut drifted_proof = proof.clone();
        drifted_proof["caller_authorization"]["transitions"][0]["function"] = json!("other");
        let error =
            validate_caller_authorization_evidence(&schema, &drifted_proof, &signed_payload)
                .expect_err("proof caller drift must fail closed");
        assert!(error.to_string().contains("caller-authorization drift"));

        let mut drifted_signed_payload = signed_payload.clone();
        drifted_signed_payload["caller_authorization"]["owner_field_id"] = json!("sha256:other");
        let error =
            validate_caller_authorization_evidence(&schema, &proof, &drifted_signed_payload)
                .expect_err("signed caller drift must fail closed");
        assert!(error.to_string().contains("caller-authorization drift"));

        let no_authorization_schema = json!({ "caller_authorization": null });
        let error =
            validate_caller_authorization_binding(&no_authorization_schema, &artifact, &abi)
                .expect_err("injected caller authorization must fail closed");
        assert!(error.to_string().contains("caller-authorization drift"));
    }
}
