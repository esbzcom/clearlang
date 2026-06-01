use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize)]
struct VerifiedStdAbiManifest {
    schema_version: u32,
    modules: Vec<VerifiedStdAbiModule>,
}

#[derive(Deserialize)]
struct VerifiedStdAbiModule {
    path: String,
    exports: Vec<VerifiedStdAbiExport>,
}

#[derive(Deserialize)]
struct VerifiedStdAbiExport {
    name: String,
    kind: VerifiedStdAbiExportKind,
}

#[derive(Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum VerifiedStdAbiExportKind {
    Type,
    Value,
}

fn load_verified_std_abi_value_symbols_from_str(
    raw: &str,
) -> std::result::Result<BTreeSet<String>, String> {
    let raw: VerifiedStdAbiManifest = serde_json::from_str(raw)
        .map_err(|err| format!("invalid verified std abi manifest JSON: {err}"))?;
    if raw.schema_version != 1 {
        return Err(format!(
            "unsupported verified std abi manifest schema version {}",
            raw.schema_version
        ));
    }
    let mut out = BTreeSet::new();
    for module in raw.modules {
        for export in module.exports {
            if export.kind == VerifiedStdAbiExportKind::Value {
                out.insert(format!("{}::{}", module.path, export.name));
            }
        }
    }
    Ok(out)
}

pub(crate) fn verified_std_abi_value_symbols() -> &'static BTreeSet<String> {
    static VERIFIED_STD_ABI_VALUE_SYMBOLS: OnceLock<BTreeSet<String>> = OnceLock::new();
    VERIFIED_STD_ABI_VALUE_SYMBOLS.get_or_init(|| {
        load_verified_std_abi_value_symbols_from_str(include_str!(
            "../../../../../docs/design/phase-27.1-verified-std-abi.manifest.v1.json"
        ))
        .expect("bundled verified std abi manifest must load")
    })
}

#[cfg(test)]
mod tests {
    use super::{load_verified_std_abi_value_symbols_from_str, verified_std_abi_value_symbols};

    #[test]
    fn rejects_unsupported_schema_version() {
        let err = load_verified_std_abi_value_symbols_from_str(
            r#"{
              "schema_version": 2,
              "modules": []
            }"#,
        )
        .expect_err("invalid schema version should fail");
        assert!(
            err.contains("unsupported verified std abi manifest schema version"),
            "expected schema version validation error, got: {err}"
        );
    }

    #[test]
    fn bundled_manifest_contains_verified_symbols_and_excludes_external_candidates() {
        let symbols = verified_std_abi_value_symbols();
        assert!(
            symbols.contains("std::host::env::chain_id"),
            "verified std abi manifest must include host boundary value symbols"
        );
        assert!(
            !symbols.contains("std::contract::address::from_bytes"),
            "verified std abi manifest must exclude external std package candidate symbols"
        );
    }
}
