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
fn u128_u256_limb_helpers_roundtrip() {
    let cases = [
        Case {
            name: "u128_lo",
            src: r#"
                pure function main() -> U64 {
                    std::u128::lo(std::u128::from_limbs(5, 6))
                }
            "#,
            expected: 5,
        },
        Case {
            name: "u128_hi",
            src: r#"
                pure function main() -> U64 {
                    std::u128::hi(std::u128::from_limbs(5, 6))
                }
            "#,
            expected: 6,
        },
        Case {
            name: "u256_limb0",
            src: r#"
                pure function main() -> U64 {
                    std::u256::limb0(std::u256::from_limbs(7, 8, 9, 10))
                }
            "#,
            expected: 7,
        },
        Case {
            name: "u256_limb3",
            src: r#"
                pure function main() -> U64 {
                    std::u256::limb3(std::u256::from_limbs(7, 8, 9, 10))
                }
            "#,
            expected: 10,
        },
    ];

    for case in cases {
        let out = run_wasm_and_get_i64_result(case.src);
        assert_eq!(out, case.expected, "output mismatch for {}", case.name);
    }
}
