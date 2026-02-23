use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn repo_sample(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../clearlang-tests")
        .join(name)
}

mod common;

fn wasmtime_run(bytes: &[u8]) -> i32 {
    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, bytes).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("main export");
    main.call(&mut store, ()).expect("invoke main")
}

#[path = "cli_it/basic.rs"]
mod basic;
#[path = "cli_it/crypto.rs"]
mod crypto;
#[path = "cli_it/diagnostics.rs"]
mod diagnostics;
#[path = "cli_it/imports.rs"]
mod imports;
#[path = "cli_it/limits.rs"]
mod limits;
#[path = "cli_it/option_result.rs"]
mod option_result;
#[path = "cli_it/runtime_env.rs"]
mod runtime_env;
#[path = "cli_it/runtime_errors.rs"]
mod runtime_errors;
#[path = "cli_it/sdk_usability.rs"]
mod sdk_usability;
#[path = "cli_it/vc_outputs.rs"]
mod vc_outputs;
