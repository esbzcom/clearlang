use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use clg_ast::{Param, Program, Type};
use serde_json::json;
use sha3::{Digest, Keccak256};

use crate::commands::helpers::{canonical_json_bytes, sha256_hex};

const FORMAT: &str = "clg.evm-wire-abi.v1";
const PROFILE: &str = "clg.evm-compatible.v1";

pub(super) fn write_evm_wire_abi(
    program: &Program,
    path: &Path,
    source_graph: Option<&serde_json::Value>,
) -> Result<()> {
    let artifact = evm_wire_abi_artifact(program, source_graph)?;
    fs::write(path, canonical_json_bytes(&artifact))
        .with_context(|| format!("write EVM wire ABI `{}`", path.display()))
}

pub(super) fn evm_wire_abi_artifact(
    program: &Program,
    source_graph: Option<&serde_json::Value>,
) -> Result<serde_json::Value> {
    let contract = match program.contracts.as_slice() {
        [contract] => contract,
        [] => bail!("--emit-evm-wire-abi requires exactly one `contract` declaration"),
        _ => bail!("--emit-evm-wire-abi does not support multiple `contract` declarations"),
    };
    let constructor = contract.init.as_ref().map(|init| {
        let inputs = wire_params(&init.params)?;
        Ok::<serde_json::Value, anyhow::Error>(json!({ "inputs": inputs }))
    }).transpose()?;
    let functions = contract
        .functions
        .iter()
        .map(|function| {
            let inputs = wire_params(&function.params)?;
            let output = evm_type(&function.ret)?;
            let signature = format!(
                "{}({})",
                function.name,
                inputs
                    .iter()
                    .map(|input| input["evm_type"].as_str().expect("wire type"))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            Ok(json!({
                "calldata": "abi-v2",
                "inputs": inputs,
                "name": function.name,
                "outputs": [{ "evm_type": output, "name": "result" }],
                "selector": selector(&signature),
                "signature": signature,
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    let events = contract
        .events
        .iter()
        .map(|event| {
            let inputs = event
                .fields
                .iter()
                .map(|field| Ok(json!({
                    "evm_type": evm_type(&field.ty)?,
                    "indexed": false,
                    "name": field.name,
                })))
                .collect::<Result<Vec<_>>>()?;
            let signature = format!(
                "{}({})",
                event.name,
                inputs
                    .iter()
                    .map(|input| input["evm_type"].as_str().expect("wire type"))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            Ok(json!({
                "inputs": inputs,
                "name": event.name,
                "signature": signature,
                "topic0": hash(&signature),
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    let identity = json!({
        "constructor": constructor,
        "contract": { "name": contract.name, "version": contract.version },
        "events": events,
        "functions": functions,
        "target": { "calldata_encoding": "abi-v2", "profile": PROFILE },
    });
    Ok(json!({
        "compiler": { "name": "clg-cli", "version": env!("CARGO_PKG_VERSION") },
        "constructor": identity["constructor"].clone(),
        "contract": identity["contract"].clone(),
        "events": identity["events"].clone(),
        "functions": identity["functions"].clone(),
        "schema_version": 1,
        "source_graph": source_graph,
        "target": {
            "bytecode": "deferred",
            "calldata_encoding": "abi-v2",
            "profile": PROFILE,
        },
        "wire_abi": {
            "digest": format!("sha256:{}", sha256_hex(&canonical_json_bytes(&identity))),
            "format": FORMAT,
        },
    }))
}

fn wire_params(params: &[Param]) -> Result<Vec<serde_json::Value>> {
    params
        .iter()
        .map(|param| Ok(json!({ "evm_type": evm_type(&param.ty)?, "name": param.name })))
        .collect()
}

fn evm_type(ty: &Type) -> Result<&'static str> {
    match ty {
        Type::Int => Ok("int256"),
        Type::U8 => Ok("uint8"),
        Type::U64 => Ok("uint64"),
        Type::U128 => Ok("uint128"),
        Type::U256 => Ok("uint256"),
        Type::Bool => Ok("bool"),
        Type::String => Ok("string"),
        Type::Bytes => Ok("bytes"),
        _ => bail!(
            "EVM wire ABI does not support ClearLang type `{}` in the first target profile",
            render_type(ty)
        ),
    }
}

fn selector(signature: &str) -> String {
    format!("0x{}", &hash(signature)[2..10])
}

fn hash(input: &str) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(input.as_bytes());
    format!("0x{}", hex::encode(hasher.finalize()))
}

fn render_type(ty: &Type) -> String {
    format!("{ty:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clg_parser::parse;

    #[test]
    fn emits_stable_selectors_and_event_topics() {
        let program = parse(
            r#"
                contract Vault version 1 {
                    state { total: U64; }
                    event Deposited { amount: U64; }
                    mut function deposit(amount: U64) -> Bool { true }
                }
            "#,
        )
        .expect("parse wire ABI fixture");
        let first = evm_wire_abi_artifact(&program, None).expect("first artifact");
        let second = evm_wire_abi_artifact(&program, None).expect("second artifact");
        assert_eq!(canonical_json_bytes(&first), canonical_json_bytes(&second));
        assert_eq!(first["functions"][0]["signature"], "deposit(uint64)");
        assert_eq!(first["functions"][0]["selector"], "0x13765838");
        assert_eq!(first["events"][0]["topic0"], "0x7f2ef186ad31df7b1c02573d76e20029546e02b4fd1e8a696b110783d5834793");
        assert_eq!(first["target"]["bytecode"], "deferred");
    }
}
