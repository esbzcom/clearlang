use clg_codegen_wasm::emit_from_ir;
use clg_parser::parse;
use clg_typer::{check_with_vcs_with_std, StdTypeInfo, StdTypeMap};

mod common;

fn get_global_i32(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<()>,
    name: &str,
) -> i32 {
    let g = instance
        .get_global(&mut *store, name)
        .expect("global present");
    g.get(&mut *store).i32().expect("i32 global")
}

fn std_types() -> StdTypeMap {
    let mut map = StdTypeMap::new();
    map.insert(
        "std::eth::Address".to_string(),
        StdTypeInfo {
            byte_len: 20,
            align: 1,
        },
    );
    map
}

fn compile(src: &str) -> Vec<u8> {
    let ast = parse(src).expect("parse ok");
    let std_types = std_types();
    let ir = check_with_vcs_with_std(&ast, &std_types)
        .expect("type-check+lower ok")
        .ir;
    emit_from_ir(&ir).expect("codegen ok")
}

#[test]
fn std_eth_struct_field_roundtrip() {
    let src = r#"
        struct Wallet { owner: std::eth::Address; }
        function main() -> Bool {
            let a = std::eth::from_bytes(std::bytes::from_string("abcdefghijklmnopqrst"));
            let w = Wallet { owner: a };
            w.owner == std::eth::from_bytes(std::bytes::from_string("abcdefghijklmnopqrst"))
        }
    "#;
    let wasm = compile(src);

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let ok = main.call(&mut store, ()).expect("call main");
    assert_eq!(ok, 1, "expected std value equality");
}

#[test]
fn std_eth_array_literal_roundtrip() {
    let src = r#"
        function main() -> Bool {
            let a = std::eth::from_array([U8(1), 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
            let b = std::eth::from_array([U8(1), 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
            let arr = [a, b];
            arr[0] == std::eth::from_array([U8(1), 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20])
        }
    "#;
    let wasm = compile(src);

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let ok = main.call(&mut store, ()).expect("call main");
    assert_eq!(ok, 1, "expected array roundtrip equality");
}

#[test]
fn std_eth_list_get_roundtrip() {
    let src = r#"
        function make() -> List<std::eth::Address> { std::list::new() }
        function main() -> Bool {
            let a = std::eth::from_bytes(std::bytes::from_string("abcdefghijklmnopqrst"));
            let l = std::list::push(make(), a);
            match std::list::get(l, 0) {
                Some(x) => x == std::eth::from_bytes(std::bytes::from_string("abcdefghijklmnopqrst")),
                None => false
            }
        }
    "#;
    let wasm = compile(src);

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let ok = main.call(&mut store, ()).expect("call main");
    assert_eq!(ok, 1, "expected list element roundtrip equality");
}

#[test]
fn std_eth_map_contains_key() {
    let src = r#"
        function make() -> Map<std::eth::Address, Int> { std::map::new() }
        function main() -> Bool {
            let a = std::eth::from_bytes(std::bytes::from_string("abcdefghijklmnopqrst"));
            let m = std::map::insert(make(), a, 7);
            std::map::contains(m, std::eth::from_bytes(std::bytes::from_string("abcdefghijklmnopqrst")))
        }
    "#;
    let wasm = compile(src);

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let ok = main.call(&mut store, ()).expect("call main");
    assert_eq!(ok, 1, "expected map key equality");
}

#[test]
fn std_eth_from_bytes_len_mismatch_traps() {
    let src = r#"
        function main() -> Int {
            let a = std::eth::from_bytes(std::bytes::from_string("short"));
            0
        }
    "#;
    let wasm = compile(src);

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, ());
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 1, "expected R000 (ContractViolation) trap code");
}
