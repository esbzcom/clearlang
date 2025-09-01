use std::fs;
use std::path::PathBuf;

use lumi_codegen_wasm::emit_from_ast;
use lumi_parser::parse;

#[test]
fn parse_build_run_constant_main() {
    // Locate the sample source file from workspace root
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../lumi-tests/08_main_const.lumi");

    let src = fs::read_to_string(&path).expect("read sample");
    let ast = parse(&src).expect("parse ok");

    let wasm = emit_from_ast(&ast).expect("codegen ok");

    // Execute via wasmtime
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::from_binary(&engine, &wasm).expect("module from bytes");
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let out = main.call(&mut store, ()).expect("invoke main");
    assert_eq!(out, 42);
}

