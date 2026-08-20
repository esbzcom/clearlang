//! Explicit, narrow EVM-compatible target adapter.
//!
//! The adapter never discovers an endpoint, account, or network.  `call` is operational for the
//! first static scalar ABI profile. Deploy and invoke submit only explicit, locally signed
//! EIP-155 transactions and write receipts only after confirmed on-chain success.

use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

use crate::commands::helpers::{canonical_json_bytes, sha256_hex};

mod transaction;

const TARGET_PROFILE: &str = "clg.evm-compatible.v1";
const WIRE_ABI_FORMAT: &str = "clg.evm-wire-abi.v1";
const RECEIPT_FORMAT: &str = "clg.target-receipt.v1";
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

pub fn deploy(args: DeployArgs) -> Result<()> {
    let endpoint = preflight(&args.target_profile, &args.rpc_url, args.chain_id)?;
    let artifact = load_artifact(&args.artifact, &args.target_profile)?;
    let abi = load_wire_abi(&args.abi, &args.target_profile)?;
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
    )
}

pub fn call(args: CallArgs) -> Result<()> {
    let endpoint = preflight(&args.target_profile, &args.rpc_url, args.chain_id)?;
    let artifact = load_artifact(&args.artifact, &args.target_profile)?;
    let abi = load_wire_abi(&args.abi, &args.target_profile)?;
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
    )
}

struct LoadedArtifact {
    digest: String,
    bytecode: Vec<u8>,
}

struct LoadedWireAbi {
    digest: String,
    value: Value,
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
    Ok(LoadedArtifact {
        digest: format!("sha256:{}", sha256_hex(&bytes)),
        bytecode,
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
    Ok(LoadedWireAbi { digest, value })
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
) -> Result<()> {
    let request_digest = format!("sha256:{}", sha256_hex(&canonical_json_bytes(&request)));
    let mut receipt = json!({
        "artifact_digest": artifact.digest,
        "block": { "hash": onchain.block_hash, "number": onchain.block_number },
        "chain_id": chain_id,
        "command": command,
        "format": RECEIPT_FORMAT,
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
    fs::write(output, canonical_json_bytes(&receipt))
        .with_context(|| format!("write target receipt `{}`", output.display()))
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
    if chain_id == 0 {
        bail!("chain ID must be a non-zero explicit numeric identifier")
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
        fs::write(
            &artifact,
            canonical_json_bytes(&json!({
                "bytecode": "0x6000",
                "target": { "profile": TARGET_PROFILE },
            })),
        )
        .expect("write artifact");
        fs::write(
            &abi,
            canonical_json_bytes(&json!({
                "constructor": constructor,
                "functions": functions,
                "target": { "profile": TARGET_PROFILE },
                "wire_abi": { "digest": "sha256:test", "format": WIRE_ABI_FORMAT },
            })),
        )
        .expect("write ABI");
        (artifact, abi)
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
        assert!(preflight(TARGET_PROFILE, "https://rpc.example.invalid", 1).is_ok());
        assert!(preflight("other", "https://rpc.example.invalid", 1)
            .unwrap_err()
            .to_string()
            .contains("unsupported target profile"));
        assert!(preflight(TARGET_PROFILE, "file:///tmp/rpc", 1)
            .unwrap_err()
            .to_string()
            .contains("http:// or https://"));
        assert!(preflight(TARGET_PROFILE, "https://rpc.example.invalid", 0)
            .unwrap_err()
            .to_string()
            .contains("non-zero"));
    }

    #[test]
    fn call_checks_chain_id_submits_calldata_and_writes_a_canonical_receipt() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock RPC");
        let address = listener.local_addr().expect("mock RPC address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [
                json!({"jsonrpc":"2.0", "id":1, "result":"0x1"}),
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
        fs::write(
            &artifact,
            canonical_json_bytes(&json!({
                "bytecode": "0x6000",
                "target": { "profile": TARGET_PROFILE },
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
                "target": { "profile": TARGET_PROFILE },
                "wire_abi": { "digest": "sha256:test", "format": WIRE_ABI_FORMAT },
            })),
        )
        .expect("write ABI");
        fs::write(&args_path, br#"{"amount":7}"#).expect("write args");

        call(CallArgs {
            target_profile: TARGET_PROFILE.to_string(),
            rpc_url: format!("http://{address}"),
            chain_id: 1,
            artifact,
            abi: abi_path,
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
        assert_eq!(receipt["chain_id"], 1);
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
            Some("eth_chainId") => json!("0x1"),
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
        fs::write(&args, br#"{"initial":7}"#).expect("write arguments");

        deploy(DeployArgs {
            target_profile: TARGET_PROFILE.to_string(),
            rpc_url: endpoint,
            chain_id: 1,
            sender: "0x1a642f0e3c3af545e7acbd38b07251b3990914f1".to_string(),
            nonce: 0,
            signing_key: write_signer(&dir),
            value: 0,
            gas_limit: 100_000,
            gas_price: 1,
            artifact,
            abi,
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
            Some("eth_chainId") => json!("0x1"),
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
        fs::write(&args, b"[]").expect("write arguments");

        invoke(InvokeArgs {
            target_profile: TARGET_PROFILE.to_string(),
            rpc_url: endpoint,
            chain_id: 1,
            artifact,
            abi,
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
}
