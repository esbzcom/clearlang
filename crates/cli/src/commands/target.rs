//! Explicit, narrow EVM-compatible target adapter.
//!
//! The adapter never discovers an endpoint, account, or network.  `call` is operational for the
//! first static scalar ABI profile; deployment and signed invocation remain fail-closed until a
//! source-to-EVM backend and transaction signer are available.

use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

use crate::commands::helpers::{canonical_json_bytes, sha256_hex};

const TARGET_PROFILE: &str = "clg.evm-compatible.v1";
const WIRE_ABI_FORMAT: &str = "clg.evm-wire-abi.v1";
const RECEIPT_FORMAT: &str = "clg.target-receipt.v1";

#[derive(Debug)]
pub struct DeployArgs {
    pub target_profile: String,
    pub rpc_url: String,
    pub chain_id: u64,
    pub artifact: PathBuf,
    pub abi: PathBuf,
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
    pub signing_key: PathBuf,
    pub value: u64,
    pub gas_limit: u64,
    pub receipt_out: PathBuf,
}

pub fn deploy(args: DeployArgs) -> Result<()> {
    let endpoint = preflight(&args.target_profile, &args.rpc_url, args.chain_id)?;
    let _artifact = load_artifact(&args.artifact, &args.target_profile)?;
    let _abi = load_wire_abi(&args.abi, &args.target_profile)?;
    let _ = (&endpoint, &args.receipt_out);
    bail!("target deploy is unavailable: ClearLang has no explicit EVM deploy transaction signer; no RPC submission was made and no target receipt was written")
}

pub fn call(args: CallArgs) -> Result<()> {
    let endpoint = preflight(&args.target_profile, &args.rpc_url, args.chain_id)?;
    let artifact = load_artifact(&args.artifact, &args.target_profile)?;
    let abi = load_wire_abi(&args.abi, &args.target_profile)?;
    let input = read_json(&args.args, "call arguments")?;
    let calldata = encode_calldata(&abi.value, &args.function, &input)?;

    let observed_chain = rpc(&endpoint, "eth_chainId", json!([]))?;
    let observed_chain =
        parse_quantity(&observed_chain).context("malformed eth_chainId response")?;
    if observed_chain != args.chain_id {
        bail!(
            "RPC chain-ID mismatch: requested {}, endpoint reported {}",
            args.chain_id,
            observed_chain
        );
    }
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
    let _artifact = load_artifact(&args.artifact, &args.target_profile)?;
    let abi = load_wire_abi(&args.abi, &args.target_profile)?;
    let input = read_json(&args.args, "invoke arguments")?;
    let _calldata = encode_calldata(&abi.value, &args.function, &input)?;
    let _ = (
        &endpoint,
        &args.contract_address,
        &args.sender,
        &args.signing_key,
        args.value,
        args.gas_limit,
        &args.receipt_out,
    );
    bail!("target invoke is unavailable: ClearLang has no explicit EVM transaction signer; no RPC submission was made and no target receipt was written")
}

struct LoadedArtifact {
    digest: String,
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
    valid_hex_bytes(bytecode).context("target artifact has malformed EVM bytecode")?;
    if bytecode == "0x" {
        bail!("target artifact has empty EVM bytecode");
    }
    Ok(LoadedArtifact {
        digest: format!("sha256:{}", sha256_hex(&bytes)),
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
                    .ok_or_else(|| anyhow!("missing call argument `{name}`"))
            })
            .collect::<Result<Vec<_>>>()?,
        _ => bail!("call arguments must be a JSON array or object"),
    };
    if values.len() != inputs.len() {
        bail!("call argument count does not match wire ABI");
    }
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
}
