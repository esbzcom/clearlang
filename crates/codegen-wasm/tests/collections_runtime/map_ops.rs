use super::*;

#[test]
fn map_insert_appends_entry() {
    let src = r#"
        function main(m: Map<Int, Int>) -> Map<Int, Int> { std::map::insert(m, 2, 20) }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let new_map = main.call(&mut store, map_ptr).expect("call main");

    let len = read_i32(&memory, &mut store, new_map + 0);
    let data_ptr = read_i32(&memory, &mut store, new_map + 12);
    let k0 = read_i32(&memory, &mut store, data_ptr + 0);
    let v0 = read_i32(&memory, &mut store, data_ptr + 4);
    let k1 = read_i32(&memory, &mut store, data_ptr + 8);
    let v1 = read_i32(&memory, &mut store, data_ptr + 12);

    assert_eq!(len, 2, "insert should increase len");
    assert_eq!(k0, 1);
    assert_eq!(v0, 10);
    assert_eq!(k1, 2);
    assert_eq!(v1, 20);
}

#[test]
fn map_insert_replaces_value() {
    let src = r#"
        function main(m: Map<Int, Int>) -> Map<Int, Int> { std::map::insert(m, 1, 99) }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let new_map = main.call(&mut store, map_ptr).expect("call main");

    let len = read_i32(&memory, &mut store, new_map + 0);
    let data_ptr = read_i32(&memory, &mut store, new_map + 12);
    let key = read_i32(&memory, &mut store, data_ptr + 0);
    let val = read_i32(&memory, &mut store, data_ptr + 4);

    assert_eq!(len, 1, "replace should keep len");
    assert_eq!(key, 1);
    assert_eq!(val, 99);
}

#[test]
fn map_remove_existing() {
    let src = r#"
        function main(m: Map<Int, Int>) -> Map<Int, Int> { std::map::remove(m, 1) }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10), (2, 20)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let new_map = main.call(&mut store, map_ptr).expect("call main");

    let len = read_i32(&memory, &mut store, new_map + 0);
    let data_ptr = read_i32(&memory, &mut store, new_map + 12);
    let key = read_i32(&memory, &mut store, data_ptr + 0);
    let val = read_i32(&memory, &mut store, data_ptr + 4);

    assert_eq!(len, 1, "remove should decrease len");
    assert_eq!(key, 2);
    assert_eq!(val, 20);
}

#[test]
fn map_contains_reports_membership() {
    let src = r#"
        function main(m: Map<Int, Int>) -> Bool { std::map::contains(m, 2) }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10), (2, 20)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let ok = main.call(&mut store, map_ptr).expect("call main");
    assert_eq!(ok, 1, "expected contains to return true");
}

#[test]
fn map_contains_misaligned_data_ptr_traps() {
    let src = r#"
        function main(m: Map<Int, Int>) -> Bool { std::map::contains(m, 1) }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    write_i32(&memory, &mut store, map_ptr + 12, map_ptr + 18);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, map_ptr);
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 11, "expected R010 (InvalidBuffer) trap code");
}

#[test]
fn map_contains_len_exceeds_cap_traps() {
    let src = r#"
        function main(m: Map<Int, Int>) -> Bool { std::map::contains(m, 1) }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10), (2, 20)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    write_i32(&memory, &mut store, map_ptr + 4, 1);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, map_ptr);
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 11, "expected R010 (InvalidBuffer) trap code");
}

#[test]
fn map_get_invalid_data_ptr_traps() {
    let src = r#"
        function main(m: Map<Int, Int>, k: Int) -> Option<Int> { std::map::get(m, k) }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let mem_bytes = (memory.size(&mut store) as i32) * 65536;
    write_i32(&memory, &mut store, map_ptr + 12, mem_bytes);

    let main = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, (map_ptr, 1));
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 11, "expected R010 (InvalidBuffer) trap code");
}

#[test]
fn map_get_reports_option() {
    let src = r#"
        function main(m: Map<Int, Int>) -> Option<Int> { std::map::get(m, 2) }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10), (2, 20)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let opt_ptr = main.call(&mut store, map_ptr).expect("call main");

    let (tag, payload) = read_option_i32(&memory, &mut store, opt_ptr);
    assert_eq!(tag, 1, "expected Some tag");
    assert_eq!(payload, 20, "expected map value");
}

