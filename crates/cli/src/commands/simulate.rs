use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clg_ast::Type;
use clg_ir::{BinOpIR, Instr, IrType, Value};
use clg_typer::{check_with_vcs_with_std_and_external, ExternalBuiltinSig};
use serde_json::{json, Value as JsonValue};

use super::modules::load_program;
use crate::commands::helpers::canonical_json_bytes;
use crate::logging::Logger;

pub fn run(
    file: PathBuf,
    function: Option<String>,
    init: bool,
    state: PathBuf,
    args: PathBuf,
    state_out: PathBuf,
    trace_out: PathBuf,
    caller: String,
    value: u64,
    block_number: u64,
    timestamp: u64,
    gas_limit: u64,
    _logger: Logger,
) -> Result<()> {
    let loaded = load_program(&file, false)?;
    let external_sigs = loaded
        .external_imports
        .iter()
        .map(|binding| ExternalBuiltinSig {
            name: binding.function.clone(),
            params: binding.params.clone(),
            ret: binding.ret.clone(),
            effect: binding.effect,
            route: binding.route,
        })
        .collect::<Vec<_>>();
    let typed = check_with_vcs_with_std_and_external(
        &loaded.program,
        &loaded.std_types,
        external_sigs.as_slice(),
    )
    .context("type-check failed")?;
    let contract = match typed.mono_program.contracts.as_slice() {
        [contract] => contract,
        _ => bail!("simulate requires exactly one contract declaration"),
    };
    let (entrypoint, params, is_init) = if init {
        let init = contract
            .init
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("contract has no init declaration"))?;
        ("init".to_string(), init.params.as_slice(), true)
    } else {
        let function = function.expect("clap requires --function when --init is absent");
        let source_func = contract
            .functions
            .iter()
            .find(|candidate| candidate.name == function)
            .ok_or_else(|| anyhow::anyhow!("contract transition `{function}` not found"))?;
        (function, source_func.params.as_slice(), false)
    };
    let ir_name = if is_init {
        format!("__clg_contract_init${}", contract.name)
    } else {
        entrypoint.clone()
    };
    let func = typed
        .ir
        .funcs
        .iter()
        .find(|candidate| candidate.name == ir_name)
        .ok_or_else(|| anyhow::anyhow!("contract transition `{entrypoint}` not found"))?;
    let mut storage = read_object(&state, "state")?;
    let arg_values = read_array(&args, "args")?;
    if arg_values.len() != params.len() {
        bail!(
            "simulator argument count mismatch: expected {}, found {}",
            params.len(),
            arg_values.len()
        );
    }
    if is_init {
        if !storage.is_empty() {
            bail!("simulator init may only execute against an empty state object");
        }
    } else {
        validate_initialized_state(&storage, &contract.fields)?;
    }

    let before = JsonValue::Object(storage.clone());
    let mut values = HashMap::new();
    for (index, (arg_value, parameter)) in arg_values.iter().zip(params).enumerate() {
        values.insert(index as u32, scalar_for_type(arg_value, &parameter.ty)?);
    }
    let event_types = contract
        .events
        .iter()
        .map(|event| {
            let fields = event
                .fields
                .iter()
                .map(|field| ir_scalar_type(&field.ty))
                .collect::<Result<Vec<_>>>()?;
            Ok((event.name.as_str(), fields))
        })
        .collect::<Result<HashMap<_, _>>>()?;
    let mut events = Vec::new();
    let mut state_changes = Vec::new();
    let mut fuel_used = 0_u64;
    let execution = execute(
        func.body.as_slice(),
        &mut values,
        &mut storage,
        &mut events,
        &mut state_changes,
        &event_types,
        &mut fuel_used,
        gas_limit,
    );
    let after = JsonValue::Object(storage.clone());
    let mut trace = json!({
        "block": { "number": block_number, "timestamp": timestamp },
        "caller": caller,
        "contract": contract.name,
        "events": events,
        "fuel_limit": gas_limit,
        "fuel_used": fuel_used,
        "function": entrypoint,
        "lifecycle": if is_init { "init" } else { "transition" },
        "result": JsonValue::Null,
        "schema_version": 1,
        "state_after": after,
        "state_before": before,
        "state_changes": state_changes,
        "status": "failure",
        "trace_format": "clg.contract-simulation-trace.v1",
        "value": value,
    });
    match execution {
        Ok(result) => {
            if is_init {
                validate_initialized_state(&storage, &contract.fields)?;
            }
            fs::write(&state_out, canonical_json_bytes(&after))
                .with_context(|| format!("write simulated state `{}`", state_out.display()))?;
            if !is_init {
                trace["result"] = scalar_json(
                    result,
                    func.ret.ok_or_else(|| {
                        anyhow::anyhow!("simulator transition has no scalar return type")
                    })?,
                );
            }
            trace["status"] = JsonValue::String("success".to_string());
        }
        Err(error) => {
            trace["failure"] = JsonValue::String(format!("{error:#}"));
            fs::write(&trace_out, canonical_json_bytes(&trace))
                .with_context(|| format!("write simulation trace `{}`", trace_out.display()))?;
            return Err(error);
        }
    }
    fs::write(&trace_out, canonical_json_bytes(&trace))
        .with_context(|| format!("write simulation trace `{}`", trace_out.display()))
}

