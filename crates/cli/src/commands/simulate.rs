use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clg_ast::{Expr, Span, Type};
use clg_ir::{BinOpIR, Instr, IrType, Value};
use clg_typer::{check_with_vcs_with_std_and_external, ExternalBuiltinSig};
use serde_json::{json, Value as JsonValue};

use super::modules::load_program;
use crate::commands::helpers::{canonical_json_bytes, make_single_json_error, CommandError};
use crate::logging::{LogLevel, Logger};

#[allow(clippy::too_many_arguments)]
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
    memory_limit: u64,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    logger.event(LogLevel::Info, "start", "simulate", &[]);
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
    let (entrypoint, params, body_span, is_init) = if init {
        let init = contract
            .init
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("contract has no init declaration"))?;
        (
            "init".to_string(),
            init.params.as_slice(),
            expr_span(&init.body),
            true,
        )
    } else {
        let function = function.expect("clap requires --function when --init is absent");
        let source_func = contract
            .functions
            .iter()
            .find(|candidate| candidate.name == function)
            .ok_or_else(|| anyhow::anyhow!("contract transition `{function}` not found"))?;
        (
            function,
            source_func.params.as_slice(),
            expr_span(&source_func.body),
            false,
        )
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
    let mut storage = read_object(&state, "state", memory_limit)?;
    let arg_values = read_array(&args, "args", memory_limit)?;
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
    let caller_identity = func
        .body
        .iter()
        .any(|instruction| matches!(instruction, Instr::ContextCaller { .. }))
        .then(|| Address::parse(&caller))
        .transpose()?;
    let trace_caller = caller_identity
        .as_ref()
        .map_or_else(|| caller.clone(), Address::canonical);
    let mut values = HashMap::new();
    for (index, (arg_value, parameter)) in arg_values.iter().zip(params).enumerate() {
        values.insert(index as u32, value_for_type(arg_value, &parameter.ty)?);
    }
    let state_types = contract
        .fields
        .iter()
        .map(|field| (field.name.as_str(), field.ty.clone()))
        .collect::<HashMap<_, _>>();
    let event_types = contract
        .events
        .iter()
        .map(|event| {
            let fields = event
                .fields
                .iter()
                .map(|field| field.ty.clone())
                .collect::<Vec<_>>();
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
        caller_identity.as_ref(),
        &state_types,
        &event_types,
        &mut fuel_used,
        gas_limit,
    );
    let after = JsonValue::Object(storage.clone());
    let mut trace = json!({
        "block": { "number": block_number, "timestamp": timestamp },
        "caller": trace_caller,
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
            write_limited_json(&state_out, &after, memory_limit)
                .with_context(|| format!("write simulated state `{}`", state_out.display()))?;
            if !is_init {
                trace["result"] = scalar_json(
                    result.scalar()?,
                    func.ret.ok_or_else(|| {
                        anyhow::anyhow!("simulator transition has no scalar return type")
                    })?,
                );
            }
            trace["status"] = JsonValue::String("success".to_string());
        }
        Err(failure) => {
            let location = failure.span.unwrap_or(body_span);
            trace["failure"] = JsonValue::String(failure.message.clone());
            trace["failure_location"] = json!({
                "end": location.end,
                "file": file.display().to_string(),
                "function": entrypoint,
                "start": location.start,
            });
            trace["stack"] = json!([{
                "end": location.end,
                "file": file.display().to_string(),
                "function": entrypoint,
                "instruction": failure.instruction,
                "start": location.start,
            }]);
            write_limited_json(&trace_out, &trace, memory_limit)
                .with_context(|| format!("write simulation trace `{}`", trace_out.display()))?;
            logger.event(
                LogLevel::Info,
                "failure",
                "simulate",
                &[("code", "C142".to_string())],
            );
            if json_errors {
                return Err(CommandError::json(make_single_json_error(
                    "C142",
                    "simulate",
                    failure.message,
                    &file,
                    location.start,
                    location.end,
                    Some(entrypoint),
                ))
                .into());
            }
            return Err(anyhow::Error::msg(failure.message));
        }
    }
    write_limited_json(&trace_out, &trace, memory_limit)
        .with_context(|| format!("write simulation trace `{}`", trace_out.display()))?;
    logger.event(LogLevel::Info, "finish", "simulate", &[]);
    Ok(())
}

#[derive(Debug)]
struct ExecutionFailure {
    message: String,
    instruction: usize,
    span: Option<Span>,
}

#[allow(clippy::too_many_arguments)]
fn execute(
    body: &[Instr],
    values: &mut HashMap<u32, SimValue>,
    storage: &mut serde_json::Map<String, JsonValue>,
    events: &mut Vec<JsonValue>,
    state_changes: &mut Vec<JsonValue>,
    caller: Option<&Address>,
    state_types: &HashMap<&str, Type>,
    event_types: &HashMap<&str, Vec<Type>>,
    fuel: &mut u64,
    limit: u64,
) -> std::result::Result<SimValue, ExecutionFailure> {
    for (instruction_index, instruction) in body.iter().enumerate() {
        *fuel = fuel.checked_add(1).ok_or_else(|| {
            execution_failure(instruction, instruction_index, "simulator fuel overflow")
        })?;
        if *fuel > limit {
            return Err(execution_failure(
                instruction,
                instruction_index,
                "simulator fuel limit exceeded",
            ));
        }
        let instruction_result = (|| -> Result<Option<SimValue>> {
            match instruction {
                Instr::IConst { dst, n, .. } => {
                    values.insert(dst.0, SimValue::Scalar(*n));
                }
                Instr::StateRead { dst, field, .. } => {
                    let stored = storage
                        .get(field)
                        .ok_or_else(|| anyhow::anyhow!("missing state field `{field}`"))?;
                    let ty = state_types
                        .get(field.as_str())
                        .ok_or_else(|| anyhow::anyhow!("unknown state field `{field}`"))?;
                    values.insert(dst.0, value_for_type(stored, ty)?);
                }
                Instr::ContextCaller { dst } => {
                    let caller = caller.ok_or_else(|| {
                        anyhow::anyhow!("simulator caller identity is required for this transition")
                    })?;
                    values.insert(dst.0, SimValue::Address(*caller));
                }
                Instr::AddressEq { dst, lhs, rhs } => {
                    values.insert(
                        dst.0,
                        SimValue::Scalar(i64::from(
                            get_address(values, *lhs)? == get_address(values, *rhs)?,
                        )),
                    );
                }
                Instr::StateWrite { field, src, .. } => {
                    let field_value = get(values, *src)?;
                    let ty = state_types
                        .get(field.as_str())
                        .ok_or_else(|| anyhow::anyhow!("unknown state field `{field}`"))?;
                    let json_value = field_value.to_json(ty)?;
                    storage.insert(field.clone(), json_value.clone());
                    state_changes.push(json!({ "field": field, "value": json_value }));
                }
                Instr::EventEmit { event, args, .. } => {
                    let argument_types = event_types.get(event.as_str()).ok_or_else(|| {
                        anyhow::anyhow!("simulator event `{event}` is not declared")
                    })?;
                    if args.len() != argument_types.len() {
                        bail!("simulator event `{event}` has an invalid argument count");
                    }
                    let event_args = args
                        .iter()
                        .zip(argument_types)
                        .map(|(argument, ty)| {
                            get(values, *argument).and_then(|value| value.to_json(ty))
                        })
                        .collect::<Result<Vec<_>>>()?;
                    events.push(json!({ "event": event, "args": event_args }));
                }
                Instr::IBin {
                    dst, op, lhs, rhs, ..
                } => {
                    values.insert(
                        dst.0,
                        SimValue::Scalar(bin(
                            *op,
                            get(values, *lhs)?.scalar()?,
                            get(values, *rhs)?.scalar()?,
                        )?),
                    );
                }
                Instr::ISelect {
                    dst,
                    cond,
                    then_v,
                    else_v,
                } => {
                    let selected = if get(values, *cond)?.scalar()? != 0 {
                        get(values, *then_v)?.clone()
                    } else {
                        get(values, *else_v)?.clone()
                    };
                    values.insert(dst.0, selected);
                }
                Instr::Guard { cond, .. } => {
                    if get(values, *cond)?.scalar()? == 0 {
                        bail!("simulator contract guard failed");
                    }
                }
                Instr::Ret { val } => return Ok(Some(get(values, *val)?.clone())),
                Instr::ExternalCall { .. } => bail!("simulator does not support external calls"),
                other => bail!("simulator does not support instruction `{other:?}`"),
            }
            Ok(None)
        })();
        match instruction_result {
            Ok(Some(result)) => return Ok(result),
            Ok(None) => {}
            Err(error) => return Err(execution_failure(instruction, instruction_index, error)),
        }
    }
    Err(ExecutionFailure {
        message: "simulator transition has no return".to_string(),
        instruction: body.len(),
        span: None,
    })
}

fn execution_failure(
    instruction: &Instr,
    instruction_index: usize,
    error: impl std::fmt::Display,
) -> ExecutionFailure {
    ExecutionFailure {
        message: error.to_string(),
        instruction: instruction_index,
        span: instruction_span(instruction),
    }
}

fn instruction_span(instruction: &Instr) -> Option<Span> {
    match instruction {
        Instr::Guard {
            span: Some((start, end)),
            ..
        } => Some(Span {
            start: *start as usize,
            end: *end as usize,
        }),
        _ => None,
    }
}

fn expr_span(expr: &Expr) -> Span {
    match expr {
        Expr::Int(_, span) | Expr::Bool(_, span) | Expr::String(_, span) | Expr::Var(_, span) => {
            *span
        }
        Expr::ArrayLit { span, .. }
        | Expr::TupleLit { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Bin { span, .. }
        | Expr::Call { span, .. }
        | Expr::Return { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Match { span, .. }
        | Expr::If { span, .. }
        | Expr::Try { span, .. }
        | Expr::Lambda { span, .. } => *span,
        Expr::Block { block } => block.span,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SimValue {
    Scalar(i64),
    Address(Address),
}

impl SimValue {
    fn scalar(&self) -> Result<i64> {
        match self {
            Self::Scalar(value) => Ok(*value),
            Self::Address(_) => bail!("simulator expected a scalar value"),
        }
    }

    fn to_json(&self, ty: &Type) -> Result<JsonValue> {
        match (self, ty) {
            (Self::Scalar(value), Type::Bool) => Ok(JsonValue::Bool(*value != 0)),
            (Self::Scalar(value), Type::Int | Type::U8 | Type::U64) => Ok(json!(value)),
            (Self::Address(address), Type::Named { name, args })
                if name == "std::eth::Address" && args.is_empty() =>
            {
                Ok(JsonValue::String(address.canonical()))
            }
            _ => bail!("simulator value does not match the declared type"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Address([u8; 20]);

impl Address {
    fn parse(raw: &str) -> Result<Self> {
        let hex = raw.strip_prefix("0x").ok_or_else(|| {
            anyhow::anyhow!("address must use canonical lowercase 0x-prefixed hex")
        })?;
        if hex.len() != 40
            || !hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            bail!("address must be exactly 20 bytes of canonical lowercase hex")
        }
        let mut bytes = [0_u8; 20];
        hex::decode_to_slice(hex, &mut bytes).map_err(|_| {
            anyhow::anyhow!("address must be exactly 20 bytes of canonical lowercase hex")
        })?;
        Ok(Self(bytes))
    }

    fn canonical(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }
}

fn get(values: &HashMap<u32, SimValue>, value: Value) -> Result<&SimValue> {
    values
        .get(&value.0)
        .ok_or_else(|| anyhow::anyhow!("simulator missing SSA value {}", value.0))
}

fn get_address(values: &HashMap<u32, SimValue>, value: Value) -> Result<Address> {
    match get(values, value)? {
        SimValue::Address(address) => Ok(*address),
        SimValue::Scalar(_) => bail!("simulator expected a canonical address value"),
    }
}

fn value_for_type(value: &JsonValue, ty: &Type) -> Result<SimValue> {
    match ty {
        Type::Bool => value
            .as_bool()
            .map(|boolean| SimValue::Scalar(if boolean { 1 } else { 0 }))
            .ok_or_else(|| anyhow::anyhow!("simulator expected a JSON boolean")),
        Type::Int => value
            .as_i64()
            .map(SimValue::Scalar)
            .ok_or_else(|| anyhow::anyhow!("simulator expected a signed JSON integer")),
        Type::U64 => value
            .as_u64()
            .and_then(|number| i64::try_from(number).ok())
            .map(SimValue::Scalar)
            .ok_or_else(|| anyhow::anyhow!("simulator expected an in-range U64 JSON integer")),
        Type::Named { name, args } if name == "std::eth::Address" && args.is_empty() => value
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("simulator expected a canonical address string"))
            .and_then(Address::parse)
            .map(SimValue::Address),
        _ => bail!("simulator supports only Bool, Int, in-range U64, and canonical Address values"),
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
        value_for_type(field_value, &field.ty)
            .with_context(|| format!("state field `{}`", field.name))?;
    }
    if storage.len() != fields.len() {
        bail!("simulator state contains undeclared fields");
    }
    Ok(())
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

fn read_object(
    path: &PathBuf,
    label: &str,
    memory_limit: u64,
) -> Result<serde_json::Map<String, JsonValue>> {
    serde_json::from_slice::<JsonValue>(&read_limited(path, label, memory_limit)?)?
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("simulator {label} must be a JSON object"))
}

fn read_array(path: &PathBuf, label: &str, memory_limit: u64) -> Result<Vec<JsonValue>> {
    serde_json::from_slice::<JsonValue>(&read_limited(path, label, memory_limit)?)?
        .as_array()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("simulator {label} must be a JSON array"))
}

fn read_limited(path: &PathBuf, label: &str, memory_limit: u64) -> Result<Vec<u8>> {
    let bytes =
        fs::read(path).with_context(|| format!("read simulator {label} `{}`", path.display()))?;
    if bytes.len() as u64 > memory_limit {
        bail!("simulator {label} exceeds memory limit of {memory_limit} bytes");
    }
    Ok(bytes)
}

fn write_limited_json(path: &PathBuf, value: &JsonValue, memory_limit: u64) -> Result<()> {
    let bytes = canonical_json_bytes(value);
    if bytes.len() as u64 > memory_limit {
        bail!("simulator output exceeds memory limit of {memory_limit} bytes");
    }
    fs::write(path, bytes).with_context(|| format!("write simulator output `{}`", path.display()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caller_guard_uses_canonical_address_identity() {
        let owner =
            Address::parse("0x1111111111111111111111111111111111111111").expect("canonical owner");
        let mut values = HashMap::new();
        let mut storage = serde_json::Map::new();
        storage.insert("owner".to_string(), json!(owner.canonical()));
        storage.insert("value".to_string(), json!(0));
        let state_types = HashMap::from([
            (
                "owner",
                Type::Named {
                    name: "std::eth::Address".to_string(),
                    args: Vec::new(),
                },
            ),
            ("value", Type::U64),
        ]);
        let body = vec![
            Instr::ContextCaller { dst: Value(0) },
            Instr::StateRead {
                dst: Value(1),
                contract: "ConfigRegistry".to_string(),
                field: "owner".to_string(),
                ty: IrType::Int,
            },
            Instr::AddressEq {
                dst: Value(2),
                lhs: Value(0),
                rhs: Value(1),
            },
            Instr::Guard {
                cond: Value(2),
                trap: clg_ir::TrapCode::ContractViolation,
                span: None,
                detail: clg_ir::GuardKind::Require,
            },
            Instr::IConst {
                dst: Value(3),
                ty: IrType::Int,
                n: 7,
            },
            Instr::StateWrite {
                contract: "ConfigRegistry".to_string(),
                field: "value".to_string(),
                src: Value(3),
                ty: IrType::U64,
            },
            Instr::Ret { val: Value(3) },
        ];
        let mut events = Vec::new();
        let mut changes = Vec::new();
        let mut fuel = 0;
        let result = execute(
            &body,
            &mut values,
            &mut storage,
            &mut events,
            &mut changes,
            Some(&owner),
            &state_types,
            &HashMap::new(),
            &mut fuel,
            100,
        )
        .expect("owner must pass the guard");
        assert_eq!(result, SimValue::Scalar(7));
        assert_eq!(storage["value"], json!(7));

        let mut values = HashMap::new();
        let mut storage = serde_json::Map::new();
        storage.insert("owner".to_string(), json!(owner.canonical()));
        storage.insert("value".to_string(), json!(0));
        let mut events = Vec::new();
        let mut changes = Vec::new();
        let mut fuel = 0;
        let other = Address::parse("0x2222222222222222222222222222222222222222")
            .expect("canonical non-owner");
        let error = execute(
            &body,
            &mut values,
            &mut storage,
            &mut events,
            &mut changes,
            Some(&other),
            &state_types,
            &HashMap::new(),
            &mut fuel,
            100,
        )
        .expect_err("non-owner must fail the guard");
        assert!(error.message.contains("simulator contract guard failed"));
        assert_eq!(storage["value"], json!(0));
        assert!(changes.is_empty());
        assert!(events.is_empty());
    }

    #[test]
    fn address_input_requires_the_canonical_external_form() {
        for invalid in [
            "1111111111111111111111111111111111111111",
            "0x111111111111111111111111111111111111111",
            "0x111111111111111111111111111111111111111G",
            "0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ] {
            assert!(Address::parse(invalid).is_err(), "must reject {invalid}");
        }
    }
}
