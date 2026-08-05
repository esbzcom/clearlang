use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use clg_ast::{ContractDecl, Program, Type};
use serde_json::json;
#[cfg(test)]
use serde_json::Value;

use crate::commands::helpers::{canonical_json_bytes, sha256_hex};

const SCHEMA_VERSION: u32 = 1;
const LAYOUT_ALGORITHM: &str = "clg.contract-state-layout.v1";
const FIELD_ID_DOMAIN: &str = "clg.contract-state-field.v1";

pub(super) fn write_contract_state_schema(program: &Program, path: &Path) -> Result<()> {
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
    let artifact = json!({
        "contract": {
            "name": contract.name,
            "version": contract.version,
        },
        "fields": layout_identity["fields"].clone(),
        "layout": {
            "algorithm": LAYOUT_ALGORITHM,
            "digest": format!("sha256:{layout_digest}"),
            "field_order": "declaration",
        },
        "schema_version": SCHEMA_VERSION,
    });
    fs::write(path, canonical_json_bytes(&artifact))
        .with_context(|| format!("write contract state schema `{}`", path.display()))
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
            }
        "#;
        let program = clg_parser::parse(source).expect("parse contract");
        let dir = tempfile::tempdir().expect("tempdir");
        let first = dir.path().join("one.json");
        let second = dir.path().join("two.json");
        write_contract_state_schema(&program, &first).expect("emit first schema");
        write_contract_state_schema(&program, &second).expect("emit second schema");
        let one = fs::read(first).expect("read first schema");
        let two = fs::read(second).expect("read second schema");
        assert_eq!(one, two, "schema bytes must be deterministic");
        let value: Value = serde_json::from_slice(&one).expect("parse schema json");
        assert_eq!(value["fields"][0]["name"], "owner");
        assert_eq!(value["fields"][1]["name"], "total");
        assert_eq!(value["fields"][2]["name"], "balances");
        assert_eq!(value["layout"]["field_order"], "declaration");
    }
}
