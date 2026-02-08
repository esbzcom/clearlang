use clg_codegen_wasm::emit_from_ir;
use clg_parser::parse;
use clg_typer::check;
use std::convert::TryInto;

mod common;

fn read_i32(memory: &wasmtime::Memory, store: &mut wasmtime::Store<()>, addr: i32) -> i32 {
    let mut buf = [0u8; 4];
    memory
        .read(store, addr as usize, &mut buf)
        .expect("read i32");
    i32::from_le_bytes(buf)
}

fn read_i64(memory: &wasmtime::Memory, store: &mut wasmtime::Store<()>, addr: i32) -> i64 {
    let mut buf = [0u8; 8];
    memory
        .read(store, addr as usize, &mut buf)
        .expect("read i64");
    i64::from_le_bytes(buf)
}

#[test]
fn struct_layout_writes_fields() {
    let src = r#"
        struct Point {
            x: Int;
            y: U64;
        }
        function main() -> Point {
            Point { x: 5, y: 9 }
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
    let ptr = main.call(&mut store, ()).expect("call main");

    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("memory export");

    let x = read_i32(&memory, &mut store, ptr);
    let y = read_i64(&memory, &mut store, ptr + 8);

    assert_eq!(x, 5, "x field should be stored at offset 0");
    assert_eq!(y, 9, "y field should be stored at offset 8");
}

#[test]
fn struct_field_access_reads_value() {
    let src = r#"
        struct Point {
            x: Int;
            y: U64;
        }
        function main() -> U64 {
            let p = Point { x: 1, y: 42 };
            p.y
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
        .get_typed_func::<(), i64>(&mut store, "main")
        .expect("get main");
    let result = main.call(&mut store, ()).expect("call main");

    assert_eq!(result, 42, "field access should return stored value");
}

#[test]
fn enum_variant_layout_writes_payload() {
    let src = r#"
        enum Pair {
            Pair(Int, Int)
        }
        function main() -> Pair {
            Pair::Pair(7, 11)
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let heap_before = instance
        .get_global(&mut store, "__clg_heap_ptr")
        .expect("heap ptr global")
        .get(&mut store)
        .i32()
        .expect("i32 heap ptr");

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let variant_ptr = main.call(&mut store, ()).expect("call main");

    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("memory export");

    let mut buf = [0u8; 16];
    memory
        .read(&mut store, variant_ptr as usize, &mut buf)
        .expect("read variant bytes");

    let tag = i32::from_le_bytes(buf[0..4].try_into().unwrap());
    let payload_lo = i32::from_le_bytes(buf[4..8].try_into().unwrap());
    let payload_hi = i32::from_le_bytes(buf[8..12].try_into().unwrap());
    let reserved = i32::from_le_bytes(buf[12..16].try_into().unwrap());

    assert_eq!(tag, 0, "first enum variant tag should be 0");
    let expected_payload = (heap_before + 3) & !3;
    assert_eq!(
        payload_lo, expected_payload,
        "tuple payload should be allocated at the aligned heap pointer"
    );
    assert_eq!(payload_hi, 0, "payload_hi should be zeroed");
    assert_eq!(reserved, 0, "reserved slot must be zeroed");

    let first = read_i32(&memory, &mut store, payload_lo);
    let second = read_i32(&memory, &mut store, payload_lo + 4);
    assert_eq!(first, 7, "first tuple field should be stored at offset 0");
    assert_eq!(
        second, 11,
        "second tuple field should be stored at offset 4"
    );
}

#[test]
fn enum_match_executes_correct_arm() {
    let src = r#"
        enum Color {
            Red,
            Green
        }
        function main() -> Int {
            let c = Color::Green();
            match c {
                Color::Red => 1,
                Color::Green => 2
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

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let result = main.call(&mut store, ()).expect("call main");

    assert_eq!(result, 2, "match should select the Green arm");
}

#[test]
fn enum_match_binds_payload_fields() {
    let src = r#"
        enum Pair {
            Pair(Int, Int),
            Zero
        }
        function main() -> Int {
            let p = Pair::Pair(3, 4);
            match p {
                Pair::Pair(a, b) => a + b,
                Pair::Zero => 0
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

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let result = main.call(&mut store, ()).expect("call main");

    assert_eq!(result, 7, "enum match should bind and sum payload fields");
}
