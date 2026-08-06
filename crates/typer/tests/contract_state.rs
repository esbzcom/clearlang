use clg_ir::Instr;
use clg_parser::parse;
use clg_typer::{check, generate_vcs, type_check_only};

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

    let module = check(&parse(source).expect("parse contract state read"))
        .expect("lower state read into target-neutral IR");
    assert!(matches!(module.funcs[0].body[0], Instr::StateRead { .. }));
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

#[test]
fn mut_contract_transition_can_write_declared_state_field() {
    let source = r#"
        contract Counter version 1 {
            state { total: U64; }
            mut function set_total(value: U64) -> U64 {
                state.total = value;
                state.total
            }
        }
    "#;
    type_check_only(&parse(source).expect("parse state write"))
        .expect("type-check mut state write");
    let module = check(&parse(source).expect("parse state write"))
        .expect("lower state write into target-neutral IR");
    assert!(matches!(module.funcs[0].body[0], Instr::StateWrite { .. }));

    let err = type_check_only(
        &parse(
            r#"
                contract Counter version 1 {
                    state { total: U64; }
                    pure function invalid(value: U64) -> U64 {
                        state.total = value;
                        state.total
                    }
                }
            "#,
        )
        .expect("parse pure state write"),
    )
    .expect_err("reject state write outside mut transition");
    assert!(format!("{err:#}").contains("T822"));
}

#[test]
fn state_transition_vc_has_deterministic_pre_post_symbols_and_frames() {
    let source = r#"
        contract Counter version 1 {
            state { total: U64; revision: U64; }
            mut function set_total(value: U64) -> U64 {
                state.total = value;
                state.total
            }
        }
    "#;
    let program = parse(source).expect("parse state transition");
    let vcs = generate_vcs(&program);
    let vc = vcs
        .iter()
        .find(|vc| vc.function == "set_total" && vc.vc_id == "state-transition:0")
        .expect("state transition VC");
    assert!(vc.pre.ast.contains("clg.state.pre.sha256:"));
    assert!(vc.post.ast.contains("writes [total]"));
    assert!(vc.vc_smt2.contains("clg.state.post.sha256:"));
    assert!(vc.vc_smt2.contains("(= |clg.state.post."));
    assert!(vc
        .assumptions
        .iter()
        .any(|assumption| assumption.id == "contract.state.transition.adapter"));
}

#[test]
fn old_snapshot_is_limited_to_state_rooted_ensure_expressions() {
    let valid = r#"
        contract Counter version 1 {
            state { total: U64; }
            pure function read() -> U64
                ensure { result == old(state.total) }
            { state.total }
        }
    "#;
    type_check_only(&parse(valid).expect("parse old state snapshot"))
        .expect("type-check state-rooted old snapshot");

    let invalid = r#"
        contract Counter version 1 {
            state { total: U64; }
            pure function invalid(value: U64) -> U64
                ensure { result == old(value) }
            { value }
        }
    "#;
    let err = type_check_only(&parse(invalid).expect("parse invalid old snapshot"))
        .expect_err("reject non-state old expression");
    assert!(format!("{err:#}").contains("T823"));
}
