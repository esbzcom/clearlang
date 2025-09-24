use clg_codegen_wasm::emit_from_ir;
use clg_parser::parse;
use clg_typer::check;
mod common;

fn run_main_i32(wasm: &[u8]) -> i32 {
    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, wasm).expect("module from bytes");
    let mut store = wasmtime::Store::new(engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    main.call(&mut store, ()).expect("invoke main")
}

#[test]
fn if_true_returns_then_branch() {
    let src = r#"
        function main() -> Int { if true { 1 } else { 2 } }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm), 1);
}

#[test]
fn if_false_returns_else_branch() {
    let src = r#"
        function main() -> Int { if false { 1 } else { 2 } }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm), 2);
}

#[test]
fn else_if_chain_selects_middle() {
    let src = r#"
        function main() -> Int { if false { 1 } else if true { 3 } else { 4 } }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm), 3);
}

#[test]
fn select_string_then_len() {
    let src = r#"
        function main() -> Int { std::str::len(if false { "hello" } else { "x" }) }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm), 1);
}
