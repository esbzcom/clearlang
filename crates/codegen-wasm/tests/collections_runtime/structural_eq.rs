use super::*;

#[test]
fn map_contains_option_key_structural_eq() {
    let src = r#"
        function make() -> Map<Option<Int>, Int> { std::map::new() }
        function main() -> Bool {
            let m = std::map::insert(make(), Some(1), 10);
            std::map::contains(m, Some(1))
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
    let ok = main.call(&mut store, ()).expect("call main");
    assert_eq!(ok, 1, "expected structural equality for Option key");
}

#[test]
fn map_contains_struct_key_structural_eq() {
    let src = r#"
        struct Point { x: Int; y: Int; }
        function make() -> Map<Point, Int> { std::map::new() }
        function main() -> Bool {
            let m = std::map::insert(make(), Point { x: 1, y: 2 }, 10);
            std::map::contains(m, Point { x: 1, y: 2 })
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
    let ok = main.call(&mut store, ()).expect("call main");
    assert_eq!(ok, 1, "expected structural equality for struct key");
}

#[test]
fn set_contains_enum_key_structural_eq() {
    let src = r#"
        enum Token { A, B }
        function make() -> Set<Token> { std::set::new() }
        function main() -> Bool {
            let s = std::set::insert(make(), Token::A());
            std::set::contains(s, Token::A())
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
    let ok = main.call(&mut store, ()).expect("call main");
    assert_eq!(ok, 1, "expected structural equality for enum key");
}

#[test]
fn set_contains_tuple_key_structural_eq() {
    let src = r#"
        function make() -> Set<(Int, Int)> { std::set::new() }
        function main() -> Bool {
            let s = std::set::insert(make(), (1, 2));
            std::set::contains(s, (1, 2))
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
    let ok = main.call(&mut store, ()).expect("call main");
    assert_eq!(ok, 1, "expected structural equality for tuple key");
}
