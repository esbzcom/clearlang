use clg_codegen_wasm::emit_from_ir;
use clg_parser::parse;
use clg_typer::check;
mod common;

fn run_wasm_and_get_i64_result(src: &str) -> i64 {
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");
    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module from bytes");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .expect("get main");
    main.call(&mut store, ()).expect("invoke main")
}

struct Case {
    name: &'static str,
    src: &'static str,
    expected: i64,
}

#[test]
fn u64_bitwise_shift_rotate_and_bytes() {
    let cases = [
        Case {
            name: "bitwise_and_or",
            src: r#"
                pure function main() -> U64 { (U64(240) & 15) | 3 }
            "#,
            expected: 3,
        },
        Case {
            name: "bitwise_xor",
            src: r#"
                pure function main() -> U64 { U64(5) ^ 6 }
            "#,
            expected: 3,
        },
        Case {
            name: "shift_left",
            src: r#"
                pure function main() -> U64 { U64(1) << 8 }
            "#,
            expected: 256,
        },
        Case {
            name: "rotate_left",
            src: r#"
                pure function main() -> U64 { std::u64::rotl(U64(1), 1) }
            "#,
            expected: 2,
        },
        Case {
            name: "rotate_right",
            src: r#"
                pure function main() -> U64 { std::u64::rotr(U64(1), 1) }
            "#,
            expected: i64::MIN,
        },
        Case {
            name: "bytes_roundtrip_le",
            src: r#"
                pure function main() -> U64 {
                    std::u64::from_bytes_le(std::u64::to_bytes_le(42))
                }
            "#,
            expected: 42,
        },
        Case {
            name: "bytes_roundtrip_be",
            src: r#"
                pure function main() -> U64 {
                    std::u64::from_bytes_be(std::u64::to_bytes_be(42))
                }
            "#,
            expected: 42,
        },
    ];

    for case in cases {
        let out = run_wasm_and_get_i64_result(case.src);
        assert_eq!(out, case.expected, "output mismatch for {}", case.name);
    }
}
