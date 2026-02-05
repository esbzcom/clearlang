use clg_codegen_wasm::emit_from_ir;
use clg_parser::parse;
use clg_typer::check;
use wasmtime::Val;

mod common;

fn align_up(value: i32, align: i32) -> i32 {
    if align <= 1 {
        return value;
    }
    let adj = align - 1;
    (value + adj) / align * align
}

fn write_i32(
    memory: &wasmtime::Memory,
    store: &mut wasmtime::Store<()>,
    addr: i32,
    value: i32,
) {
    memory
        .write(store, addr as usize, &value.to_le_bytes())
        .expect("write i32");
}

fn read_i32(
    memory: &wasmtime::Memory,
    store: &mut wasmtime::Store<()>,
    addr: i32,
) -> i32 {
    let mut buf = [0u8; 4];
    memory
        .read(store, addr as usize, &mut buf)
        .expect("read i32");
    i32::from_le_bytes(buf)
}

fn read_option_i32(
    memory: &wasmtime::Memory,
    store: &mut wasmtime::Store<()>,
    ptr: i32,
) -> (i32, i32) {
    let tag = read_i32(memory, store, ptr);
    let payload = read_i32(memory, store, ptr + 4);
    (tag, payload)
}

fn get_global_i32(instance: &wasmtime::Instance, store: &mut wasmtime::Store<()>, name: &str) -> i32 {
    let g = instance
        .get_global(&mut *store, name)
        .expect("global present");
    g.get(&mut *store).i32().expect("i32 global")
}

fn set_global_i32(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<()>,
    name: &str,
    value: i32,
) {
    let g = instance
        .get_global(&mut *store, name)
        .expect("global present");
    g.set(&mut *store, Val::I32(value)).expect("set global");
}

fn alloc_list(
    memory: &wasmtime::Memory,
    store: &mut wasmtime::Store<()>,
    heap_ptr: i32,
    elems: &[i32],
) -> (i32, i32) {
    let list_ptr = align_up(heap_ptr, 4);
    let len = elems.len() as i32;
    let cap = if len > 0 { len } else { 1 };
    let data_ptr = list_ptr + 16;

    write_i32(memory, store, list_ptr + 0, len);
    write_i32(memory, store, list_ptr + 4, cap);
    write_i32(memory, store, list_ptr + 8, 0);
    write_i32(memory, store, list_ptr + 12, data_ptr);

    for (idx, value) in elems.iter().enumerate() {
        let offset = data_ptr + (idx as i32) * 4;
        write_i32(memory, store, offset, *value);
    }

    let end = data_ptr + cap * 4;
    let new_heap = align_up(end, 4);
    (list_ptr, new_heap)
}

fn alloc_map(
    memory: &wasmtime::Memory,
    store: &mut wasmtime::Store<()>,
    heap_ptr: i32,
    entries: &[(i32, i32)],
) -> (i32, i32) {
    let map_ptr = align_up(heap_ptr, 4);
    let len = entries.len() as i32;
    let cap = if len > 0 { len } else { 1 };
    let entry_size = 8;
    let data_ptr = map_ptr + 16;

    write_i32(memory, store, map_ptr + 0, len);
    write_i32(memory, store, map_ptr + 4, cap);
    write_i32(memory, store, map_ptr + 8, 0);
    write_i32(memory, store, map_ptr + 12, data_ptr);

    for (idx, (key, val)) in entries.iter().enumerate() {
        let base = data_ptr + (idx as i32) * entry_size;
        write_i32(memory, store, base, *key);
        write_i32(memory, store, base + 4, *val);
    }

    let end = data_ptr + cap * entry_size;
    let new_heap = align_up(end, 4);
    (map_ptr, new_heap)
}

fn assert_pair_unordered(a: i32, b: i32, x: i32, y: i32) {
    assert!(
        (a == x && b == y) || (a == y && b == x),
        "expected pair {{ {}, {} }} got {{ {}, {} }}",
        x,
        y,
        a,
        b
    );
}

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
