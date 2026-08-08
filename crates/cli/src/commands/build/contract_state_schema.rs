use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use clg_ast::{ContractDecl, Program, Type};
use clg_typer::expr_to_source;
use serde_json::json;
#[cfg(test)]
use serde_json::Value;

use crate::commands::helpers::{canonical_json_bytes, sha256_hex};

const SCHEMA_VERSION: u32 = 1;
const LAYOUT_ALGORITHM: &str = "clg.contract-state-layout.v1";
const FIELD_ID_DOMAIN: &str = "clg.contract-state-field.v1";
const EVENT_ABI_ALGORITHM: &str = "clg.contract-event-abi.v1";
const EVENT_ID_DOMAIN: &str = "clg.contract-event.v1";
const EVENT_FIELD_ID_DOMAIN: &str = "clg.contract-event-field.v1";
const INVARIANT_ID_DOMAIN: &str = "clg.contract-state-invariant.v1";
const INIT_ID_DOMAIN: &str = "clg.contract-init.v1";
const TARGET_PROFILE_ID: &str = "clg.contract-state-solver.v1";

pub(super) fn write_contract_state_schema(
    program: &Program,
    path: &Path,
    source_graph: Option<&serde_json::Value>,
) -> Result<()> {
    let artifact = contract_state_schema_artifact(program, source_graph)?;
    fs::write(path, canonical_json_bytes(&artifact))
        .with_context(|| format!("write contract state schema `{}`", path.display()))
}

