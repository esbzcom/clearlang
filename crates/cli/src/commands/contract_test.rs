//! Deterministic simulator-only contract campaign runner.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use super::simulate;
use crate::commands::helpers::{canonical_json_bytes, sha256_hex};
use crate::logging::Logger;

const PLAN_FORMAT: &str = "clg.contract-test-plan.v1";
const REPORT_FORMAT: &str = "clg.contract-test-report.v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    format: String,
    schema_version: u32,
    seed: u64,
    cases: u32,
    function: String,
    state: Value,
    caller: String,
    value: u64,
    block_number: u64,
    timestamp: u64,
    gas_limit: u64,
    memory_limit: u64,
    generators: Vec<Generator>,
    #[serde(default)]
    properties: Vec<Property>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
enum Generator {
    Bool,
    U8 { min: u8, max: u8 },
    U64 { min: u64, max: u64 },
    Int { min: i64, max: i64 },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Property {
    ResultEqualsArg { arg: usize },
    StateFieldEqualsArg { field: String, arg: usize },
}

pub fn run(
    source: PathBuf,
    plan_path: PathBuf,
    report_out: PathBuf,
    trace_dir: PathBuf,
    logger: Logger,
) -> Result<()> {
    let plan_bytes = fs::read(&plan_path)
        .with_context(|| format!("read contract test plan `{}`", plan_path.display()))?;
    let plan_value: Value = serde_json::from_slice(&plan_bytes)
        .with_context(|| format!("parse contract test plan `{}`", plan_path.display()))?;
    let plan: Plan = serde_json::from_value(plan_value.clone())
        .with_context(|| format!("validate contract test plan `{}`", plan_path.display()))?;
    validate_plan(&plan)?;
    fs::create_dir_all(&trace_dir).with_context(|| {
        format!(
            "create contract test trace directory `{}`",
            trace_dir.display()
        )
    })?;

    let plan_digest = format!("sha256:{}", sha256_hex(&canonical_json_bytes(&plan_value)));
    let mut rng = XorShift64::new(plan.seed);
    let mut cases = Vec::with_capacity(plan.cases as usize);
    let mut first_error = None;
    for index in 0..plan.cases {
        let stem = format!("case-{index:04}");
        let args_path = trace_dir.join(format!("{stem}.args.json"));
        let state_path = trace_dir.join(format!("{stem}.state.json"));
        let state_out = trace_dir.join(format!("{stem}.state-out.json"));
        let trace_out = trace_dir.join(format!("{stem}.trace.json"));
        let args = Value::Array(
            plan.generators
                .iter()
                .map(|generator| generator.sample(&mut rng))
                .collect(),
        );
        fs::write(&args_path, canonical_json_bytes(&args))
            .with_context(|| format!("write campaign arguments `{}`", args_path.display()))?;
        fs::write(&state_path, canonical_json_bytes(&plan.state))
            .with_context(|| format!("write campaign state `{}`", state_path.display()))?;
        let result = simulate::run(
            source.clone(),
            Some(plan.function.clone()),
            false,
            state_path.clone(),
            args_path.clone(),
            state_out.clone(),
            trace_out.clone(),
            plan.caller.clone(),
            plan.value,
            plan.block_number,
            plan.timestamp,
            plan.gas_limit,
            plan.memory_limit,
            false,
            logger,
        );
        let status = match result {
            Err(error) => {
                first_error.get_or_insert_with(|| format!("{error:#}"));
                "failure"
            }
            Ok(()) => match check_properties(&plan.properties, &args, &trace_out) {
                Ok(()) => "success",
                Err(error) => {
                    first_error.get_or_insert_with(|| format!("{error:#}"));
                    "failure"
                }
            },
        };
        cases.push(json!({
            "args": args,
            "case": index,
            "replay": { "argv": replay_argv(&source, &plan, &state_path, &args_path, &state_out, &trace_out) },
            "state_out": state_out,
            "status": status,
            "trace": trace_out,
        }));
        if first_error.is_some() {
            break;
        }
    }
    let report = json!({
        "cases": cases,
        "format": REPORT_FORMAT,
        "plan_digest": plan_digest,
        "schema_version": 1,
        "seed": plan.seed,
        "source": source,
        "status": if first_error.is_some() { "failure" } else { "success" },
    });
    fs::write(&report_out, canonical_json_bytes(&report))
        .with_context(|| format!("write contract test report `{}`", report_out.display()))?;
    if let Some(error) = first_error {
        bail!("contract campaign failed: {error}")
    }
    Ok(())
}

impl Generator {
    fn sample(&self, rng: &mut XorShift64) -> Value {
        match self {
            Self::Bool => Value::Bool(rng.next() & 1 == 1),
            Self::U8 { min, max } => json!(sample_u64(rng, u64::from(*min), u64::from(*max))),
            Self::U64 { min, max } => json!(sample_u64(rng, *min, *max)),
            Self::Int { min, max } => json!(sample_i64(rng, *min, *max)),
        }
    }

