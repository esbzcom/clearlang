use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use clg_ast::Program;
use clg_ast::{Effect, Param, Type};
use clg_typer::StdTypeInfo;

mod error;
mod graph;
mod imports;
mod package_metadata;
mod resolve;
mod std_metadata;

#[derive(Clone)]
pub struct ExternalImportBinding {
    pub function: String,
    pub import_module: String,
    pub import_name: String,
    pub params: Vec<Param>,
    pub ret: Type,
    pub effect: Effect,
}

pub struct ProgramLoad {
    pub program: Program,
    pub std_types: HashMap<String, StdTypeInfo>,
    pub external_imports: Vec<ExternalImportBinding>,
}

pub fn load_program(entry: &Path, json_errors: bool) -> Result<ProgramLoad> {
    graph::load_program(entry, json_errors)
}

pub fn std_type_info() -> HashMap<String, StdTypeInfo> {
    std_metadata::std_type_info()
}

struct Exports {
    values: HashSet<String>,
    types: HashSet<String>,
}

struct ModuleUnit {
    path_str: String,
    file: PathBuf,
    program: Program,
    is_entry: bool,
    exports: Exports,
    local_values: HashSet<String>,
    local_types: HashSet<String>,
}

struct ImportEnv {
    module_aliases: HashMap<String, String>,
    imported_values: HashMap<String, String>,
    imported_types: HashMap<String, String>,
}

struct ResolveCtx<'a> {
    prefix: &'a str,
    local_values: &'a HashMap<String, String>,
    local_types: &'a HashMap<String, String>,
    imported_values: &'a HashMap<String, String>,
    imported_types: &'a HashMap<String, String>,
    module_aliases: &'a HashMap<String, String>,
}

fn target_prefix(module: &ModuleUnit) -> &str {
    if module.is_entry {
        ""
    } else {
        module.path_str.as_str()
    }
}

fn qualify_name(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{}::{}", prefix, name)
    }
}
