use super::*;

#[test]
fn list_push_appends_payload() {
    let src = r#"
        function main(l: List<Int>) -> List<Int> { std::list::push(l, 42) }
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
    let new_list = main.call(&mut store, list_ptr).expect("call main");

    let len = read_i32(&memory, &mut store, new_list + 0);
    let data_ptr = read_i32(&memory, &mut store, new_list + 12);
    let a = read_i32(&memory, &mut store, data_ptr + 0);
    let b = read_i32(&memory, &mut store, data_ptr + 4);
    let c = read_i32(&memory, &mut store, data_ptr + 8);

    assert_eq!(len, 3, "push should increase len");
    assert_eq!(a, 1, "first element preserved");
    assert_eq!(b, 2, "second element preserved");
    assert_eq!(c, 42, "pushed element appended");
}

#[test]
fn list_remove_shifts_payload() {
    let src = r#"
        function main(l: List<Int>) -> List<Int> { std::list::remove(l, 1) }
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
    let (list_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[5, 6, 7]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let new_list = main.call(&mut store, list_ptr).expect("call main");

    let len = read_i32(&memory, &mut store, new_list + 0);
    let data_ptr = read_i32(&memory, &mut store, new_list + 12);
    let a = read_i32(&memory, &mut store, data_ptr + 0);
    let b = read_i32(&memory, &mut store, data_ptr + 4);

    assert_eq!(len, 2, "remove should decrease len");
    assert_eq!(a, 5, "first element preserved");
    assert_eq!(b, 7, "elements should shift left");
}

#[test]
fn list_remove_take_returns_removed_value() {
    let src = r#"
        function main(l: List<Int>) -> Option<Int> { std::list::remove_take(l, 1)[1] }
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
    let (list_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[5, 6, 7]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let opt_ptr = main.call(&mut store, list_ptr).expect("call main");

    let (tag, payload) = read_option_i32(&memory, &mut store, opt_ptr);
    assert_eq!(tag, 1, "expected Some tag");
    assert_eq!(payload, 6, "expected removed value");
}

#[test]
fn list_remove_take_resource_returns_removed_value() {
    let src = r#"
        resource File { drop {} }
        pure function main(consume l: List<File>) -> Option<File> { std::list::remove_take(l, 1)[1] }
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
    let (list_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[5, 6, 7]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let opt_ptr = main.call(&mut store, list_ptr).expect("call main");

    let (tag, payload) = read_option_i32(&memory, &mut store, opt_ptr);
    assert_eq!(tag, 1, "expected Some tag");
    assert_eq!(payload, 6, "expected removed resource handle");
}

#[test]
fn list_remove_take_returns_updated_list() {
    let src = r#"
        function main(l: List<Int>) -> List<Int> { std::list::remove_take(l, 1)[0] }
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
    let (list_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[5, 6, 7]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let new_list = main.call(&mut store, list_ptr).expect("call main");

    let len = read_i32(&memory, &mut store, new_list + 0);
    let data_ptr = read_i32(&memory, &mut store, new_list + 12);
    let a = read_i32(&memory, &mut store, data_ptr + 0);
    let b = read_i32(&memory, &mut store, data_ptr + 4);

    assert_eq!(len, 2, "remove_take should decrease len");
    assert_eq!(a, 5, "first element preserved");
    assert_eq!(b, 7, "elements should shift left");
}

#[test]
fn list_get_in_bounds_returns_some() {
    let src = r#"
        function main(l: List<Int>) -> Option<Int> { std::list::get(l, 1) }
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
    let opt_ptr = main.call(&mut store, list_ptr).expect("call main");

    let (tag, payload) = read_option_i32(&memory, &mut store, opt_ptr);
    assert_eq!(tag, 1, "expected Some tag");
    assert_eq!(payload, 20, "expected element at index 1");
}

#[test]
fn list_get_oob_returns_none() {
    let src = r#"
        function main(l: List<Int>) -> Option<Int> { std::list::get(l, 5) }
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
    let opt_ptr = main.call(&mut store, list_ptr).expect("call main");

    let (tag, payload) = read_option_i32(&memory, &mut store, opt_ptr);
    assert_eq!(tag, 0, "expected None tag");
    assert_eq!(payload, 0, "None payload should be zero");
}

#[test]
fn list_pop_nonempty_returns_some() {
    let src = r#"
        function main(l: List<Int>) -> Option<Int> { std::list::pop(l) }
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
    let (list_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[3, 4]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let opt_ptr = main.call(&mut store, list_ptr).expect("call main");

    let (tag, payload) = read_option_i32(&memory, &mut store, opt_ptr);
    assert_eq!(tag, 1, "expected Some tag");
    assert_eq!(payload, 4, "expected last element");
}

#[test]
fn list_pop_empty_returns_none() {
    let src = r#"
        function main(l: List<Int>) -> Option<Int> { std::list::pop(l) }
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
    let (list_ptr, new_heap) = alloc_list(&memory, &mut store, heap_ptr, &[]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", new_heap);

    let main = instance
        .get_typed_func::<i32, i32>(&mut store, "main")
        .expect("get main");
    let opt_ptr = main.call(&mut store, list_ptr).expect("call main");

    let (tag, payload) = read_option_i32(&memory, &mut store, opt_ptr);
    assert_eq!(tag, 0, "expected None tag");
    assert_eq!(payload, 0, "None payload should be zero");
}

#[test]
fn list_remove_take_branch_workflow_returns_branch_specific_head() {
    let src = r#"
        function main(flag: Bool, l: List<Int>) -> Int {
            let next = if flag {
                std::list::remove_take(l, 0)[0]
            } else {
                std::list::remove_take(l, 1)[0]
            };
            match std::list::get(next, 0) {
                Some(v) => v,
                None => 0
            }
        }
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
    let main = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "main")
        .expect("get main");

    let heap_ptr_0 = get_global_i32(&instance, &mut store, "__clg_heap_ptr");
    let (list_ptr_true, heap_ptr_1) = alloc_list(&memory, &mut store, heap_ptr_0, &[8, 9, 10]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", heap_ptr_1);
    let out_true = main.call(&mut store, (1, list_ptr_true)).expect("call main true");
    assert_eq!(out_true, 9, "flag=true removes head, next head should be 9");

    let heap_ptr_2 = get_global_i32(&instance, &mut store, "__clg_heap_ptr");
    let (list_ptr_false, heap_ptr_3) =
        alloc_list(&memory, &mut store, heap_ptr_2, &[8, 9, 10]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", heap_ptr_3);
    let out_false = main
        .call(&mut store, (0, list_ptr_false))
        .expect("call main false");
    assert_eq!(out_false, 8, "flag=false removes index 1, head should stay 8");
}