    fn valid(&self) -> bool {
        match self {
            Self::Bool => true,
            Self::U8 { min, max } => min <= max,
            Self::U64 { min, max } => min <= max,
            Self::Int { min, max } => min <= max,
        }
    }
}

fn validate_plan(plan: &Plan) -> Result<()> {
    if plan.format != PLAN_FORMAT || plan.schema_version != 1 {
        bail!("contract test plan must use `{PLAN_FORMAT}` schema_version 1")
    }
    if plan.cases == 0 || plan.function.is_empty() || plan.gas_limit == 0 || plan.memory_limit == 0
    {
        bail!("contract test plan requires non-zero cases/limits and a transition function")
    }
    if !plan.state.is_object()
        || plan.caller.trim().is_empty()
        || !plan.generators.iter().all(Generator::valid)
    {
        bail!("contract test plan has invalid state, caller, or generator bounds")
    }
    for property in &plan.properties {
        match property {
            Property::ResultEqualsArg { arg } => {
                ensure_argument_index(*arg, plan.generators.len())?
            }
            Property::StateFieldEqualsArg { field, arg } => {
                if field.is_empty() {
                    bail!("contract test property state field must be non-empty")
                }
                ensure_argument_index(*arg, plan.generators.len())?;
            }
        }
    }
    Ok(())
}

fn ensure_argument_index(index: usize, count: usize) -> Result<()> {
    if index >= count {
        bail!(
            "contract test property references argument {index}, but only {count} generators exist"
        )
    }
    Ok(())
}

fn check_properties(properties: &[Property], args: &Value, trace_path: &Path) -> Result<()> {
    let trace: Value = serde_json::from_slice(
        &fs::read(trace_path)
            .with_context(|| format!("read campaign trace `{}`", trace_path.display()))?,
    )
    .with_context(|| format!("parse campaign trace `{}`", trace_path.display()))?;
    let args = args.as_array().expect("campaign arguments are an array");
    for property in properties {
        match property {
            Property::ResultEqualsArg { arg } if trace["result"] != args[*arg] => {
                bail!("contract test property failed: result does not equal argument {arg}")
            }
            Property::StateFieldEqualsArg { field, arg }
                if trace["state_after"][field] != args[*arg] =>
            {
                bail!("contract test property failed: state field `{field}` does not equal argument {arg}")
            }
            _ => {}
        }
    }
    Ok(())
}

fn replay_argv(
    source: &Path,
    plan: &Plan,
    state: &Path,
    args: &Path,
    state_out: &Path,
    trace: &Path,
) -> Vec<String> {
    vec![
        "clg".to_string(),
        "simulate".to_string(),
        source.display().to_string(),
        "--function".to_string(),
        plan.function.clone(),
        "--state".to_string(),
        state.display().to_string(),
        "--args".to_string(),
        args.display().to_string(),
        "--state-out".to_string(),
        state_out.display().to_string(),
        "--trace-out".to_string(),
        trace.display().to_string(),
        "--caller".to_string(),
        plan.caller.clone(),
        "--value".to_string(),
        plan.value.to_string(),
        "--block-number".to_string(),
        plan.block_number.to_string(),
        "--timestamp".to_string(),
        plan.timestamp.to_string(),
        "--gas-limit".to_string(),
        plan.gas_limit.to_string(),
        "--memory-limit".to_string(),
        plan.memory_limit.to_string(),
    ]
}

struct XorShift64(u64);
impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        })
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}
fn sample_u64(rng: &mut XorShift64, min: u64, max: u64) -> u64 {
    if min == 0 && max == u64::MAX {
        rng.next()
    } else {
        min + rng.next() % (max - min + 1)
    }
}
fn sample_i64(rng: &mut XorShift64, min: i64, max: i64) -> i64 {
    let span = (i128::from(max) - i128::from(min) + 1) as u128;
    (i128::from(min) + (((u128::from(rng.next())) % span) as i128)) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generator_replay_is_seed_deterministic_and_bounded() {
        let generator = Generator::Int { min: -3, max: 3 };
        let mut first = XorShift64::new(42);
        let mut second = XorShift64::new(42);
        let values = (0..8)
            .map(|_| generator.sample(&mut first))
            .collect::<Vec<_>>();
        assert_eq!(
            values,
            (0..8)
                .map(|_| generator.sample(&mut second))
                .collect::<Vec<_>>()
        );
        assert!(values.iter().all(|value| value
            .as_i64()
            .is_some_and(|value| (-3..=3).contains(&value))));
    }

    #[test]
    fn property_failure_is_fail_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let trace = dir.path().join("trace.json");
        fs::write(&trace, br#"{"result":1,"state_after":{"total":1}}"#).expect("trace");
        let error = check_properties(
            &[Property::StateFieldEqualsArg {
                field: "total".to_string(),
                arg: 0,
            }],
            &json!([2]),
            &trace,
        )
        .expect_err("property mismatch must fail");
        assert!(error.to_string().contains("property failed"));
    }
}
