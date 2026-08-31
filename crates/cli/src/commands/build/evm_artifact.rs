use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use clg_ast::{ContractDecl, Effect, Expr, Func, Program, Stmt, Type};
use clg_ir::{Function as IrFunction, Instr, Module as IrModule, Value};
use serde_json::{json, Value as JsonValue};
use sha3::{Digest, Keccak256};

use super::contract_state_schema::contract_state_schema_artifact;
use super::evm_wire_abi::evm_wire_abi_artifact;
use crate::commands::helpers::{canonical_json_bytes, sha256_hex};

const FORMAT: &str = "clg.evm-artifact.v1";
const PROFILE: &str = "clg.evm-compatible.v1";
const STATIC_PROFILE: &str = "clg.evm-static-pure.v1";
const STATEFUL_PROFILE: &str = "clg.evm-stateful-scalar.v1";
const STORAGE_SLOT_DOMAIN: &str = "clg.evm-stateful-scalar-slot.v1\0";
const VALUE_MEMORY_BASE: u16 = 0x0400;

pub(super) fn write_evm_artifact(
    program: &Program,
    ir: &IrModule,
    path: &Path,
    source_graph: Option<&JsonValue>,
) -> Result<()> {
    let artifact = evm_artifact(program, ir, source_graph)?;
    fs::write(path, canonical_json_bytes(&artifact))
        .with_context(|| format!("write EVM artifact `{}`", path.display()))
}

fn evm_artifact(
    program: &Program,
    ir: &IrModule,
    source_graph: Option<&JsonValue>,
) -> Result<JsonValue> {
    let contract = single_contract(program)?;
    let wire_abi = evm_wire_abi_artifact(program, source_graph)?;
    let stateful = contract.init.is_some()
        || contract.functions.iter().any(|function| function.effect == Effect::Mut)
        || contract
            .functions
            .iter()
            .any(|function| function_reads_state(&function.body));
    if stateful {
        stateful_artifact(program, ir, contract, &wire_abi, source_graph)
    } else {
        static_artifact(contract, &wire_abi, source_graph)
    }
}