pub(super) fn check_contract_state_schema_compatibility(
    program: &Program,
    prior_schema_path: &Path,
) -> Result<()> {
    let prior_bytes = fs::read(prior_schema_path)
        .with_context(|| format!("read prior contract state schema `{}`", prior_schema_path.display()))?;
    let prior: serde_json::Value = serde_json::from_slice(&prior_bytes)
        .with_context(|| format!("parse prior contract state schema `{}`", prior_schema_path.display()))?;
    let current = contract_state_schema_artifact(program, None)?;

    if prior["schema_version"].as_u64() != Some(SCHEMA_VERSION.into()) {
        bail!("prior contract state schema has unsupported schema_version");
    }
    let prior_name = prior["contract"]["name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("prior contract state schema missing contract.name"))?;
    let current_name = current["contract"]["name"]
        .as_str()
        .expect("generated schema has contract name");
    if prior_name != current_name {
        bail!(
            "contract state schema identity mismatch: prior `{prior_name}`, current `{current_name}`"
        );
    }
    let prior_version = prior["contract"]["version"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("prior contract state schema missing contract.version"))?;
    let current_version = current["contract"]["version"]
        .as_u64()
        .expect("generated schema has contract version");
    if current_version <= prior_version {
        bail!(
            "contract version must increase for schema compatibility: prior {prior_version}, current {current_version}"
        );
    }
    let prior_fields = prior["fields"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("prior contract state schema missing fields array"))?;
    let current_fields = current["fields"]
        .as_array()
        .expect("generated schema has fields array");
    let incompatible = current_fields.len() < prior_fields.len()
        || prior_fields.iter().enumerate().any(|(index, prior_field)| {
            let current_field = &current_fields[index];
            ["field_id", "index", "name", "type"]
                .into_iter()
                .any(|key| prior_field[key] != current_field[key])
        });
    if incompatible {
        let prior_digest = prior["schema"]["digest"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("prior contract state schema missing schema.digest"))?;
        let declared_digest = current["migration"]["from_schema"].as_str();
        if declared_digest != Some(prior_digest) {
            bail!(
                "contract state schema is incompatible; declare a migration from prior schema digest `{prior_digest}`"
            );
        }
    }
    Ok(())
}

fn contract_state_schema_artifact(
    program: &Program,
    source_graph: Option<&serde_json::Value>,
) -> Result<serde_json::Value> {
    let contract = match program.contracts.as_slice() {
        [contract] => contract,
        [] => bail!("--emit-contract-state-schema requires exactly one `contract` declaration"),
        _ => bail!("--emit-contract-state-schema does not support multiple `contract` declarations"),
    };

    let fields = contract
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            json!({
                "field_id": field_id(contract, &field.name),
                "index": index,
                "name": field.name,
                "type": render_type(&field.ty),
            })
        })
        .collect::<Vec<_>>();
    let layout_identity = json!({
        "algorithm": LAYOUT_ALGORITHM,
        "contract": contract.name,
        "fields": fields,
        "version": contract.version,
    });
    let layout_digest = sha256_hex(canonical_json_bytes(&layout_identity).as_slice());
    let events = contract
        .events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            let event_id = event_id(contract, &event.name);
            let fields = event
                .fields
                .iter()
                .enumerate()
                .map(|(field_index, field)| {
                    json!({
                        "field_id": event_field_id(&event_id, &field.name),
                        "index": field_index,
                        "name": field.name,
                        "type": render_type(&field.ty),
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "event_id": event_id,
                "fields": fields,
                "index": index,
                "name": event.name,
            })
        })
        .collect::<Vec<_>>();
    let event_abi_identity = json!({
        "algorithm": EVENT_ABI_ALGORITHM,
        "contract": contract.name,
        "events": events,
        "version": contract.version,
    });
    let event_abi_digest = sha256_hex(canonical_json_bytes(&event_abi_identity).as_slice());
    let invariants = contract
        .invariants
        .iter()
        .enumerate()
        .map(|(index, invariant)| {
            let expression = expr_to_source(&invariant.expr, 0);
            let input = format!(
                "{INVARIANT_ID_DOMAIN}\0{}\0{index}\0{expression}",
                contract.name
            );
            json!({
                "expression": expression,
                "index": index,
                "invariant_id": format!("sha256:{}", sha256_hex(input.as_bytes())),
            })
        })
        .collect::<Vec<_>>();
    let init = contract.init.as_ref().map(|init| {
        let params = init
            .params
            .iter()
            .map(|param| json!({ "name": param.name, "type": render_type(&param.ty) }))
            .collect::<Vec<_>>();
        let body = expr_to_source(&init.body, 0);
        let identity = json!({
            "contract": contract.name,
            "params": params,
            "version": contract.version,
        });
        let init_id_input = format!(
            "{INIT_ID_DOMAIN}\0{}",
            sha256_hex(canonical_json_bytes(&identity).as_slice())
        );
        json!({
            "body_digest": format!("sha256:{}", sha256_hex(body.as_bytes())),
            "init_id": format!("sha256:{}", sha256_hex(init_id_input.as_bytes())),
            "params": identity["params"].clone(),
        })
    });
    let migration = contract.migration.as_ref().map(|migration| {
        let ast_digest = sha256_hex(format!("{:?}", migration.body).as_bytes());
        json!({
            "ast_digest": format!("sha256:{ast_digest}"),
            "from_schema": migration.from_schema,
            "kind": "exclusive_state_transition",
        })
    });
    let schema_identity = json!({
        "contract": {
            "name": contract.name,
            "version": contract.version,
        },
        "compiler": {
            "name": "clg-cli",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "events": event_abi_identity["events"].clone(),
        "fields": layout_identity["fields"].clone(),
        "init": init,
        "invariants": invariants,
        "migration": migration,
        "schema_version": SCHEMA_VERSION,
    });
    let schema_digest = sha256_hex(canonical_json_bytes(&schema_identity).as_slice());
    Ok(json!({
        "contract": {
            "name": contract.name,
            "version": contract.version,
        },
        "compiler": schema_identity["compiler"].clone(),
        "fields": layout_identity["fields"].clone(),
        "event_abi": {
            "algorithm": EVENT_ABI_ALGORITHM,
            "digest": format!("sha256:{event_abi_digest}"),
            "event_order": "declaration",
        },
        "events": event_abi_identity["events"].clone(),
        "init": schema_identity["init"].clone(),
        "invariants": schema_identity["invariants"].clone(),
        "layout": {
            "algorithm": LAYOUT_ALGORITHM,
            "digest": format!("sha256:{layout_digest}"),
            "field_order": "declaration",
            "target_profile": TARGET_PROFILE_ID,
        },
        "migration": schema_identity["migration"].clone(),
        "schema": {
            "digest": format!("sha256:{schema_digest}"),
            "format": "clg.contract-state-schema.v1",
        },
        "schema_version": SCHEMA_VERSION,
        "source_graph": source_graph,
    }))
}

