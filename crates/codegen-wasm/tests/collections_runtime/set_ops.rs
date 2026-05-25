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

#[test]
fn set_subset_true_when_all_elements_present() {
    let src = r#"
        function main(a: Set<Int>, b: Set<Int>) -> Bool { std::set::subset(a, b) }
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
    let (a_ptr, heap_after_a) = alloc_list(&memory, &mut store, heap_ptr, &[1, 2]);
    let (b_ptr, heap_after_b) = alloc_list(&memory, &mut store, heap_after_a, &[1, 2, 3]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", heap_after_b);

    let main = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "main")
        .expect("get main");
    let ok = main.call(&mut store, (a_ptr, b_ptr)).expect("call main");
    assert_eq!(ok, 1, "expected subset to return true");
}

#[test]
fn set_subset_false_when_element_missing() {
    let src = r#"
        function main(a: Set<Int>, b: Set<Int>) -> Bool { std::set::subset(a, b) }
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
    let (a_ptr, heap_after_a) = alloc_list(&memory, &mut store, heap_ptr, &[1, 4]);
    let (b_ptr, heap_after_b) = alloc_list(&memory, &mut store, heap_after_a, &[1, 2, 3]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", heap_after_b);

    let main = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "main")
        .expect("get main");
    let ok = main.call(&mut store, (a_ptr, b_ptr)).expect("call main");
    assert_eq!(ok, 0, "expected subset to return false");
}

#[test]
fn set_union_merges_without_duplicates() {
    let src = r#"
        function main(a: Set<Int>, b: Set<Int>) -> Set<Int> { std::set::union(a, b) }
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
    let (a_ptr, heap_after_a) = alloc_list(&memory, &mut store, heap_ptr, &[1, 2]);
    let (b_ptr, heap_after_b) = alloc_list(&memory, &mut store, heap_after_a, &[2, 3]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", heap_after_b);

    let main = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "main")
        .expect("get main");
    let out_ptr = main.call(&mut store, (a_ptr, b_ptr)).expect("call main");

    let values = read_set_values(&memory, &mut store, out_ptr);
    assert_eq!(values, vec![1, 2, 3]);
}

#[test]
fn set_intersect_keeps_shared_elements() {
    let src = r#"
        function main(a: Set<Int>, b: Set<Int>) -> Set<Int> { std::set::intersect(a, b) }
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
    let (a_ptr, heap_after_a) = alloc_list(&memory, &mut store, heap_ptr, &[1, 2, 4]);
    let (b_ptr, heap_after_b) = alloc_list(&memory, &mut store, heap_after_a, &[2, 3, 4]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", heap_after_b);

    let main = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "main")
        .expect("get main");
    let out_ptr = main.call(&mut store, (a_ptr, b_ptr)).expect("call main");

    let values = read_set_values(&memory, &mut store, out_ptr);
    assert_eq!(values, vec![2, 4]);
}

#[test]
fn set_diff_removes_rhs_elements() {
    let src = r#"
        function main(a: Set<Int>, b: Set<Int>) -> Set<Int> { std::set::diff(a, b) }
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
    let (a_ptr, heap_after_a) = alloc_list(&memory, &mut store, heap_ptr, &[1, 2, 3]);
    let (b_ptr, heap_after_b) = alloc_list(&memory, &mut store, heap_after_a, &[2, 9]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", heap_after_b);

    let main = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "main")
        .expect("get main");
    let out_ptr = main.call(&mut store, (a_ptr, b_ptr)).expect("call main");

    let values = read_set_values(&memory, &mut store, out_ptr);
    assert_eq!(values, vec![1, 3]);
}

#[test]
fn set_intersect_dedupes_noncanonical_lhs_payload() {
    let src = r#"
        function main(a: Set<Int>, b: Set<Int>) -> Set<Int> { std::set::intersect(a, b) }
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
    let (a_ptr, heap_after_a) = alloc_list(&memory, &mut store, heap_ptr, &[2, 2, 4]);
    let (b_ptr, heap_after_b) = alloc_list(&memory, &mut store, heap_after_a, &[2, 5]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", heap_after_b);

    let main = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "main")
        .expect("get main");
    let out_ptr = main.call(&mut store, (a_ptr, b_ptr)).expect("call main");

    let values = read_set_values(&memory, &mut store, out_ptr);
    assert_eq!(values, vec![2], "intersect output should stay canonical");
}

#[test]
fn set_diff_dedupes_noncanonical_lhs_payload() {
    let src = r#"
        function main(a: Set<Int>, b: Set<Int>) -> Set<Int> { std::set::diff(a, b) }
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
    let (a_ptr, heap_after_a) = alloc_list(&memory, &mut store, heap_ptr, &[1, 1, 3, 3]);
    let (b_ptr, heap_after_b) = alloc_list(&memory, &mut store, heap_after_a, &[2]);
    set_global_i32(&instance, &mut store, "__clg_heap_ptr", heap_after_b);

    let main = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "main")
        .expect("get main");
    let out_ptr = main.call(&mut store, (a_ptr, b_ptr)).expect("call main");

    let values = read_set_values(&memory, &mut store, out_ptr);
    assert_eq!(values, vec![1, 3], "diff output should stay canonical");
}

fn read_set_values(
    memory: &wasmtime::Memory,
    store: &mut wasmtime::Store<()>,
    set_ptr: i32,
) -> Vec<i32> {
    let len = read_i32(memory, store, set_ptr);
    let data_ptr = read_i32(memory, store, set_ptr + 12);
    let mut out = Vec::with_capacity(len as usize);
    for i in 0..len {
        out.push(read_i32(memory, store, data_ptr + (i * 4)));
    }
    out.sort_unstable();
    out
}