fn static_artifact(
    contract: &ContractDecl,
    wire_abi: &JsonValue,
    source_graph: Option<&JsonValue>,
) -> Result<JsonValue> {
    let descriptors = wire_abi["functions"]
        .as_array()
        .expect("wire ABI always has functions");
    let handlers = contract
        .functions
        .iter()
        .zip(descriptors)
        .map(|(function, descriptor)| {
            Ok(Handler {
                selector: descriptor["selector"]
                    .as_str()
                    .expect("wire ABI selector")
                    .to_string(),
                code: static_handler(compile_static_pure_function(function)?),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if handlers.is_empty() {
        bail!("EVM static-pure profile requires at least one contract function")
    }
    artifact_value(
        contract,
        wire_abi,
        source_graph,
        STATIC_PROFILE,
        wrap_init_code(&emit_runtime(&handlers)?)?,
        None,
        None,
    )
}

fn stateful_artifact(
    program: &Program,
    ir: &IrModule,
    contract: &ContractDecl,
    wire_abi: &JsonValue,
    source_graph: Option<&JsonValue>,
) -> Result<JsonValue> {
    let schema = contract_state_schema_artifact(program, source_graph)?;
    let storage = storage_layout(&schema)?;
    ensure_scalar_events(contract)?;
    let events = event_topics(wire_abi)?;
    let descriptors = wire_abi["functions"]
        .as_array()
        .expect("wire ABI always has functions");
    if contract.functions.len() != descriptors.len() {
        bail!("wire ABI function count does not match contract")
    }
    let mut mappings = Vec::new();
    let mut handlers = Vec::new();
    for (function, descriptor) in contract.functions.iter().zip(descriptors) {
        ensure_scalar_signature(function)?;
        let lowered = find_lowered_function(ir, &function.name)?;
        let (code, operations) = compile_transition(lowered, function.params.len(), &storage, &events, false)?;
        handlers.push(Handler {
            selector: descriptor["selector"]
                .as_str()
                .expect("wire ABI selector")
                .to_string(),
            code,
        });
        mappings.push(json!({ "function": function.name, "operations": operations }));
    }
    if handlers.is_empty() {
        bail!("EVM stateful scalar profile requires at least one contract function")
    }
    let constructor = match &contract.init {
        Some(init) => {
            ensure_scalar_params(&init.params)?;
            let lowered = find_lowered_function(ir, &format!("__clg_contract_init${}", contract.name))?;
            let (code, operations) = compile_transition(lowered, init.params.len(), &storage, &events, true)?;
            mappings.push(json!({ "function": format!("{}::init", contract.name), "operations": operations }));
            constructor_parameter_loader(init.params.len())?
                .into_iter()
                .chain(code)
                .collect()
        }
        None if contract.fields.is_empty() => Vec::new(),
        None => bail!("EVM stateful scalar profile requires a direct `init` for every state field"),
    };
    let runtime = emit_runtime(&handlers)?;
    let bytecode = wrap_stateful_init_code(&constructor, &runtime)?;
    let state_mapping = json!({
        "schema_digest": schema["schema"]["digest"].clone(),
        "storage_layout": storage.values().map(|field| json!({
            "field": field.name,
            "field_id": field.field_id,
            "slot": field.slot,
            "type": field.ty,
        })).collect::<Vec<_>>(),
        "transition_mapping": mappings,
    });
    artifact_value(
        contract,
        wire_abi,
        source_graph,
        STATEFUL_PROFILE,
        bytecode,
        Some(state_mapping),
        Some(schema["schema"]["digest"].clone()),
    )
}

fn artifact_value(
    contract: &ContractDecl,
    wire_abi: &JsonValue,
    source_graph: Option<&JsonValue>,
    execution_profile: &str,
    bytecode: Vec<u8>,
    state_mapping: Option<JsonValue>,
    state_schema_digest: Option<JsonValue>,
) -> Result<JsonValue> {
    let identity = json!({
        "bytecode": format!("0x{}", hex::encode(&bytecode)),
        "contract": { "name": contract.name, "version": contract.version },
        "execution_profile": execution_profile,
        "state_mapping": state_mapping,
        "state_schema_digest": state_schema_digest,
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
        "execution_profile": execution_profile,
        "schema_version": 1,
        "source_graph": source_graph,
        "state_mapping": identity["state_mapping"].clone(),
        "state_schema": { "digest": identity["state_schema_digest"].clone() },
        "target": { "profile": PROFILE },
        "wire_abi": { "digest": identity["wire_abi_digest"].clone(), "format": "clg.evm-wire-abi.v1" },
    }))
}

fn single_contract(program: &Program) -> Result<&ContractDecl> {
    match program.contracts.as_slice() {
        [contract] => Ok(contract),
        [] => bail!("--emit-evm-artifact requires exactly one `contract` declaration"),
        _ => bail!("--emit-evm-artifact does not support multiple `contract` declarations"),
    }
}

#[derive(Clone)]
struct StorageField {
    name: String,
    field_id: String,
    slot: String,
    slot_bytes: [u8; 32],
    ty: String,
}

fn storage_layout(schema: &JsonValue) -> Result<BTreeMap<String, StorageField>> {
    schema["fields"]
        .as_array()
        .ok_or_else(|| anyhow!("generated state schema is missing fields"))?
        .iter()
        .map(|field| {
            let name = field["name"].as_str().ok_or_else(|| anyhow!("malformed state field name"))?.to_string();
            let field_id = field["field_id"].as_str().ok_or_else(|| anyhow!("malformed state field ID"))?.to_string();
            let ty = field["type"].as_str().ok_or_else(|| anyhow!("malformed state field type"))?.to_string();
            if !matches!(ty.as_str(), "Bool" | "U8" | "U64" | "U128" | "Int") {
                bail!("EVM stateful scalar profile does not support state field `{name}` of type `{ty}`")
            }
            let mut hasher = Keccak256::new();
            hasher.update(STORAGE_SLOT_DOMAIN.as_bytes());
            hasher.update(field_id.as_bytes());
            let slot_bytes: [u8; 32] = hasher.finalize().into();
            Ok((name.clone(), StorageField { name, field_id, slot: format!("0x{}", hex::encode(slot_bytes)), slot_bytes, ty }))
        })
        .collect()
}

fn event_topics(wire_abi: &JsonValue) -> Result<BTreeMap<String, [u8; 32]>> {
    wire_abi["events"]
        .as_array()
        .ok_or_else(|| anyhow!("wire ABI is missing events"))?
        .iter()
        .map(|event| {
            let name = event["name"].as_str().ok_or_else(|| anyhow!("malformed event name"))?.to_string();
            let topic = event["topic0"].as_str().ok_or_else(|| anyhow!("malformed event topic"))?;
            let bytes = decode_fixed_32(topic).context("malformed event topic")?;
            Ok((name, bytes))
        })
        .collect()
}

fn ensure_scalar_signature(function: &Func) -> Result<()> {
    ensure_scalar_params(&function.params)?;
    if !scalar_type(&function.ret) {
        bail!("EVM stateful scalar profile does not support return type in function `{}`", function.name)
    }
    if !function.type_params.is_empty() {
        bail!("EVM stateful scalar profile does not support generic function `{}`", function.name)
    }
    Ok(())
}

fn ensure_scalar_params(params: &[clg_ast::Param]) -> Result<()> {
    for param in params {
        if !scalar_type(&param.ty) {
            bail!("EVM stateful scalar profile does not support parameter `{}`", param.name)
        }
    }
    Ok(())
}

fn ensure_scalar_events(contract: &ContractDecl) -> Result<()> {
    for event in &contract.events {
        for field in &event.fields {
            if !scalar_type(&field.ty) {
                bail!(
                    "EVM stateful scalar profile does not support event `{}.{}`",
                    event.name,
                    field.name
                )
            }
        }
    }
    Ok(())
}

fn scalar_type(ty: &Type) -> bool {
    matches!(ty, Type::Bool | Type::U8 | Type::U64 | Type::U128 | Type::Int)
}

fn find_lowered_function<'a>(ir: &'a IrModule, name: &str) -> Result<&'a IrFunction> {
    ir.funcs
        .iter()
        .find(|function| function.name == name)
        .ok_or_else(|| anyhow!("missing lowered function `{name}`"))
}

fn function_reads_state(expr: &Expr) -> bool {
    match expr {
        Expr::FieldAccess { base, .. } => matches!(base.as_ref(), Expr::Var(name, _) if name == "state") || function_reads_state(base),
        Expr::Block { block } => block.statements.iter().any(stmt_reads_state) || block.tail.as_deref().is_some_and(function_reads_state),
        Expr::Bin { lhs, rhs, .. } => function_reads_state(lhs) || function_reads_state(rhs),
        Expr::Call { args, .. } => args.iter().any(function_reads_state),
        Expr::If { cond, then_br, else_br, .. } => function_reads_state(cond) || function_reads_state(then_br) || function_reads_state(else_br),
        _ => false,
    }
}

fn stmt_reads_state(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => function_reads_state(expr),
        Stmt::While { cond, invariant, body, .. } => {
            function_reads_state(cond) || function_reads_state(invariant) || body.statements.iter().any(stmt_reads_state) || body.tail.as_deref().is_some_and(function_reads_state)
        }
    }
}

