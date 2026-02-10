use super::*;

#[test]
fn list_len_reads_header() {
    let src = r#"
        function main(l: List<Int>) -> Int { std::list::len(l) }
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
    let (list_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[10, 20]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let len = main.call(&mut store, list_ptr).expect("call main");

    assert_eq!(len, 2, "len should come from header");
}

#[test]
fn list_len_invalid_handle_traps() {
    let src = r#"
        function main(l: List<Int>) -> Int { std::list::len(l) }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, 1);
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 11, "expected R010 (InvalidBuffer) trap code");
}

#[test]
fn list_get_invalid_data_ptr_traps() {
    let src = r#"
        function main(l: List<Int>) -> Option<Int> { std::list::get(l, 0) }
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
    let (list_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[10]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let mem_bytes = (memory.size(&mut store) as i32) * 65536;
    write_i32(&memory, &mut store, list_ptr + 12, mem_bytes);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, list_ptr);
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 11, "expected R010 (InvalidBuffer) trap code");
}

#[test]
fn list_get_null_data_ptr_traps() {
    let src = r#"
        function main(l: List<Int>) -> Option<Int> { std::list::get(l, 0) }
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
    let (list_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[10]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    write_i32(&memory, &mut store, list_ptr + 12, 0);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, list_ptr);
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 11, "expected R010 (InvalidBuffer) trap code");
}

#[test]
fn list_get_len_exceeds_cap_traps() {
    let src = r#"
        function main(l: List<Int>) -> Option<Int> { std::list::get(l, 0) }
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
    let (list_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[10, 20]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    write_i32(&memory, &mut store, list_ptr + 4, 1);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, list_ptr);
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 11, "expected R010 (InvalidBuffer) trap code");
}

#[test]
fn list_get_len_overflow_traps() {
    let src = r#"
        function main(l: List<Int>) -> Option<Int> { std::list::get(l, 0) }
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
    let (list_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[10]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    write_i32(&memory, &mut store, list_ptr + 0, 600_000_000);
    write_i32(&memory, &mut store, list_ptr + 4, 600_000_000);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, list_ptr);
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 11, "expected R010 (InvalidBuffer) trap code");
}

#[test]
fn set_contains_null_data_ptr_traps() {
    let src = r#"
        function main(s: Set<Int>) -> Bool { std::set::contains(s, 1) }
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
    let (set_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[1]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    write_i32(&memory, &mut store, set_ptr + 12, 0);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, set_ptr);
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 11, "expected R010 (InvalidBuffer) trap code");
}

#[test]
fn set_contains_misaligned_data_ptr_traps() {
    let src = r#"
        function main(s: Set<Int>) -> Bool { std::set::contains(s, 1) }
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
    let (set_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[1]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    write_i32(&memory, &mut store, set_ptr + 12, set_ptr + 18);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, set_ptr);
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 11, "expected R010 (InvalidBuffer) trap code");
}

#[test]
fn set_contains_len_exceeds_cap_traps() {
    let src = r#"
        function main(s: Set<Int>) -> Bool { std::set::contains(s, 1) }
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

    write_i32(&memory, &mut store, set_ptr + 4, 1);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, set_ptr);
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 11, "expected R010 (InvalidBuffer) trap code");
}

#[test]
fn list_insert_oob_traps() {
    let src = r#"
        function main(l: List<Int>) -> List<Int> { std::list::insert(l, 1, 5) }
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
    let (list_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[1, 2]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, list_ptr);
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 10, "expected R009 (CollectionBounds) trap code");
}