fn execute(
    body: &[Instr],
    values: &mut HashMap<u32, i64>,
    storage: &mut serde_json::Map<String, JsonValue>,
    events: &mut Vec<JsonValue>,
    state_changes: &mut Vec<JsonValue>,
    event_types: &HashMap<&str, Vec<IrType>>,
    fuel: &mut u64,
    limit: u64,
) -> Result<i64> {
    for instruction in body {
        *fuel = fuel
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("simulator fuel overflow"))?;
        if *fuel > limit {
            bail!("simulator fuel limit exceeded");
        }
        match instruction {
            Instr::IConst { dst, n, .. } => {
                values.insert(dst.0, *n);
            }
            Instr::StateRead { dst, field, .. } => {
                let stored = storage
                    .get(field)
                    .ok_or_else(|| anyhow::anyhow!("missing state field `{field}`"))?;
                values.insert(dst.0, scalar(stored)?);
            }
            Instr::StateWrite { field, src, ty, .. } => {
                let field_value = get(values, *src)?;
                let json_value = scalar_json(field_value, *ty);
                storage.insert(field.clone(), json_value.clone());
                state_changes.push(json!({ "field": field, "value": json_value }));
            }
            Instr::EventEmit { event, args, .. } => {
                let argument_types = event_types
                    .get(event.as_str())
                    .ok_or_else(|| anyhow::anyhow!("simulator event `{event}` is not declared"))?;
                if args.len() != argument_types.len() {
                    bail!("simulator event `{event}` has an invalid argument count");
                }
                let event_args = args
                    .iter()
                    .zip(argument_types)
                    .map(|(argument, ty)| {
                        get(values, *argument).map(|value| scalar_json(value, *ty))
                    })
                    .collect::<Result<Vec<_>>>()?;
                events.push(json!({ "event": event, "args": event_args }));
            }
            Instr::IBin {
                dst, op, lhs, rhs, ..
            } => {
                values.insert(dst.0, bin(*op, get(values, *lhs)?, get(values, *rhs)?)?);
            }
            Instr::ISelect {
                dst,
                cond,
                then_v,
                else_v,
            } => {
                let selected = if get(values, *cond)? != 0 {
                    get(values, *then_v)?
                } else {
                    get(values, *else_v)?
                };
                values.insert(dst.0, selected);
            }
            Instr::Guard { cond, .. } => {
                if get(values, *cond)? == 0 {
                    bail!("simulator contract guard failed");
                }
            }
            Instr::Ret { val } => return get(values, *val),
            Instr::ExternalCall { .. } => bail!("simulator does not support external calls"),
            other => bail!("simulator does not support instruction `{other:?}`"),
        }
    }
    bail!("simulator transition has no return")
}

fn get(values: &HashMap<u32, i64>, value: Value) -> Result<i64> {
    values
        .get(&value.0)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("simulator missing SSA value {}", value.0))
}

fn scalar_for_type(value: &JsonValue, ty: &Type) -> Result<i64> {
    match ty {
        Type::Bool => value
            .as_bool()
            .map(|boolean| if boolean { 1 } else { 0 })
            .ok_or_else(|| anyhow::anyhow!("simulator expected a JSON boolean")),
        Type::Int => value
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("simulator expected a signed JSON integer")),
        Type::U64 => value
            .as_u64()
            .and_then(|number| i64::try_from(number).ok())
            .ok_or_else(|| anyhow::anyhow!("simulator expected an in-range U64 JSON integer")),
        _ => bail!("simulator supports only Bool, Int, and in-range U64 scalar values"),
    }
}

