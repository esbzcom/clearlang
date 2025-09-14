use lumi_codegen_wasm::emit_from_ir;
use lumi_parser::parse;
use lumi_typer::check;

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
fn str_len_of_literal_returns_byte_length() {
    let src = r#"
        pure function main() -> Int { std::str::len("Hello") }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");
    let out = run_wasm_and_get_i32_result(&wasm);
    assert_eq!(out, 5);
}