struct Handler { selector: String, code: Vec<u8> }

fn compile_transition(
    function: &IrFunction,
    param_count: usize,
    storage: &BTreeMap<String, StorageField>,
    events: &BTreeMap<String, [u8; 32]>,
    constructor: bool,
) -> Result<(Vec<u8>, Vec<JsonValue>)> {
    let mut code = if constructor { Vec::new() } else { runtime_parameter_loader(param_count)? };
    let mut mappings = Vec::new();
    for instruction in &function.body {
        match instruction {
            Instr::IConst { dst, n, .. } => {
                code.extend(push32(signed_word(*n)));
                code.extend(store_value(*dst)?);
            }
            Instr::StateRead { dst, field, .. } => {
                let field = storage.get(field).ok_or_else(|| anyhow!("unknown state field `{field}`"))?;
                code.extend(push32(field.slot_bytes));
                code.push(0x54); // SLOAD
                code.extend(store_value(*dst)?);
                mappings.push(json!({ "kind": "StateRead", "field_id": field.field_id, "slot": field.slot }));
            }
            Instr::StateWrite { field, src, .. } => {
                let field = storage.get(field).ok_or_else(|| anyhow!("unknown state field `{field}`"))?;
                code.extend(load_value(*src)?);
                code.extend(push32(field.slot_bytes));
                code.push(0x55); // SSTORE
                mappings.push(json!({ "kind": "StateWrite", "field_id": field.field_id, "slot": field.slot }));
            }
            Instr::EventEmit { event, args, .. } => {
                let topic = events.get(event).ok_or_else(|| anyhow!("unknown event `{event}`"))?;
                for (index, arg) in args.iter().enumerate() {
                    code.extend(load_value(*arg)?);
                    code.extend(push_usize(index * 32)?);
                    code.push(0x52); // MSTORE
                }
                code.extend(push32(*topic));
                code.extend(push_usize(args.len() * 32)?);
                code.extend(push_usize(0)?);
                code.push(0xa1); // LOG1
                mappings.push(json!({ "kind": "EventEmit", "event": event, "topic0": format!("0x{}", hex::encode(topic)) }));
            }
            Instr::Ret { val } if constructor => { let _ = val; }
            Instr::Ret { val } => {
                code.extend(load_value(*val)?);
                code.extend(push_usize(0)?);
                code.push(0x52); // MSTORE
                code.extend(push_usize(32)?);
                code.extend(push_usize(0)?);
                code.push(0xf3); // RETURN
            }
            other => bail!("EVM stateful scalar profile has no verified mapping for `{other:?}` in function `{}`", function.name),
        }
    }
    if !constructor && !function.body.iter().any(|instruction| matches!(instruction, Instr::Ret { .. })) {
        bail!("EVM stateful scalar function `{}` does not return", function.name)
    }
    Ok((code, mappings))
}