fn event_id(contract: &ContractDecl, event_name: &str) -> String {
    let input = format!("{EVENT_ID_DOMAIN}\0{}\0{event_name}", contract.name);
    format!("sha256:{}", sha256_hex(input.as_bytes()))
}

fn event_field_id(event_id: &str, field_name: &str) -> String {
    let input = format!("{EVENT_FIELD_ID_DOMAIN}\0{event_id}\0{field_name}");
    format!("sha256:{}", sha256_hex(input.as_bytes()))
}

fn field_id(contract: &ContractDecl, field_name: &str) -> String {
    let input = format!("{FIELD_ID_DOMAIN}\0{}\0{field_name}", contract.name);
    format!("sha256:{}", sha256_hex(input.as_bytes()))
}

fn render_type(ty: &Type) -> String {
    match ty {
        Type::Int => "Int".to_string(),
        Type::U8 => "U8".to_string(),
        Type::U64 => "U64".to_string(),
        Type::U128 => "U128".to_string(),
        Type::U256 => "U256".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::String => "String".to_string(),
        Type::Bytes => "Bytes".to_string(),
        Type::Named { name, args } => render_named_type(name, args),
        Type::Option(inner) => format!("Option<{}>", render_type(inner)),
        Type::Result(ok, err) => format!("Result<{},{}>", render_type(ok), render_type(err)),
        Type::List(inner) => format!("List<{}>", render_type(inner)),
        Type::Set(inner) => format!("Set<{}>", render_type(inner)),
        Type::Map(key, value) => format!("Map<{},{}>", render_type(key), render_type(value)),
        Type::Array(inner, Some(length)) => format!("[{};{}]", render_type(inner), length),
        Type::Array(inner, None) => format!("Array<{}>", render_type(inner)),
        Type::Slice(inner) => format!("Slice<{}>", render_type(inner)),
        Type::Tuple(items) => format!(
            "({})",
            items.iter().map(render_type).collect::<Vec<_>>().join(",")
        ),
        Type::Fn { params, ret } => format!(
            "function({})->{}",
            params.iter().map(render_type).collect::<Vec<_>>().join(","),
            render_type(ret)
        ),
    }
}

fn render_named_type(name: &str, args: &[Type]) -> String {
    if args.is_empty() {
        name.to_string()
    } else {
        format!(
            "{}<{}>",
            name,
            args.iter().map(render_type).collect::<Vec<_>>().join(",")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_byte_stable_and_preserves_declaration_order() {
        let source = r#"
            contract Vault version 1 {
                state { owner: Bytes; total: U64; balances: Map<Bytes, U64>; }
                invariant { state.total >= U64(0) }
                init(owner: Bytes, total: U64) {
                    state.owner = owner;
                    state.total = total;
                    state.balances = std::map::new();
                    0
                }
            }
        "#;
        let program = clg_parser::parse(source).expect("parse contract");
        let dir = tempfile::tempdir().expect("tempdir");
        let first = dir.path().join("one.json");
        let second = dir.path().join("two.json");
        write_contract_state_schema(&program, &first, None).expect("emit first schema");
        write_contract_state_schema(&program, &second, None).expect("emit second schema");
        let one = fs::read(first).expect("read first schema");
        let two = fs::read(second).expect("read second schema");
        assert_eq!(one, two, "schema bytes must be deterministic");
        let value: Value = serde_json::from_slice(&one).expect("parse schema json");
        assert_eq!(value["fields"][0]["name"], "owner");
        assert_eq!(value["fields"][1]["name"], "total");
        assert_eq!(value["fields"][2]["name"], "balances");
        assert_eq!(value["layout"]["field_order"], "declaration");
        assert_eq!(value["invariants"][0]["expression"], "state.total >= U64(0)");
        assert!(value["invariants"][0]["invariant_id"]
            .as_str()
            .expect("invariant id")
            .starts_with("sha256:"));
        assert!(value["init"]["init_id"]
            .as_str()
            .expect("init id")
            .starts_with("sha256:"));
    }
}
