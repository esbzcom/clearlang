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

    let duplicate_event = r#"
        contract Vault version 1 {
            state { total: U64; }
            event Deposit { amount: U64; }
            event Deposit { amount: U64; }
        }
    "#;
    let err = type_check_only(&parse(duplicate_event).expect("parse duplicate event"))
        .expect_err("reject duplicate event");
    assert!(format!("{err:#}").contains("T824"));

    let invalid_event_field = r#"
        contract Vault version 1 {
            state { total: U64; }
            event Callback { handler: function(Int) -> Int; }
        }
    "#;
    let err = type_check_only(&parse(invalid_event_field).expect("parse invalid event field"))
        .expect_err("reject non-serializable event field");
    assert!(format!("{err:#}").contains("T826"));
}

#[test]
fn contract_init_requires_direct_initialization_of_every_state_field() {
    let valid = r#"
        contract Vault version 1 {
            state { owner: Bytes; total: U64; }
            init(owner: Bytes, initial: U64) {
                state.owner = owner;
                state.total = initial;
                0
            }
        }
    "#;
    type_check_only(&parse(valid).expect("parse contract init")).expect("type-check complete init");
    let with_invariant = valid.replace(
        "init(owner: Bytes, initial: U64)",
        "invariant { state.total >= U64(0) }\n            init(owner: Bytes, initial: U64)",
    );
    let init_vc = generate_vcs(&parse(&with_invariant).expect("parse init invariant"))
        .into_iter()
        .find(|vc| vc.function == "Vault::init" && vc.vc_id == "init-invariant:0")
        .expect("init invariant VC");
    assert!(init_vc.assumptions.is_empty());
    assert!(init_vc.vc_smt2.contains("clg.state.post.sha256:"));

    let incomplete = valid.replace("                state.total = initial;\n", "");
    let err = type_check_only(&parse(&incomplete).expect("parse incomplete init"))
        .expect_err("reject incomplete init");
    assert!(format!("{err:#}").contains("T829"));
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
fn mut_contract_transition_can_emit_declared_event() {
    let source = r#"
        contract Vault version 1 {
            state { total: U64; }
            event Deposit { account: Bytes; amount: U64; }
            mut function deposit(account: Bytes, amount: U64) -> U64 {
                emit Deposit(account, amount);
                amount
            }
        }
    "#;
    type_check_only(&parse(source).expect("parse event emission"))
        .expect("type-check event emission");
    let module = check(&parse(source).expect("parse event emission"))
        .expect("lower target-neutral event emission");
    assert!(module.funcs[0]
        .body
        .iter()
        .any(|instruction| matches!(instruction, Instr::EventEmit { .. })));

    let invalid = r#"
        contract Vault version 1 {
            state { total: U64; }
            event Deposit { amount: U64; }
            pure function invalid(amount: U64) -> U64 {
                emit Deposit(amount);
                amount
            }
        }
    "#;
    let err = type_check_only(&parse(invalid).expect("parse pure event emission"))
        .expect_err("reject event emission outside mut transition");
    assert!(format!("{err:#}").contains("T827"));
}

#[test]
fn contract_migration_has_exclusive_state_write_access() {
    let source = r#"
        contract Vault version 2 {
            state { total: U64; }
            migrate from schema "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" {
                state.total = U64(0);
                0
            }
        }
    "#;
    type_check_only(&parse(source).expect("parse migration"))
        .expect("type-check migration state write");
    let migration_vc = generate_vcs(&parse(source).expect("parse migration"))
        .into_iter()
        .find(|vc| vc.function == "Vault::migrate" && vc.vc_id == "migration-invariant:0");
    assert!(
        migration_vc.is_none(),
        "migration without invariant has no invariant VC"
    );

    let with_invariant = source.replace(
        "migrate from schema",
        "invariant { state.total >= U64(0) }\n            migrate from schema",
    );
    let migration_vc = generate_vcs(&parse(&with_invariant).expect("parse migration invariant"))
        .into_iter()
        .find(|vc| vc.function == "Vault::migrate" && vc.vc_id == "migration-invariant:0")
        .expect("migration invariant VC");
    assert!(migration_vc.vc_smt2.contains("clg.state.post.sha256:"));
    assert!(migration_vc.assumptions.is_empty());

    let invalid_digest = source.replace(
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "wrong",
    );
    let err = type_check_only(&parse(&invalid_digest).expect("parse migration digest"))
        .expect_err("reject invalid migration digest");
    assert!(format!("{err:#}").contains("T828"));

    let eventful = r#"
        contract Vault version 2 {
            state { total: U64; }
            event Migrated { total: U64; }
            migrate from schema "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" {
                emit Migrated(U64(0));
                0
            }
        }
    "#;
    let err = type_check_only(&parse(eventful).expect("parse eventful migration"))
        .expect_err("reject event emission from migration");
    assert!(format!("{err:#}").contains("T827"));
}

#[test]
fn mut_contract_transition_can_make_typed_external_call() {
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
    type_check_only(&parse(source).expect("parse external call"))
        .expect("type-check explicit external call");
    let module = check(&parse(source).expect("parse external call"))
        .expect("lower target-neutral external call");
    assert!(module.funcs[0]
        .body
        .iter()
        .any(|instruction| matches!(instruction, Instr::ExternalCall { .. })));

    let invalid = source.replace("mut function", "pure function");
    let err = type_check_only(&parse(&invalid).expect("parse pure external call"))
        .expect_err("reject external call outside mut transition");
    assert!(format!("{err:#}").contains("T830"));
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
    assert!(vc.assumptions.is_empty());
}

#[test]
fn contract_state_invariants_are_checked_and_emit_transition_obligations() {
    let source = r#"
        contract Counter version 1 {
            state { open: Bool; }
            invariant { state.open == true }
            mut function activate() -> Bool {
                state.open = true;
                state.open
            }
        }
    "#;
    let program = parse(source).expect("parse state invariant");
    type_check_only(&program).expect("type-check state invariant");
    let vc = generate_vcs(&program)
        .into_iter()
        .find(|vc| vc.function == "activate" && vc.vc_id == "state-invariant:0")
        .expect("state invariant VC");
    assert!(vc.pre.smt2.contains("|clg.state.pre.sha256:"));
    assert!(vc.post.smt2.contains("|clg.state.post.sha256:"));
    assert!(vc.vc_smt2.contains("(=> (and (= |clg.state.pre."));
    assert!(vc.assumptions.is_empty());

    let invalid = r#"
        contract Counter version 1 {
            state { total: U64; }
            invariant { state.total }
        }
    "#;
    let err = type_check_only(&parse(invalid).expect("parse non-bool invariant"))
        .expect_err("reject non-bool invariant");
    assert!(format!("{err:#}").contains("T014"));
}

#[test]
fn scalar_state_model_relates_writes_to_exit_invariants_and_keeps_unsupported_flow_blocked() {
    let valid = r#"
        contract Counter version 1 {
            state { open: Bool; }
            invariant { state.open == true }
            mut function activate() -> Bool {
                state.open = true;
                state.open
            }
        }
    "#;
    let valid_vc = generate_vcs(&parse(valid).expect("parse valid transition"))
        .into_iter()
        .find(|vc| vc.vc_id == "state-invariant:0")
        .expect("valid invariant VC");
    assert!(valid_vc.vc_smt2.contains("(= |clg.state.post."));
    assert!(valid_vc.vc_smt2.contains(" true)"));
    assert!(valid_vc.assumptions.is_empty());

    let invalid = valid.replace("state.open = true", "state.open = false");
    let invalid_vc = generate_vcs(&parse(&invalid).expect("parse invalid transition"))
        .into_iter()
        .find(|vc| vc.vc_id == "state-invariant:0")
        .expect("invalid invariant VC");
    assert!(invalid_vc.vc_smt2.contains("(= |clg.state.post."));
    assert!(invalid_vc.vc_smt2.contains(" false)"));
    assert!(invalid_vc.assumptions.is_empty());

    let unsupported = valid.replace(
        "state { open: Bool; }",
        "state { open: Bool; balances: Map<Bytes, U64>; }",
    );
    let unsupported_vc = generate_vcs(&parse(&unsupported).expect("parse unsupported flow"))
        .into_iter()
        .find(|vc| vc.vc_id == "state-invariant:0")
        .expect("unsupported invariant VC");
    assert!(unsupported_vc
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
    let snapshot_vcs = generate_vcs(&parse(valid).expect("parse old state snapshot"));
    let snapshot_vc = snapshot_vcs
        .iter()
        .find(|vc| vc.function == "read" && vc.vc_id == "vc:0")
        .expect("snapshot ensure VC");
    assert!(snapshot_vc.post.smt2.contains("|clg.state.pre.sha256:"));
    assert!(snapshot_vc.vc_smt2.contains("|clg.state.post.sha256:"));
    assert!(snapshot_vc
        .vc_smt2
        .contains("(declare-const |clg.state.pre.sha256:"));
    assert!(snapshot_vc.assumptions.is_empty());

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