fn runtime_parameter_loader(count: usize) -> Result<Vec<u8>> {
    let mut code = Vec::new();
    for index in 0..count {
        code.extend(push_usize(4 + index * 32)?);
        code.push(0x35); // CALLDATALOAD
        code.extend(store_value(Value(index as u32))?);
    }
    Ok(code)
}

fn constructor_parameter_loader(count: usize) -> Result<Vec<u8>> {
    let mut code = Vec::new();
    for index in 0..count {
        code.push(0x38); // CODESIZE
        code.extend(push_usize(count * 32)?);
        code.push(0x03); // SUB: code_size - args_size
        code.extend(push_usize(index * 32)?);
        code.push(0x01); // ADD
        code.extend(push_usize(32)?);
        code.push(0x90); // SWAP1 -> size, offset
        code.extend(push_usize(value_memory_offset(Value(index as u32))? as usize)?);
        code.push(0x39); // CODECOPY
    }
    Ok(code)
}

fn static_handler(constant: [u8; 32]) -> Vec<u8> {
    let mut code = push32(constant);
    code.extend([0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3]);
    code
}

fn compile_static_pure_function(function: &Func) -> Result<[u8; 32]> {
    if function.effect != Effect::Pure { bail!("EVM static-pure profile requires `pure` function `{}`", function.name) }
    if !function.params.is_empty() || !function.type_params.is_empty() { bail!("EVM static-pure profile requires zero-parameter, non-generic function `{}`", function.name) }
    let value = literal_value(&function.body)?;
    match function.ret {
        Type::Bool if value == 0 || value == 1 => Ok(unsigned_word(value as u128)),
        Type::U8 if (0..=u8::MAX as i128).contains(&value) => Ok(unsigned_word(value as u128)),
        Type::U64 if (0..=u64::MAX as i128).contains(&value) => Ok(unsigned_word(value as u128)),
        Type::U128 if value >= 0 => Ok(unsigned_word(value as u128)),
        Type::Int => Ok(signed_word(i64::try_from(value).map_err(|_| anyhow!("Int literal is out of range"))?)),
        _ => bail!("EVM static-pure profile does not support the result of function `{}`", function.name),
    }
}

fn literal_value(expr: &Expr) -> Result<i128> {
    match expr {
        Expr::Int(value, _) => Ok((*value).into()),
        Expr::Bool(value, _) => Ok(i128::from(*value)),
        Expr::Call { callee, args, .. } if matches!(callee.as_str(), "U8" | "U64" | "U128" | "Int") && args.len() == 1 => literal_value(&args[0]),
        Expr::Block { block } => block.tail.as_deref().ok_or_else(|| anyhow!("EVM static-pure profile requires a literal tail expression")).and_then(literal_value),
        _ => bail!("EVM static-pure profile requires a literal Bool/Int/U8/U64/U128 result"),
    }
}

