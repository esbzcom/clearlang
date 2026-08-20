use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use clg_ast::{Effect, Expr, Func, Program, Type};
use serde_json::json;

use super::evm_wire_abi::evm_wire_abi_artifact;
use crate::commands::helpers::{canonical_json_bytes, sha256_hex};

const FORMAT: &str = "clg.evm-artifact.v1";
const PROFILE: &str = "clg.evm-compatible.v1";
const EXECUTION_PROFILE: &str = "clg.evm-static-pure.v1";

pub(super) fn write_evm_artifact(
    program: &Program,
    path: &Path,
    source_graph: Option<&serde_json::Value>,
) -> Result<()> {
    let artifact = evm_artifact(program, source_graph)?;
    fs::write(path, canonical_json_bytes(&artifact))
        .with_context(|| format!("write EVM artifact `{}`", path.display()))
}

fn evm_artifact(program: &Program, source_graph: Option<&serde_json::Value>) -> Result<serde_json::Value> {
    let contract = match program.contracts.as_slice() {
        [contract] => contract,
        [] => bail!("--emit-evm-artifact requires exactly one `contract` declaration"),
        _ => bail!("--emit-evm-artifact does not support multiple `contract` declarations"),
    };
    if contract.init.is_some() {
        bail!("EVM static-pure profile does not support `init`; constructor bytecode is fail-closed")
    }
    let wire_abi = evm_wire_abi_artifact(program, source_graph)?;
    let wire_functions = wire_abi["functions"]
        .as_array()
        .expect("wire ABI always has function array");
    let handlers = contract
        .functions
        .iter()
        .zip(wire_functions)
        .map(|(function, descriptor)| {
            let selector = descriptor["selector"]
                .as_str()
                .expect("wire ABI function selector");
            let constant = compile_static_pure_function(function)?;
            Ok((selector, constant))
        })
        .collect::<Result<Vec<_>>>()?;
    if handlers.is_empty() {
        bail!("EVM static-pure profile requires at least one contract function")
    }
    let runtime = emit_runtime(&handlers)?;
    let init_code = wrap_init_code(&runtime)?;
    let identity = json!({
        "bytecode": format!("0x{}", hex::encode(&init_code)),
        "contract": { "name": contract.name, "version": contract.version },
        "execution_profile": EXECUTION_PROFILE,
        "target_profile": PROFILE,
        "wire_abi_digest": wire_abi["wire_abi"]["digest"].clone(),
    });
    Ok(json!({
        "artifact": {
            "digest": format!("sha256:{}", sha256_hex(&canonical_json_bytes(&identity))),
            "format": FORMAT,
        },
        "bytecode": identity["bytecode"].clone(),
        "compiler": { "name": "clg-cli", "version": env!("CARGO_PKG_VERSION") },
        "contract": identity["contract"].clone(),
        "execution_profile": EXECUTION_PROFILE,
        "schema_version": 1,
        "source_graph": source_graph,
        "target": { "profile": PROFILE },
        "wire_abi": { "digest": identity["wire_abi_digest"].clone(), "format": "clg.evm-wire-abi.v1" },
    }))
}

fn compile_static_pure_function(function: &Func) -> Result<[u8; 32]> {
    if function.effect != Effect::Pure {
        bail!("EVM static-pure profile requires `pure` function `{}`", function.name)
    }
    if !function.params.is_empty() || !function.type_params.is_empty() {
        bail!("EVM static-pure profile requires zero-parameter, non-generic function `{}`", function.name)
    }
    let value = literal_value(&function.body)?;
    let mut word = [0_u8; 32];
    match function.ret {
        Type::Bool => {
            if value != 0 && value != 1 {
                bail!("Boolean function `{}` must return true or false", function.name)
            }
            word[31] = value as u8;
        }
        Type::U8 => {
            if !(0..=u8::MAX as i128).contains(&value) { bail!("U8 function `{}` returned an out-of-range literal", function.name); }
            word[31] = value as u8;
        }
        Type::U64 => {
            if !(0..=u64::MAX as i128).contains(&value) { bail!("U64 function `{}` returned an out-of-range literal", function.name); }
            word[24..].copy_from_slice(&(value as u64).to_be_bytes());
        }
        Type::U128 => {
            if value < 0 { bail!("U128 function `{}` returned a negative literal", function.name); }
            word[16..].copy_from_slice(&(value as u128).to_be_bytes());
        }
        Type::Int => {
            let value = i64::try_from(value).map_err(|_| anyhow::anyhow!("Int literal is out of range"))?;
            if value < 0 { word.fill(0xff); }
            word[24..].copy_from_slice(&value.to_be_bytes());
        }
        _ => bail!("EVM static-pure profile does not support return type in function `{}`", function.name),
    }
    Ok(word)
}

