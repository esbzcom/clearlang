use clg_codegen_wasm::emit_from_ir;
use clg_parser::parse;
use clg_typer::check;

#[test]
fn wasm_backend_rejects_target_neutral_contract_state_operations() {
    let source = r#"
        contract Counter version 1 {
            state { total: U64; }
            mut function set_total(value: U64) -> U64 {
                state.total = value;
                state.total
            }
        }
    "#;
    let ir = check(&parse(source).expect("parse stateful contract"))
        .expect("lower target-neutral state operations");
    let err = emit_from_ir(&ir).expect_err("Wasm lacks a contract state adapter");
    assert!(err.to_string().contains("no contract state adapter"));
}
