use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

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
    fn load() -> Self {
        let raw: StdMetadata =
            serde_json::from_str(include_str!("../../../assets/std-metadata.json"))
                .expect("invalid std metadata");
        if raw.schema_version != 2 {
            panic!(
                "unsupported std metadata schema version {}",
                raw.schema_version
            );
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
                        let layout = export.layout.unwrap_or_else(|| {
                            panic!("std type `{}` is missing layout metadata", export.name)
                        });
                        if layout.bytes == 0 {
                            panic!("std type `{}` has zero-byte layout", export.name);
                        }
                        if layout.align == 0 {
                            panic!("std type `{}` has zero alignment", export.name);
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
                            panic!("duplicate std type `{}`", export.name);
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

        StdMetadataIndex { modules, types }
    }

    pub(super) fn module(&self, path: &str) -> Option<&StdModuleIndex> {
        self.modules.get(path)
    }
}

pub(super) fn std_metadata() -> &'static StdMetadataIndex {
    static STD_METADATA: OnceLock<StdMetadataIndex> = OnceLock::new();
    STD_METADATA.get_or_init(StdMetadataIndex::load)
}

pub(super) fn std_type_info() -> HashMap<String, StdTypeInfo> {
    std_metadata().types.clone()
}