fn literal_value(expr: &Expr) -> Result<i128> {
    match expr {
        Expr::Int(value, _) => Ok((*value).into()),
        Expr::Bool(value, _) => Ok(i128::from(*value)),
        Expr::Call { callee, args, .. } if matches!(callee.as_str(), "U8" | "U64" | "U128" | "Int") && args.len() == 1 => literal_value(&args[0]),
        Expr::Block { block } => block
            .tail
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("EVM static-pure profile requires a literal tail expression"))
            .and_then(literal_value),
        _ => bail!("EVM static-pure profile requires a literal Bool/Int/U8/U64/U128 result"),
    }
}

fn emit_runtime(handlers: &[(impl AsRef<str>, [u8; 32])]) -> Result<Vec<u8>> {
    let dispatch_len = 6usize
        .checked_add(handlers.len().checked_mul(11).ok_or_else(|| anyhow::anyhow!("too many EVM handlers"))?)
        .and_then(|value| value.checked_add(5))
        .ok_or_else(|| anyhow::anyhow!("EVM runtime is too large"))?;
    let mut runtime = vec![0x60, 0x00, 0x35, 0x60, 0xe0, 0x1c]; // calldataload(0) >> 224
    for (index, (selector, _)) in handlers.iter().enumerate() {
        let selector = selector.as_ref().strip_prefix("0x").expect("wire selector prefix");
        let selector = hex::decode(selector).expect("wire selector hex");
        let destination = dispatch_len + index * 42;
        if destination > u16::MAX as usize { bail!("EVM runtime is too large") }
        runtime.extend([0x80, 0x63]); // DUP1 PUSH4
        runtime.extend(selector);
        runtime.extend([0x14, 0x61, (destination >> 8) as u8, destination as u8, 0x57]); // EQ PUSH2 JUMPI
    }
    runtime.extend([0x60, 0x00, 0x60, 0x00, 0xfd]); // revert unknown selector
    for (_, constant) in handlers {
        runtime.push(0x5b); // JUMPDEST
        runtime.push(0x7f); // PUSH32
        runtime.extend(constant);
        runtime.extend([0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3]); // MSTORE; return 32 bytes
    }
    Ok(runtime)
}

fn wrap_init_code(runtime: &[u8]) -> Result<Vec<u8>> {
    if runtime.len() > u16::MAX as usize { bail!("EVM runtime is too large") }
    let runtime_len = runtime.len() as u16;
    let offset = 15_u16;
    let mut init = vec![
        0x61, (runtime_len >> 8) as u8, runtime_len as u8,
        0x61, (offset >> 8) as u8, offset as u8,
        0x60, 0x00, 0x39,
        0x61, (runtime_len >> 8) as u8, runtime_len as u8,
        0x60, 0x00, 0xf3,
    ];
    init.extend(runtime);
    Ok(init)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clg_parser::parse;

    #[test]
    fn emits_stable_deployable_bytecode_for_static_pure_contracts() {
        let program = parse(r#"
            contract Constant version 1 {
                state { total: U64; }
                pure function answer() -> U64 { U64(42) }
            }
        "#).expect("parse source");
        let first = evm_artifact(&program, None).expect("first artifact");
        let second = evm_artifact(&program, None).expect("second artifact");
        assert_eq!(canonical_json_bytes(&first), canonical_json_bytes(&second));
        assert_eq!(first["artifact"]["format"], FORMAT);
        assert_eq!(first["target"]["profile"], PROFILE);
        assert!(first["bytecode"].as_str().expect("bytecode").len() > 2);
    }

    #[test]
    fn rejects_non_literal_or_non_pure_functions() {
        let program = parse(r#"
            contract Constant version 1 {
                state { total: U64; }
                mut function answer() -> U64 { U64(42) }
            }
        "#).expect("parse source");
        assert!(evm_artifact(&program, None).unwrap_err().to_string().contains("requires `pure`"));
    }
}
