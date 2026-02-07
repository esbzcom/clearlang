use clg_codegen_wasm::emit_from_ir;
use clg_parser::parse;
use clg_typer::check;

mod common;

#[test]
fn array_len_returns_length() {
    let src = r#"
        function main() -> Int { std::array::len([1, 2, 3]) }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let len = main.call(&mut store, ()).expect("call main");
    assert_eq!(len, 3);
}

#[test]
fn slice_from_array_indexes_correctly() {
    let src = r#"
        function main() -> Int {
            let a = [10, 20, 30];
            let s = std::slice::from_array(a);
            s[1]
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let value = main.call(&mut store, ()).expect("call main");
    assert_eq!(value, 20);
}

#[test]
fn slice_sub_returns_window() {
    let src = r#"
        function main() -> Int {
            let a = [5, 6, 7, 8];
            let s = std::slice::from_array(a);
            let t = std::slice::sub(s, 1, 2);
            t[0] + t[1]
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let value = main.call(&mut store, ()).expect("call main");
    assert_eq!(value, 13);
}