#[test]
fn map_get_missing_returns_none() {
    let src = r#"
        function main(m: Map<Int, Int>) -> Option<Int> { std::map::get(m, 9) }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10), (2, 20)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let opt_ptr = main.call(&mut store, map_ptr).expect("call main");

    let (tag, payload) = read_option_i32(&memory, &mut store, opt_ptr);
    assert_eq!(tag, 0, "expected None tag");
    assert_eq!(payload, 0, "None payload should be zero");
}

#[test]
fn map_remove_missing_is_noop() {
    let src = r#"
        function main(m: Map<Int, Int>) -> Map<Int, Int> { std::map::remove(m, 9) }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10), (2, 20)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let new_map = main.call(&mut store, map_ptr).expect("call main");

    let len = read_i32(&memory, &mut store, new_map + 0);
    let data_ptr = read_i32(&memory, &mut store, new_map + 12);
    let k0 = read_i32(&memory, &mut store, data_ptr + 0);
    let v0 = read_i32(&memory, &mut store, data_ptr + 4);
    let k1 = read_i32(&memory, &mut store, data_ptr + 8);
    let v1 = read_i32(&memory, &mut store, data_ptr + 12);

    assert_eq!(len, 2, "remove missing should keep len");
    let entries_ok = (k0 == 1 && v0 == 10 && k1 == 2 && v1 == 20)
        || (k0 == 2 && v0 == 20 && k1 == 1 && v1 == 10);
    assert!(entries_ok, "entries should be preserved");
}

#[test]
fn map_insert_take_returns_replaced_value() {
    let src = r#"
        function main(m: Map<Int, Int>) -> Option<Int> { std::map::insert_take(m, 1, 99)[1] }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10), (2, 20)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let opt_ptr = main.call(&mut store, map_ptr).expect("call main");

    let (tag, payload) = read_option_i32(&memory, &mut store, opt_ptr);
    assert_eq!(tag, 1, "expected Some tag");
    assert_eq!(payload, 10, "expected replaced old value");
}

#[test]
fn map_remove_take_returns_removed_value() {
    let src = r#"
        function main(m: Map<Int, Int>) -> Option<Int> { std::map::remove_take(m, 2)[1] }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10), (2, 20)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let opt_ptr = main.call(&mut store, map_ptr).expect("call main");

    let (tag, payload) = read_option_i32(&memory, &mut store, opt_ptr);
    assert_eq!(tag, 1, "expected Some tag");
    assert_eq!(payload, 20, "expected removed value");
}

#[test]
fn map_insert_take_returns_updated_map() {
    let src = r#"
        function main(m: Map<Int, Int>) -> Map<Int, Int> { std::map::insert_take(m, 1, 99)[0] }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10), (2, 20)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let new_map = main.call(&mut store, map_ptr).expect("call main");

    let len = read_i32(&memory, &mut store, new_map + 0);
    let data_ptr = read_i32(&memory, &mut store, new_map + 12);
    let k0 = read_i32(&memory, &mut store, data_ptr + 0);
    let v0 = read_i32(&memory, &mut store, data_ptr + 4);
    let k1 = read_i32(&memory, &mut store, data_ptr + 8);
    let v1 = read_i32(&memory, &mut store, data_ptr + 12);

    assert_eq!(len, 2, "replace should keep len");
    let entries_ok = (k0 == 1 && v0 == 99 && k1 == 2 && v1 == 20)
        || (k0 == 2 && v0 == 20 && k1 == 1 && v1 == 99);
    assert!(entries_ok, "map should contain updated replacement");
}

#[test]
fn map_remove_take_returns_updated_map() {
    let src = r#"
        function main(m: Map<Int, Int>) -> Map<Int, Int> { std::map::remove_take(m, 1)[0] }
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
    let (map_ptr, new_heap) = alloc_map(&memory, &mut store, heap_ptr, &[(1, 10), (2, 20)]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let new_map = main.call(&mut store, map_ptr).expect("call main");

    let len = read_i32(&memory, &mut store, new_map + 0);
    let data_ptr = read_i32(&memory, &mut store, new_map + 12);
    let key = read_i32(&memory, &mut store, data_ptr + 0);
    let val = read_i32(&memory, &mut store, data_ptr + 4);

    assert_eq!(len, 1, "remove_take should decrease len");
    assert_eq!(key, 2);
    assert_eq!(val, 20);
}
