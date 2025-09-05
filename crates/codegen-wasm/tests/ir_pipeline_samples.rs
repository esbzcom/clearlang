use std::fs;
use std::path::{Path, PathBuf};

use lumi_codegen_wasm::emit_from_ir;
use lumi_parser::parse;
use lumi_typer::check;

fn sample_path(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../lumi-tests")
        .join(file)
}

fn read_sample(file: &str) -> String {
    fs::read_to_string(sample_path(file)).expect("read sample")
}

struct Case {
    file: &'static str,
    parse_ok: bool,
    expected: Option<i32>,
}

fn run_wasm_and_get_i32_result(wasm: &[u8]) -> i32 {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::from_binary(&engine, wasm).expect("module from bytes");
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    main.call(&mut store, ()).expect("invoke main")
}

// Phase 3.6 — purpose: IR pipeline e2e across samples 01–05; 06 negative parse
#[test]
fn ir_pipeline_samples() {
    let cases = [
        Case { file: "01_hello.lumi", parse_ok: true,  expected: Some(42) },
        Case { file: "02_arith.lumi", parse_ok: true,  expected: Some(15) },
        Case { file: "03_nested_calls.lumi", parse_ok: true,  expected: Some(7) },
        Case { file: "04_multiline_call.lumi", parse_ok: true,  expected: Some(30) },
        Case { file: "05_trailing_param_comma.lumi", parse_ok: true,  expected: Some(3) },
        Case { file: "09_large_arith.lumi", parse_ok: true,  expected: Some(28) },
        Case { file: "12_zero_arg_fn.lumi", parse_ok: true,  expected: Some(7) },
        Case { file: "06_trailing_call_comma.lumi", parse_ok: false, expected: None },
        Case { file: "07_bools.lumi", parse_ok: true,  expected: None }, // no main export expected
        Case { file: "08_main_const.lumi", parse_ok: true,  expected: Some(42) },
    ];

    for c in cases {        
        let src = read_sample(c.file);
        let parsed = parse(&src);

        if !c.parse_ok {
            assert!(parsed.is_err(), "parse should fail for {}", c.file);
            continue;
        }

        let ast = parsed.expect("parse ok");
        if let Some(expect) = c.expected {
            let ir = check(&ast).expect("type-check+lower ok");
            let wasm = emit_from_ir(&ir).expect("codegen (IR) ok");
            let out = run_wasm_and_get_i32_result(&wasm);
            assert_eq!(out, expect, "output mismatch for {}", c.file);
        } else {
            // Expected to parse but not have a main export (e.g., 07_bools)
            let ir = check(&ast).expect("type-check+lower ok");
            let wasm = emit_from_ir(&ir).expect("codegen (IR) ok");
            let engine = wasmtime::Engine::default();
            let module = wasmtime::Module::from_binary(&engine, &wasm).expect("module from bytes");
            let mut store = wasmtime::Store::new(&engine, ());
            let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
            assert!(
                instance.get_typed_func::<(), i32>(&mut store, "main").is_err(),
                "module should not export main for {}",
                c.file
            );
        }
    }
}