fn emit_runtime(handlers: &[Handler]) -> Result<Vec<u8>> {
    let dispatch_len = 6usize
        .checked_add(handlers.len().checked_mul(11).ok_or_else(|| anyhow!("too many EVM handlers"))?)
        .and_then(|value| value.checked_add(5))
        .ok_or_else(|| anyhow!("EVM runtime is too large"))?;
    let mut offsets = Vec::new();
    let mut next = dispatch_len;
    for handler in handlers {
        if next > u16::MAX as usize { bail!("EVM runtime is too large") }
        offsets.push(next);
        next = next.checked_add(1 + handler.code.len()).ok_or_else(|| anyhow!("EVM runtime is too large"))?;
    }
    let mut runtime = vec![0x60, 0x00, 0x35, 0x60, 0xe0, 0x1c];
    for (handler, offset) in handlers.iter().zip(offsets) {
        let selector = hex::decode(handler.selector.strip_prefix("0x").ok_or_else(|| anyhow!("malformed selector"))?)?;
        if selector.len() != 4 { bail!("malformed selector") }
        runtime.extend([0x80, 0x63]);
        runtime.extend(selector);
        runtime.extend([0x14, 0x61, (offset >> 8) as u8, offset as u8, 0x57]);
    }
    runtime.extend([0x60, 0x00, 0x60, 0x00, 0xfd]);
    for handler in handlers {
        runtime.push(0x5b);
        runtime.extend(&handler.code);
    }
    Ok(runtime)
}

fn wrap_init_code(runtime: &[u8]) -> Result<Vec<u8>> { wrap_stateful_init_code(&[], runtime) }

fn wrap_stateful_init_code(constructor: &[u8], runtime: &[u8]) -> Result<Vec<u8>> {
    if runtime.len() > u16::MAX as usize { bail!("EVM runtime is too large") }
    let runtime_len = runtime.len() as u16;
    let offset = u16::try_from(constructor.len() + 15).map_err(|_| anyhow!("EVM init code is too large"))?;
    let mut init = constructor.to_vec();
    init.extend([
        0x61, (runtime_len >> 8) as u8, runtime_len as u8,
        0x61, (offset >> 8) as u8, offset as u8,
        0x60, 0x00, 0x39,
        0x61, (runtime_len >> 8) as u8, runtime_len as u8,
        0x60, 0x00, 0xf3,
    ]);
    init.extend(runtime);
    Ok(init)
}

fn value_memory_offset(value: Value) -> Result<u16> {
    let offset = VALUE_MEMORY_BASE as u32 + value.0.checked_mul(32).ok_or_else(|| anyhow!("IR value index overflow"))?;
    u16::try_from(offset).map_err(|_| anyhow!("EVM stateful scalar profile has too many IR values"))
}

fn store_value(value: Value) -> Result<Vec<u8>> { let mut out = push_usize(value_memory_offset(value)? as usize)?; out.push(0x52); Ok(out) }
fn load_value(value: Value) -> Result<Vec<u8>> { let mut out = push_usize(value_memory_offset(value)? as usize)?; out.push(0x51); Ok(out) }

fn push_usize(value: usize) -> Result<Vec<u8>> {
    if value <= u8::MAX as usize { Ok(vec![0x60, value as u8]) }
    else if value <= u16::MAX as usize { Ok(vec![0x61, (value >> 8) as u8, value as u8]) }
    else { bail!("EVM immediate is too large") }
}
fn push32(word: [u8; 32]) -> Vec<u8> { let mut out = vec![0x7f]; out.extend(word); out }
fn unsigned_word(value: u128) -> [u8; 32] { let mut word = [0; 32]; word[16..].copy_from_slice(&value.to_be_bytes()); word }
fn signed_word(value: i64) -> [u8; 32] { let mut word = if value < 0 { [0xff; 32] } else { [0; 32] }; word[24..].copy_from_slice(&value.to_be_bytes()); word }
fn decode_fixed_32(value: &str) -> Result<[u8; 32]> { let bytes = hex::decode(value.strip_prefix("0x").ok_or_else(|| anyhow!("hex must be 0x-prefixed"))?)?; bytes.try_into().map_err(|_| anyhow!("expected 32 bytes")) }

