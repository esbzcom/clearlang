use super::*;

#[test]
fn set_insert_avoids_duplicates() {
    let src = r#"
        function main(s: Set<Int>) -> Set<Int> { std::set::insert(s, 2) }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("memory export");

    let heap_ptr = get_global_i32(&instance, &mut store, "__clg_heap_ptr");
    let (set_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[1, 2]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let new_set = main.call(&mut store, set_ptr).expect("call main");

    let len = read_i32(&memory, &mut store, new_set + 0);
    let data_ptr = read_i32(&memory, &mut store, new_set + 12);
    let a = read_i32(&memory, &mut store, data_ptr + 0);
    let b = read_i32(&memory, &mut store, data_ptr + 4);

    assert_eq!(len, 2, "duplicate insert should keep len");
    assert_pair_unordered(a, b, 1, 2);
}

#[test]
fn set_contains_reports_membership() {
    let src = r#"
        function main(s: Set<Int>) -> Bool { std::set::contains(s, 2) }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("memory export");

    let heap_ptr = get_global_i32(&instance, &mut store, "__clg_heap_ptr");
    let (set_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[1, 2]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let ok = main.call(&mut store, set_ptr).expect("call main");
    assert_eq!(ok, 1, "expected contains to return true");
}

#[test]
fn set_remove_missing_is_noop() {
    let src = r#"
        function main(s: Set<Int>) -> Set<Int> { std::set::remove(s, 9) }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("memory export");

    let heap_ptr = get_global_i32(&instance, &mut store, "__clg_heap_ptr");
    let (set_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[1, 2]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let new_set = main.call(&mut store, set_ptr).expect("call main");

    let len = read_i32(&memory, &mut store, new_set + 0);
    let data_ptr = read_i32(&memory, &mut store, new_set + 12);
    let a = read_i32(&memory, &mut store, data_ptr + 0);
    let b = read_i32(&memory, &mut store, data_ptr + 4);

    assert_eq!(len, 2, "remove missing should keep len");
    assert_pair_unordered(a, b, 1, 2);
}

#[test]
fn set_remove_existing() {
    let src = r#"
        function main(s: Set<Int>) -> Set<Int> { std::set::remove(s, 1) }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("memory export");

    let heap_ptr = get_global_i32(&instance, &mut store, "__clg_heap_ptr");
    let (set_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[1, 2]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let new_set = main.call(&mut store, set_ptr).expect("call main");

    let len = read_i32(&memory, &mut store, new_set + 0);
    let data_ptr = read_i32(&memory, &mut store, new_set + 12);
    let only = read_i32(&memory, &mut store, data_ptr + 0);

    assert_eq!(len, 1, "remove should decrease len");
    assert_eq!(only, 2, "remaining element should be preserved");
}
