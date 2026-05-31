use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use clg_typer::StdTypeInfo;
use serde::Deserialize;

#[derive(Deserialize)]
struct StdMetadata {
    schema_version: u32,
    modules: Vec<StdModule>,
}

#[derive(Deserialize)]
struct StdModule {
    path: String,
    exports: Vec<StdExport>,
}

#[derive(Deserialize)]
struct StdExport {
    name: String,
    kind: StdExportKind,
    #[serde(default)]
    layout: Option<StdTypeLayout>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum StdExportKind {
    Type,
    Value,
}

#[derive(Deserialize, Clone)]
struct StdTypeLayout {
    bytes: u32,
    align: u32,
}

pub(super) struct StdModuleIndex {
    pub(super) values: HashSet<String>,
    pub(super) types: HashSet<String>,
}

pub(super) struct StdMetadataIndex {
    modules: HashMap<String, StdModuleIndex>,
    types: HashMap<String, StdTypeInfo>,
}

impl StdMetadataIndex {
    fn load_from_str(raw: &str) -> std::result::Result<Self, String> {
        let raw: StdMetadata =
            serde_json::from_str(raw).map_err(|err| format!("invalid std metadata JSON: {err}"))?;
        Self::from_raw(raw)
    }

    fn load() -> std::result::Result<Self, String> {
        Self::load_from_str(include_str!("../../../assets/std-metadata.json"))
    }

    fn from_raw(raw: StdMetadata) -> std::result::Result<Self, String> {
        if raw.schema_version != 2 {
            return Err(format!(
                "unsupported std metadata schema version {}",
                raw.schema_version
            ));
        }

        let mut modules = HashMap::new();
        let mut types = HashMap::new();

        for module in raw.modules {
            let mut values = HashSet::new();
            let mut types_set = HashSet::new();
            for export in module.exports {
                match export.kind {
                    StdExportKind::Value => {
                        values.insert(export.name);
                    }
                    StdExportKind::Type => {
                        let layout = export.layout.ok_or_else(|| {
                            format!("std type `{}` is missing layout metadata", export.name)
                        })?;
                        if layout.bytes == 0 {
                            return Err(format!("std type `{}` has zero-byte layout", export.name));
                        }
                        if layout.align == 0 {
                            return Err(format!("std type `{}` has zero alignment", export.name));
                        }

                        let qualified = format!("{}::{}", module.path, export.name);
                        if types
                            .insert(
                                qualified,
                                StdTypeInfo {
                                    byte_len: layout.bytes,
                                    align: layout.align,
                                },
                            )
                            .is_some()
                        {
                            return Err(format!("duplicate std type `{}`", export.name));
                        }
                        types_set.insert(export.name);
                    }
                }
            }

            modules.insert(
                module.path,
                StdModuleIndex {
                    values,
                    types: types_set,
                },
            );
        }

        Ok(StdMetadataIndex { modules, types })
    }

    pub(super) fn module(&self, path: &str) -> Option<&StdModuleIndex> {
        self.modules.get(path)
    }
}

pub(super) fn std_metadata() -> Result<&'static StdMetadataIndex> {
    static STD_METADATA: OnceLock<std::result::Result<StdMetadataIndex, String>> = OnceLock::new();
    match STD_METADATA.get_or_init(StdMetadataIndex::load) {
        Ok(metadata) => Ok(metadata),
        Err(message) => Err(anyhow!("invalid std metadata: {message}")),
    }
}

pub(super) fn std_type_info() -> Result<HashMap<String, StdTypeInfo>> {
    Ok(std_metadata()?.types.clone())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::StdMetadataIndex;

    #[test]
    fn rejects_unsupported_schema_version() {
        let err = match StdMetadataIndex::load_from_str(
            r#"{
              "schema_version": 1,
              "modules": []
            }"#,
        ) {
            Ok(_) => panic!("invalid schema version should fail"),
            Err(err) => err,
        };
        assert!(
            err.contains("unsupported std metadata schema version"),
            "expected schema version validation error, got: {err}"
        );
    }

    #[test]
    fn rejects_type_without_layout() {
        let err = match StdMetadataIndex::load_from_str(
            r#"{
              "schema_version": 2,
              "modules": [
                {
                  "path": "std::foo",
                  "exports": [
                    {"name":"Bar","kind":"type"}
                  ]
                }
              ]
            }"#,
        ) {
            Ok(_) => panic!("missing type layout should fail"),
            Err(err) => err,
        };
        assert!(
            err.contains("missing layout metadata"),
            "expected missing layout validation error, got: {err}"
        );
    }

    #[test]
    fn accepts_minimal_valid_metadata() {
        let index = StdMetadataIndex::load_from_str(
            r#"{
              "schema_version": 2,
              "modules": [
                {
                  "path": "std::foo",
                  "exports": [
                    {"name":"make","kind":"value"},
                    {"name":"Bar","kind":"type","layout":{"bytes":8,"align":4}}
                  ]
                }
              ]
            }"#,
        )
        .expect("valid metadata");
        let module = index.module("std::foo").expect("std::foo module present");
        assert!(module.values.contains("make"));
        assert!(module.types.contains("Bar"));
    }

    #[test]
    fn std_unit_exports_match_supported_assert_surface() {
        let index = StdMetadataIndex::load().expect("bundled std metadata must load");
        let module = index.module("std::unit").expect("std::unit module present");
        let expected: BTreeSet<&str> = [
            "assert_true",
            "assert_false",
            "assert_eq_int",
            "assert_eq_u64",
            "assert_eq_bool",
            "fail",
        ]
        .into_iter()
        .collect();
        let actual: BTreeSet<&str> = module.values.iter().map(|v| v.as_str()).collect();
        assert_eq!(
            actual, expected,
            "std::unit metadata exports must stay in sync with supported assert/fail surface"
        );
    }
}
