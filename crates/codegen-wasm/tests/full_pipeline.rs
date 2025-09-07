use std::fs;
use std::path::{Path, PathBuf};

use lumi_codegen_wasm::emit_from_ast;
use lumi_parser::parse;

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

#[test]
fn pipeline_expected_outputs() {
    // expected: None means codegen not supported yet (but parse may succeed)
    let cases = [
        Case { file: "01_hello.lumi", parse_ok: true,  expected: Some(42) }, // add2(20,22)
        Case { file: "02_arith.lumi", parse_ok: true,  expected: Some(15) }, // (2+3)*4 - 5
        Case { file: "03_nested_calls.lumi", parse_ok: true,  expected: Some(7) }, // add(1, mul(2,3))
        Case { file: "04_multiline_call.lumi", parse_ok: true,  expected: Some(30) }, // add(10, (2+3)*4)
        Case { file: "05_trailing_param_comma.lumi", parse_ok: true,  expected: Some(3) }, // add(1,2)
        Case { file: "06_trailing_call_comma.lumi", parse_ok: false, expected: None },
        Case { file: "07_bools.lumi", parse_ok: true,  expected: None }, // not Int-returning main
        Case { file: "08_main_const.lumi", parse_ok: true,  expected: Some(42) },
        Case { file: "09_large_arith.lumi", parse_ok: true,  expected: Some(28) },
        Case { file: "12_zero_arg_fn.lumi", parse_ok: true,  expected: Some(7) },
        Case { file: "13_bool_arith_invalid.lumi", parse_ok: true,  expected: None },
        Case { file: "14_arity_mismatch.lumi", parse_ok: true,  expected: None },
        Case { file: "15_str_literal.lumi", parse_ok: true,  expected: None },
        Case { file: "16_namespaced_call.lumi", parse_ok: true,  expected: None },
    ];

    for c in cases {        
        let src = read_sample(c.file);
        let parsed = parse(&src);

        if !c.parse_ok {
            assert!(parsed.is_err(), "parse should fail for {}", c.file);
            continue;
        }

        let ast = parsed.expect("parse ok");
        match c.expected {
            Some(expect) => {
                let wasm = emit_from_ast(&ast).expect("codegen ok");
                let out = run_wasm_and_get_i32_result(&wasm);
                assert_eq!(out, expect, "output mismatch for {}", c.file);
            }
            None => {
                assert!(
                    emit_from_ast(&ast).is_err(),
                    "codegen should not yet support {}",
                    c.file
                );
            }
        }
    }
}
