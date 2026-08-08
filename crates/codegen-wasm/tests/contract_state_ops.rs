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

#[test]
fn wasm_backend_rejects_target_neutral_contract_event_emission() {
    let source = r#"
        contract Vault version 1 {
            state { total: U64; }
            event Deposit { amount: U64; }
            mut function deposit(amount: U64) -> U64 {
                emit Deposit(amount);
                amount
            }
        }
    "#;
    let ir = check(&parse(source).expect("parse eventful contract"))
        .expect("lower target-neutral event operation");
    let err = emit_from_ir(&ir).expect_err("Wasm lacks a contract event adapter");
    assert!(err.to_string().contains("no contract event adapter"));
}

#[test]
fn wasm_backend_rejects_target_neutral_external_calls() {
    let source = r#"
        interface Receiver { io function receive(amount: U64) -> Bool; }
        contract Vault version 1 {
            state { total: U64; }
            mut function notify(amount: U64) -> U64 {
                external_call Receiver.receive(amount);
                amount
            }
        }
    "#;
    let ir = check(&parse(source).expect("parse external call"))
        .expect("lower target-neutral external call");
    let err = emit_from_ir(&ir).expect_err("Wasm lacks an external-call adapter");
    assert!(err.to_string().contains("no external call adapter"));
}
