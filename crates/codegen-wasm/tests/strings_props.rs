use clg_codegen_wasm::emit_from_ir;
use clg_parser::parse;
use clg_typer::check;
use proptest::prelude::*;

mod common;

fn run_main_i32(wasm: &[u8]) -> i32 {
    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, wasm).expect("module from bytes");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    main.call(&mut store, ()).expect("invoke main")
}

fn clg_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn lit(s: &str) -> String {
    format!("\"{}\"", clg_escape(s))
}

fn compile_and_run_bool_eq(a: &str, b: &str) -> i32 {
    let src = format!(
        "function main() -> Bool {{ std::str::eq({}, {}) }}",
        lit(a),
        lit(b)
    );
    let ast = parse(&src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");
    run_main_i32(&wasm)
}

fn compile_and_run_len_of_concat(a: &str, b: &str) -> i32 {
    let src = format!(
        "function main() -> Int {{ std::str::len(std::str::concat({}, {})) }}",
        lit(a),
        lit(b)
    );
    let ast = parse(&src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");
    run_main_i32(&wasm)
}

const ALPHABET: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', ' ', '_', '-', '.', ',', '!', '?',
];

fn small_ascii_string() -> impl Strategy<Value = String> {
    proptest::collection::vec(proptest::sample::select(ALPHABET), 0..12)
        .prop_map(|v| v.into_iter().collect())
}

proptest! {
    #[test]
    fn prop_eq_reflexive(s in small_ascii_string()) {
        let out = compile_and_run_bool_eq(&s, &s);
        prop_assert_eq!(out, 1);
    }

    #[test]
    fn prop_eq_symmetric(a in small_ascii_string(), b in small_ascii_string()) {
        let ab = compile_and_run_bool_eq(&a, &b);
        let ba = compile_and_run_bool_eq(&b, &a);
        prop_assert_eq!(ab, ba);
    }

    #[test]
    fn prop_concat_len_adds(a in small_ascii_string(), b in small_ascii_string()) {
        let out = compile_and_run_len_of_concat(&a, &b);
        let expect = (a.len() + b.len()) as i32;
        prop_assert_eq!(out, expect);
    }
}