#[cfg(test)]
mod tests {
    use super::*;
    use clg_parser::parse;
    use clg_typer::check;
    use std::collections::BTreeMap;

    #[test]
    fn emits_stable_deployable_bytecode_for_static_pure_contracts() {
        let program = parse(r#"contract Constant version 1 { state { total: U64; } pure function answer() -> U64 { U64(42) } }"#).expect("parse");
        let ir = check(&program).expect("lower");
        let first = evm_artifact(&program, &ir, None).expect("first artifact");
        let second = evm_artifact(&program, &ir, None).expect("second artifact");
        assert_eq!(canonical_json_bytes(&first), canonical_json_bytes(&second));
        assert_eq!(first["execution_profile"], STATIC_PROFILE);
    }

    #[test]
    fn emits_state_slots_and_ir_mapping_for_direct_scalar_transition() {
        let program = parse(r#"
            contract Counter version 1 {
                state { total: U64; }
                event Changed { total: U64; }
                init(initial: U64) { state.total = initial; 0 }
                mut function set_total(value: U64) -> U64 { state.total = value; emit Changed(value); state.total }
            }
        "#).expect("parse");
        let ir = check(&program).expect("lower");
        let artifact = evm_artifact(&program, &ir, None).expect("stateful artifact");
        assert_eq!(artifact["execution_profile"], STATEFUL_PROFILE);
        assert_eq!(artifact["state_mapping"]["storage_layout"].as_array().map(Vec::len), Some(1));
        assert!(artifact["state_mapping"]["transition_mapping"].to_string().contains("StateWrite"));
        assert!(artifact["state_mapping"]["transition_mapping"].to_string().contains("EventEmit"));
    }

    #[test]
    fn rejects_unmapped_ir_instruction() {
        let program = parse(r#"
            contract Counter version 1 {
                state { total: U64; }
                init(initial: U64) { state.total = initial; 0 }
                mut function set_total(value: U64) -> U64 { state.total = value; state.total }
            }
        "#).expect("parse");
        let mut ir = check(&program).expect("lower");
        ir.funcs[0].body.insert(0, Instr::ISelect {
            dst: Value(99), cond: Value(0), then_v: Value(0), else_v: Value(0),
        });
        assert!(evm_artifact(&program, &ir, None).unwrap_err().to_string().contains("no verified mapping"));
    }

    #[test]
    fn emitted_stateful_bytecode_runs_constructor_transition_and_event() {
        let program = parse(r#"
            contract Counter version 1 {
                state { total: U64; }
                event Changed { total: U64; }
                init(initial: U64) { state.total = initial; 0 }
                pure function read() -> U64 { state.total }
                mut function set_total(value: U64) -> U64 { state.total = value; emit Changed(value); state.total }
            }
        "#).expect("parse");
        let ir = check(&program).expect("lower");
        let artifact = evm_artifact(&program, &ir, None).expect("artifact");
        let mut storage = BTreeMap::new();
        let mut creation = hex::decode(artifact["bytecode"].as_str().expect("bytecode").trim_start_matches("0x")).expect("hex");
        creation.extend(unsigned_word(7));
        let runtime = run_evm(&creation, &[], &mut storage).expect("constructor").0;
        let slot = decode_fixed_32(artifact["state_mapping"]["storage_layout"][0]["slot"].as_str().expect("slot")).expect("slot bytes");
        assert_eq!(storage.get(&slot), Some(&unsigned_word(7)));
        let wire = evm_wire_abi_artifact(&program, None).expect("wire ABI");
        let selector = |name: &str| wire["functions"].as_array().expect("functions").iter().find(|function| function["name"] == name).and_then(|function| function["selector"].as_str()).expect("selector");
        let mut set_call = hex::decode(selector("set_total").trim_start_matches("0x")).expect("selector");
        set_call.extend(unsigned_word(9));
        let (result, logs) = run_evm(&runtime, &set_call, &mut storage).expect("set total");
        assert_eq!(result, unsigned_word(9));
        assert_eq!(storage.get(&slot), Some(&unsigned_word(9)));
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].1, unsigned_word(9).to_vec());
        let read = hex::decode(selector("read").trim_start_matches("0x")).expect("selector");
        assert_eq!(run_evm(&runtime, &read, &mut storage).expect("read").0, unsigned_word(9));
    }

    #[test]
    fn stateful_scalar_profile_conforms_for_every_supported_storage_word_type() {
        let program = parse(r#"
            contract Scalars version 1 {
                state { enabled: Bool; small: U8; amount: U128; delta: Int; }
                init(enabled: Bool, small: U8, amount: U128, delta: Int) {
                    state.enabled = enabled;
                    state.small = small;
                    state.amount = amount;
                    state.delta = delta;
                    0
                }
                mut function set(enabled: Bool, small: U8, amount: U128, delta: Int) -> Int {
                    state.enabled = enabled;
                    state.small = small;
                    state.amount = amount;
                    state.delta = delta;
                    state.delta
                }
            }
        "#).expect("parse");
        let ir = check(&program).expect("lower");
        let artifact = evm_artifact(&program, &ir, None).expect("artifact");
        assert_eq!(artifact["execution_profile"], STATEFUL_PROFILE);
        let mut creation = hex::decode(artifact["bytecode"].as_str().expect("bytecode").trim_start_matches("0x")).expect("hex");
        creation.extend(unsigned_word(1));
        creation.extend(unsigned_word(7));
        creation.extend(unsigned_word((1_u128 << 80) + 3));
        creation.extend(signed_word(-5));
        let mut storage = BTreeMap::new();
        let runtime = run_evm(&creation, &[], &mut storage).expect("constructor").0;
        let slots = artifact["state_mapping"]["storage_layout"].as_array().expect("storage layout");
        let slot_for = |name: &str| slots.iter().find(|field| field["field"] == name).and_then(|field| field["slot"].as_str()).map(decode_fixed_32).expect("slot").expect("slot bytes");
        assert_eq!(storage.get(&slot_for("enabled")), Some(&unsigned_word(1)));
        assert_eq!(storage.get(&slot_for("small")), Some(&unsigned_word(7)));
        assert_eq!(storage.get(&slot_for("amount")), Some(&unsigned_word((1_u128 << 80) + 3)));
        assert_eq!(storage.get(&slot_for("delta")), Some(&signed_word(-5)));

        let wire = evm_wire_abi_artifact(&program, None).expect("wire ABI");
        let selector = wire["functions"].as_array().expect("functions").iter()
            .find(|function| function["name"] == "set")
            .and_then(|function| function["selector"].as_str()).expect("set selector");
        let mut call = hex::decode(selector.trim_start_matches("0x")).expect("selector hex");
        call.extend(unsigned_word(0));
        call.extend(unsigned_word(9));
        call.extend(unsigned_word((1_u128 << 100) + 11));
        call.extend(signed_word(-11));
        assert_eq!(run_evm(&runtime, &call, &mut storage).expect("set").0, signed_word(-11));
        assert_eq!(storage.get(&slot_for("enabled")), Some(&unsigned_word(0)));
        assert_eq!(storage.get(&slot_for("small")), Some(&unsigned_word(9)));
        assert_eq!(storage.get(&slot_for("amount")), Some(&unsigned_word((1_u128 << 100) + 11)));
        assert_eq!(storage.get(&slot_for("delta")), Some(&signed_word(-11)));
    }

    #[allow(clippy::type_complexity)]
    fn run_evm(code: &[u8], calldata: &[u8], storage: &mut BTreeMap<[u8; 32], [u8; 32]>) -> Result<(Vec<u8>, Vec<([u8; 32], Vec<u8>)>)> {
        let mut pc = 0; let mut stack = Vec::<[u8; 32]>::new(); let mut memory = Vec::new(); let mut logs = Vec::new();
        for _ in 0..10_000 {
            let op = *code.get(pc).ok_or_else(|| anyhow!("end of code"))?; pc += 1;
            if (0x60..=0x7f).contains(&op) { let count = (op - 0x5f) as usize; let bytes = code.get(pc..pc + count).ok_or_else(|| anyhow!("truncated push"))?; pc += count; let mut word = [0; 32]; word[32-count..].copy_from_slice(bytes); stack.push(word); continue; }
            let pop = |stack: &mut Vec<[u8; 32]>| stack.pop().ok_or_else(|| anyhow!("stack underflow"));
            match op {
                0x01 => { let right=pop(&mut stack)?; let left=pop(&mut stack)?; stack.push(word_add(left,right)); }
                0x03 => { let right=pop(&mut stack)?; let left=pop(&mut stack)?; stack.push(word_sub(left,right)); }
                0x14 => { let right=pop(&mut stack)?; let left=pop(&mut stack)?; stack.push(unsigned_word(u128::from(left==right))); }
                0x1c => { let shift=word_usize(pop(&mut stack)?)?; let value=pop(&mut stack)?; stack.push(word_shr(value,shift)); }
                0x35 => { let offset=word_usize(pop(&mut stack)?)?; stack.push(read_word(calldata,offset)); }
                0x38 => stack.push(unsigned_word(code.len() as u128)),
                0x39 => { let destination=word_usize(pop(&mut stack)?)?; let source=word_usize(pop(&mut stack)?)?; let size=word_usize(pop(&mut stack)?)?; ensure_mem(&mut memory,destination+size); for index in 0..size { memory[destination+index]=code.get(source+index).copied().unwrap_or(0); } }
                0x51 => { let offset=word_usize(pop(&mut stack)?)?; stack.push(read_word(&memory,offset)); }
                0x52 => { let offset=word_usize(pop(&mut stack)?)?; let value=pop(&mut stack)?; ensure_mem(&mut memory,offset+32); memory[offset..offset+32].copy_from_slice(&value); }
                0x54 => { let slot=pop(&mut stack)?; stack.push(storage.get(&slot).copied().unwrap_or([0;32])); }
                0x55 => { let slot=pop(&mut stack)?; let value=pop(&mut stack)?; storage.insert(slot,value); }
                0x57 => { let destination=word_usize(pop(&mut stack)?)?; let condition=pop(&mut stack)?; if condition != [0;32] { pc=destination; } }
                0x5b => {}, 0x80 => { let value=*stack.last().ok_or_else(|| anyhow!("stack underflow"))?; stack.push(value); },
                0x90 => { let len=stack.len(); if len<2 { bail!("stack underflow") } stack.swap(len-1,len-2); }
                0xa1 => { let start=word_usize(pop(&mut stack)?)?; let size=word_usize(pop(&mut stack)?)?; let topic=pop(&mut stack)?; ensure_mem(&mut memory,start+size); logs.push((topic,memory[start..start+size].to_vec())); }
                0xf3 => { let start=word_usize(pop(&mut stack)?)?; let size=word_usize(pop(&mut stack)?)?; ensure_mem(&mut memory,start+size); return Ok((memory[start..start+size].to_vec(),logs)); }
                0xfd => bail!("revert"), _ => bail!("unsupported opcode 0x{op:02x}"),
            }
        }
        bail!("execution limit")
    }

    fn ensure_mem(memory: &mut Vec<u8>, size: usize) { if memory.len()<size { memory.resize(size,0); } }
    fn read_word(memory: &[u8], offset: usize) -> [u8;32] { let mut word=[0;32]; for (index, byte) in word.iter_mut().enumerate() { *byte=memory.get(offset+index).copied().unwrap_or(0); } word }
    fn word_usize(word: [u8;32]) -> Result<usize> { usize::try_from(u64::from_be_bytes(word[24..].try_into().expect("tail"))).map_err(|_| anyhow!("usize")) }
    fn word_add(left: [u8;32], right: [u8;32]) -> [u8;32] { let mut out=[0;32]; let mut carry=0u16; for index in (0..32).rev() { let sum=left[index] as u16+right[index] as u16+carry; out[index]=sum as u8; carry=sum>>8; } out }
    fn word_sub(left: [u8;32], right: [u8;32]) -> [u8;32] { let mut out=[0;32]; let mut borrow=0i16; for index in (0..32).rev() { let value=left[index] as i16-right[index] as i16-borrow; out[index]=value as u8; borrow=i16::from(value<0); } out }
    #[allow(clippy::needless_range_loop)]
    fn word_shr(value: [u8;32], shift: usize) -> [u8;32] { if shift>=256 { return [0;32] } let byte_shift=shift/8; let bit_shift=shift%8; let mut out=[0;32]; for target in byte_shift..32 { let source=target-byte_shift; out[target]|=value[source]>>bit_shift; if bit_shift!=0 && source>0 { out[target]|=value[source-1]<<(8-bit_shift); } } out }
}
