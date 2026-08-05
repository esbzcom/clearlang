use clg_parser::parse;
use clg_typer::{check, type_check_only};

#[test]
fn contract_state_fields_must_be_persistable_and_unique() {
    let valid = r#"
        contract Vault version 1 {
            state { owner: Bytes; total: U64; balances: Map<Bytes, U64>; }
        }
        function main() -> Int { 0 }
    "#;
    check(&parse(valid).expect("parse valid contract state")).expect("type-check valid state");

    let duplicate = r#"
        contract Vault version 1 { state { total: U64; total: U64; } }
        function main() -> Int { 0 }
    "#;
    let err = check(&parse(duplicate).expect("parse duplicate state"))
        .expect_err("reject duplicate field");
    assert!(format!("{err:#}").contains("T820"));

    let function_value = r#"
        contract Vault version 1 { state { callback: function(Int) -> Int; } }
        function main() -> Int { 0 }
    "#;
    let err = check(&parse(function_value).expect("parse function state"))
        .expect_err("reject function state");
    assert!(format!("{err:#}").contains("T821"));
}

#[test]
fn contract_owned_pure_function_can_read_declared_state_field() {
    let source = r#"
        contract Counter version 1 {
            state { total: U64; }
            pure function read() -> U64 { state.total }
        }
    "#;
    type_check_only(&parse(source).expect("parse contract state read"))
        .expect("type-check contract state read");

    let err = check(&parse(source).expect("parse contract state read"))
        .expect_err("target lowering must fail closed without a state adapter");
    assert!(format!("{err:#}").contains("stateful contract lowering is unavailable"));
}

#[test]
fn state_is_not_visible_to_non_contract_functions() {
    let source = r#"
        contract Counter version 1 { state { total: U64; } }
        pure function invalid() -> U64 { state.total }
    "#;
    let err = type_check_only(&parse(source).expect("parse ordinary function"))
        .expect_err("state must remain contract-scoped");
    assert!(format!("{err:#}").contains("T006"));
}
