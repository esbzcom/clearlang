use clg_parser::parse;
use clg_typer::check;

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