fn validate_initialized_state(
    storage: &serde_json::Map<String, JsonValue>,
    fields: &[clg_ast::StructField],
) -> Result<()> {
    for field in fields {
        let field_value = storage
            .get(&field.name)
            .ok_or_else(|| anyhow::anyhow!("simulator state is missing field `{}`", field.name))?;
        scalar_for_type(field_value, &field.ty)
            .with_context(|| format!("state field `{}`", field.name))?;
    }
    if storage.len() != fields.len() {
        bail!("simulator state contains undeclared fields");
    }
    Ok(())
}

fn scalar(value: &JsonValue) -> Result<i64> {
    value
        .as_i64()
        .or_else(|| value.as_bool().map(|boolean| if boolean { 1 } else { 0 }))
        .ok_or_else(|| anyhow::anyhow!("simulator supports scalar integers and booleans only"))
}

fn ir_scalar_type(ty: &Type) -> Result<IrType> {
    match ty {
        Type::Bool => Ok(IrType::Bool),
        Type::Int => Ok(IrType::Int),
        Type::U64 => Ok(IrType::U64),
        _ => bail!("simulator supports only Bool, Int, and in-range U64 scalar values"),
    }
}

fn scalar_json(value: i64, ty: IrType) -> JsonValue {
    match ty {
        IrType::Bool => JsonValue::Bool(value != 0),
        IrType::Int | IrType::U8 | IrType::U64 => json!(value),
        IrType::U128 | IrType::U256 => {
            unreachable!("unsupported IR type is rejected before output")
        }
    }
}

fn read_object(path: &PathBuf, label: &str) -> Result<serde_json::Map<String, JsonValue>> {
    serde_json::from_slice::<JsonValue>(
        &fs::read(path).with_context(|| format!("read simulator {label} `{}`", path.display()))?,
    )?
    .as_object()
    .cloned()
    .ok_or_else(|| anyhow::anyhow!("simulator {label} must be a JSON object"))
}

fn read_array(path: &PathBuf, label: &str) -> Result<Vec<JsonValue>> {
    serde_json::from_slice::<JsonValue>(
        &fs::read(path).with_context(|| format!("read simulator {label} `{}`", path.display()))?,
    )?
    .as_array()
    .cloned()
    .ok_or_else(|| anyhow::anyhow!("simulator {label} must be a JSON array"))
}

fn bin(op: BinOpIR, left: i64, right: i64) -> Result<i64> {
    Ok(match op {
        BinOpIR::Add => left
            .checked_add(right)
            .ok_or_else(|| anyhow::anyhow!("simulator integer overflow"))?,
        BinOpIR::Sub => left
            .checked_sub(right)
            .ok_or_else(|| anyhow::anyhow!("simulator integer overflow"))?,
        BinOpIR::Mul => left
            .checked_mul(right)
            .ok_or_else(|| anyhow::anyhow!("simulator integer overflow"))?,
        BinOpIR::Div => left
            .checked_div(right)
            .ok_or_else(|| anyhow::anyhow!("simulator division overflow or zero"))?,
        BinOpIR::Lt => i64::from(left < right),
        BinOpIR::Le => i64::from(left <= right),
        BinOpIR::LeU => i64::from((left as u64) <= (right as u64)),
        BinOpIR::Gt => i64::from(left > right),
        BinOpIR::Ge => i64::from(left >= right),
        BinOpIR::Eq => i64::from(left == right),
        BinOpIR::Neq => i64::from(left != right),
        BinOpIR::And => i64::from(left != 0 && right != 0),
        BinOpIR::Or => i64::from(left != 0 || right != 0),
        BinOpIR::Xor => left ^ right,
        BinOpIR::Shl => left
            .checked_shl(right as u32)
            .ok_or_else(|| anyhow::anyhow!("simulator shift overflow"))?,
        BinOpIR::Shr => left
            .checked_shr(right as u32)
            .ok_or_else(|| anyhow::anyhow!("simulator shift overflow"))?,
    })
}
