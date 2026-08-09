use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use clg_ast::{Effect, Program};
use serde_json::json;

use super::contract_state_schema::{contract_state_schema_artifact, render_type};
use crate::commands::helpers::{canonical_json_bytes, sha256_hex};

const ABI_FORMAT: &str = "clg.contract-abi.v1";
const ABI_SCHEMA_VERSION: u32 = 1;
const ABI_ID_DOMAIN: &str = "clg.contract-abi-function.v1";
const ABI_PROFILE: &str = "clg.evm-compatible.abi.v1";

pub(super) fn write_contract_abi(
    program: &Program,
    path: &Path,
    source_graph: Option<&serde_json::Value>,
) -> Result<()> {
    let artifact = contract_abi_artifact(program, source_graph)?;
    fs::write(path, canonical_json_bytes(&artifact))
        .with_context(|| format!("write contract ABI `{}`", path.display()))
}

fn contract_abi_artifact(
    program: &Program,
    source_graph: Option<&serde_json::Value>,
) -> Result<serde_json::Value> {
    let contract = match program.contracts.as_slice() {
        [contract] => contract,
        [] => bail!("--emit-contract-abi requires exactly one `contract` declaration"),
        _ => bail!("--emit-contract-abi does not support multiple `contract` declarations"),
    };
    let state_schema = contract_state_schema_artifact(program, None)?;
    let functions = contract
        .functions
        .iter()
        .enumerate()
        .map(|(index, function)| {
            let params = function
                .params
                .iter()
                .map(|param| json!({ "name": param.name, "type": render_type(&param.ty) }))
                .collect::<Vec<_>>();
            let signature = format!(
                "{}({})->{}",
                function.name,
                params
                    .iter()
                    .map(|param| param["type"].as_str().expect("ABI parameter type"))
                    .collect::<Vec<_>>()
                    .join(","),
                render_type(&function.ret)
            );
            let id_input = format!(
                "{ABI_ID_DOMAIN}\0{}\0{}\0{signature}",
                contract.name, contract.version
            );
            json!({
                "effect": effect_name(function.effect),
                "function_id": format!("sha256:{}", sha256_hex(id_input.as_bytes())),
                "index": index,
                "name": function.name,
                "params": params,
                "return": render_type(&function.ret),
                "signature": signature,
            })
        })
        .collect::<Vec<_>>();
    let identity = json!({
        "algorithm": ABI_FORMAT,
        "contract": { "name": contract.name, "version": contract.version },
        "errors": [],
        "events": state_schema["events"].clone(),
        "functions": functions,
        "state_schema": {
            "digest": state_schema["schema"]["digest"].clone(),
            "event_abi_digest": state_schema["event_abi"]["digest"].clone(),
            "layout_digest": state_schema["layout"]["digest"].clone(),
        },
        "target": { "profile": ABI_PROFILE },
    });
    let digest = format!("sha256:{}", sha256_hex(canonical_json_bytes(&identity).as_slice()));
    Ok(json!({
        "abi": {
            "digest": digest,
            "format": ABI_FORMAT,
        },
        "compiler": {
            "name": "clg-cli",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "contract": identity["contract"].clone(),
        "errors": {
            "items": identity["errors"].clone(),
            "status": "unsupported",
        },
        "events": identity["events"].clone(),
        "functions": identity["functions"].clone(),
        "schema_version": ABI_SCHEMA_VERSION,
        "source_graph": source_graph,
        "state_schema": identity["state_schema"].clone(),
        "target": {
            "profile": ABI_PROFILE,
            "selector_derivation": "deferred",
            "wire_encoding": "deferred",
        },
    }))
}

fn effect_name(effect: Effect) -> &'static str {
    match effect {
        Effect::None => "none",
        Effect::Pure => "pure",
        Effect::Mut => "mut",
        Effect::Io => "io",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clg_parser::parse;

    #[test]
    fn abi_is_byte_stable_and_links_contract_schema_identities() {
        let program = parse(
            r#"
                contract Vault version 1 {
                    state { total: U64; }
                    event Deposited { amount: U64; }
                    pure function balance() -> U64 { U64(0) }
                    mut function deposit(amount: U64) -> U64 { amount }
                }
            "#,
        )
        .expect("parse contract ABI fixture");
        let first = contract_abi_artifact(&program, None).expect("first ABI");
        let second = contract_abi_artifact(&program, None).expect("second ABI");
        assert_eq!(canonical_json_bytes(&first), canonical_json_bytes(&second));
        assert_eq!(first["functions"][0]["signature"], "balance()->U64");
        assert_eq!(first["functions"][1]["effect"], "mut");
        assert_eq!(first["events"][0]["name"], "Deposited");
        assert_eq!(first["errors"]["status"], "unsupported");
        assert_eq!(first["target"]["wire_encoding"], "deferred");
        assert_eq!(
            first["state_schema"]["digest"],
            contract_state_schema_artifact(&program, None).expect("state schema")["schema"]["digest"]
        );
    }
}
