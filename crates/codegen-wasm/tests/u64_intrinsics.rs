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
fn u64_wrap_and_sat_intrinsics() {
    let cases = [
        Case {
            name: "sub_wrap_to_max",
            src: r#"
                pure function main() -> U64 { std::u64::sub_wrap(0, 1) }
            "#,
            expected: -1,
        },
        Case {
            name: "add_wrap_to_zero",
            src: r#"
                pure function main() -> U64 {
                    std::u64::add_wrap(std::u64::sub_wrap(0, 1), 1)
                }
            "#,
            expected: 0,
        },
        Case {
            name: "add_sat_to_max",
            src: r#"
                pure function main() -> U64 {
                    std::u64::add_sat(std::u64::sub_wrap(0, 1), 1)
                }
            "#,
            expected: -1,
        },
        Case {
            name: "sub_sat_to_zero",
            src: r#"
                pure function main() -> U64 { std::u64::sub_sat(0, 1) }
            "#,
            expected: 0,
        },
        Case {
            name: "mul_sat_to_max",
            src: r#"
                pure function main() -> U64 {
                    std::u64::mul_sat(std::u64::sub_wrap(0, 1), 2)
                }
            "#,
            expected: -1,
        },
        Case {
            name: "add_wrapping_alias",
            src: r#"
                pure function main() -> U64 {
                    std::u64::add_wrapping(std::u64::sub_wrapping(0, 1), 1)
                }
            "#,
            expected: 0,
        },
        Case {
            name: "mul_saturating_alias",
            src: r#"
                pure function main() -> U64 {
                    std::u64::mul_saturating(std::u64::sub_wrap(0, 1), 2)
                }
            "#,
            expected: -1,
        },
    ];

    for case in cases {
        let out = run_wasm_and_get_i64_result(case.src);
        assert_eq!(out, case.expected, "output mismatch for {}", case.name);
    }
}
