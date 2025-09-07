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
fn ir_pipeline_hello() {
    let src = r#"
        pure function add2(a: Int, b: Int) -> Int { a + b }
        function main() -> Int { add2(20, 22) }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("ir codegen ok");
    let out = run_wasm_and_get_i32_result(&wasm);
    assert_eq!(out, 42);
}
