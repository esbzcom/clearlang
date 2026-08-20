#[test]
fn build_emits_deterministic_contract_state_schema() {
    let dir = tempdir().expect("tempdir");
    let support_dir = dir.path().join("support");
    fs::create_dir_all(&support_dir).expect("create support directory");
    let support = support_dir.join("values.clear");
    fs::write(&support, "export function one() -> U64 { U64(1) }").expect("write support source");
    let source = dir.path().join("vault.clear");
    fs::write(
        &source,
        r#"
            import support::values

            contract Vault version 1 {
                state { owner: Bytes; total: U64; balances: Map<Bytes, U64>; }
                event Deposit { account: Bytes; amount: U64; }
            }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write source");
    let first_schema = dir.path().join("first.schema.json");
    let second_schema = dir.path().join("second.schema.json");
    let changed_source_schema = dir.path().join("changed-source.schema.json");

    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "build",
            source.to_str().expect("source path"),
            "-o",
            dir.path().join("vault.wasm").to_str().expect("wasm path"),
            "--emit-contract-state-schema",
            first_schema.to_str().expect("schema path"),
        ])
        .assert()
        .success();
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "build",
            source.to_str().expect("source path"),
            "-o",
            dir.path().join("vault-second.wasm").to_str().expect("wasm path"),
            "--emit-contract-state-schema",
            second_schema.to_str().expect("schema path"),
        ])
        .assert()
        .success();

    let first = fs::read(&first_schema).expect("read first schema");
    let second = fs::read(&second_schema).expect("read second schema");
    assert_eq!(first, second, "schema output must be byte-stable");
    let schema: Value = serde_json::from_slice(&first).expect("parse schema");
    assert_eq!(schema["schema_version"], 1);
    assert_eq!(schema["contract"]["name"], "Vault");
    assert_eq!(schema["contract"]["version"], 1);
    assert_eq!(schema["fields"][0]["name"], "owner");
    assert_eq!(schema["fields"][1]["name"], "total");
    assert_eq!(schema["fields"][2]["name"], "balances");
    assert_eq!(schema["layout"]["field_order"], "declaration");
    assert_eq!(
        schema["layout"]["target_profile"],
        "clg.contract-state-solver.v1"
    );
    assert_eq!(schema["invariants"].as_array().map(Vec::len), Some(0));
    assert!(schema["init"].is_null());
    assert_eq!(schema["compiler"]["name"], "clg-cli");
    assert!(schema["source_graph"]["digest"]
        .as_str()
        .expect("source digest")
        .starts_with("sha256:"));
    assert_eq!(schema["source_graph"]["files"].as_array().map(Vec::len), Some(2));
    assert_eq!(schema["source_graph"]["files"][0]["path"], "support/values.clear");
    assert_eq!(schema["source_graph"]["files"][1]["path"], "vault.clear");
    assert_eq!(schema["events"][0]["name"], "Deposit");
    assert_eq!(schema["events"][0]["fields"][0]["name"], "account");
    assert_eq!(schema["events"][0]["fields"][1]["name"], "amount");
    assert_eq!(schema["event_abi"]["algorithm"], "clg.contract-event-abi.v1");
    assert!(schema["event_abi"]["digest"]
        .as_str()
        .expect("event ABI digest")
        .starts_with("sha256:"));

    fs::write(&support, "export function one() -> U64 { U64(2) }").expect("change support source");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "build",
            source.to_str().expect("source path"),
            "-o",
            dir.path().join("vault-changed.wasm").to_str().expect("wasm path"),
            "--emit-contract-state-schema",
            changed_source_schema.to_str().expect("schema path"),
        ])
        .assert()
        .success();
    let changed: Value = serde_json::from_slice(
        &fs::read(&changed_source_schema).expect("read changed-source schema"),
    )
    .expect("parse changed-source schema");
    assert_ne!(
        schema["source_graph"]["digest"],
        changed["source_graph"]["digest"],
        "all loaded source files must contribute to the contract identity"
    );
}

#[test]
fn build_checks_contract_state_schema_is_append_only() {
    let dir = tempdir().expect("tempdir");
    let prior_source = dir.path().join("vault-v1.clear");
    let compatible_source = dir.path().join("vault-v2.clear");
    let incompatible_source = dir.path().join("vault-invalid.clear");
    let migrated_source = dir.path().join("vault-migrated.clear");
    let prior_schema = dir.path().join("vault-v1.schema.json");
    fs::write(
        &prior_source,
        r#"
            contract Vault version 1 { state { owner: Bytes; total: U64; } }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write prior source");
    fs::write(
        &compatible_source,
        r#"
            contract Vault version 2 { state { owner: Bytes; total: U64; revision: U64; } }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write compatible source");
    fs::write(
        &incompatible_source,
        r#"
            contract Vault version 2 { state { total: U64; owner: Bytes; } }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write incompatible source");

    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "build",
            prior_source.to_str().expect("prior source path"),
            "-o",
            dir.path().join("prior.wasm").to_str().expect("prior wasm path"),
            "--emit-contract-state-schema",
            prior_schema.to_str().expect("prior schema path"),
        ])
        .assert()
        .success();
    let prior: Value = serde_json::from_slice(&fs::read(&prior_schema).expect("read prior schema"))
        .expect("parse prior schema");
    let prior_digest = prior["schema"]["digest"]
        .as_str()
        .expect("prior schema digest");
    fs::write(
        &migrated_source,
        format!(
            r#"
                contract Vault version 2 {{
                    state {{ total: U64; owner: Bytes; }}
                    migrate from schema "{prior_digest}" {{
                        state.total = U64(0);
                        0
                    }}
                }}
                function main() -> Int {{ 0 }}
            "#
        ),
    )
    .expect("write migrated source");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "build",
            compatible_source.to_str().expect("compatible source path"),
            "-o",
            dir.path()
                .join("compatible.wasm")
                .to_str()
                .expect("compatible wasm path"),
            "--check-contract-state-schema",
            prior_schema.to_str().expect("prior schema path"),
        ])
        .assert()
        .success();
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "build",
            incompatible_source
                .to_str()
                .expect("incompatible source path"),
            "-o",
            dir.path()
                .join("incompatible.wasm")
                .to_str()
                .expect("incompatible wasm path"),
            "--check-contract-state-schema",
            prior_schema.to_str().expect("prior schema path"),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("declare a migration"));
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "build",
            migrated_source.to_str().expect("migrated source path"),
            "-o",
            dir.path()
                .join("migrated.wasm")
                .to_str()
                .expect("migrated wasm path"),
            "--check-contract-state-schema",
            prior_schema.to_str().expect("prior schema path"),
        ])
        .assert()
        .success();
}

#[test]
fn build_emits_deterministic_target_contract_abi() {
    let dir = tempdir().expect("tempdir");
    let support_dir = dir.path().join("support");
    fs::create_dir_all(&support_dir).expect("create support directory");
    let support = support_dir.join("values.clear");
    fs::write(&support, "export function one() -> U64 { U64(1) }").expect("write support source");
    let source = dir.path().join("vault.clear");
    fs::write(
        &source,
        r#"
            import support::values

            contract Vault version 1 {
                state { total: U64; }
                event Deposited { amount: U64; }
                pure function balance() -> U64 { U64(0) }
                mut function deposit(amount: U64) -> U64 { amount }
            }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write source");
    let first_abi = dir.path().join("first.abi.json");
    let second_abi = dir.path().join("second.abi.json");
    let changed_source_abi = dir.path().join("changed-source.abi.json");

    for (abi, wasm) in [
        (&first_abi, dir.path().join("first.wasm")),
        (&second_abi, dir.path().join("second.wasm")),
    ] {
        Command::cargo_bin("clg")
            .expect("clg binary")
            .args([
                "build",
                source.to_str().expect("source path"),
                "-o",
                wasm.to_str().expect("wasm path"),
                "--emit-contract-abi",
                abi.to_str().expect("ABI path"),
            ])
            .assert()
            .success();
    }

    let first = fs::read(&first_abi).expect("read first ABI");
    let second = fs::read(&second_abi).expect("read second ABI");
    assert_eq!(first, second, "ABI output must be byte-stable");
    let abi: Value = serde_json::from_slice(&first).expect("parse ABI");
    assert_eq!(abi["abi"]["format"], "clg.contract-abi.v1");
    assert_eq!(abi["contract"]["name"], "Vault");
    assert_eq!(abi["functions"][0]["signature"], "balance()->U64");
    assert_eq!(abi["functions"][1]["signature"], "deposit(U64)->U64");
    assert_eq!(abi["events"][0]["name"], "Deposited");
    assert_eq!(abi["errors"]["status"], "unsupported");
    assert_eq!(abi["target"]["profile"], "clg.evm-compatible.abi.v1");
    assert_eq!(abi["target"]["wire_encoding"], "deferred");
    assert!(abi["state_schema"]["digest"]
        .as_str()
        .expect("state schema digest")
        .starts_with("sha256:"));
    assert!(abi["source_graph"]["digest"]
        .as_str()
        .expect("source graph digest")
        .starts_with("sha256:"));

    fs::write(&support, "export function one() -> U64 { U64(2) }").expect("change support source");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "build",
            source.to_str().expect("source path"),
            "-o",
            dir.path()
                .join("changed-source.wasm")
                .to_str()
                .expect("wasm path"),
            "--emit-contract-abi",
            changed_source_abi.to_str().expect("ABI path"),
        ])
        .assert()
        .success();
    let changed: Value = serde_json::from_slice(
        &fs::read(&changed_source_abi).expect("read changed-source ABI"),
    )
    .expect("parse changed-source ABI");
    assert_ne!(
        abi["source_graph"]["digest"],
        changed["source_graph"]["digest"],
        "all loaded source files must contribute to the ABI identity"
    );
}

#[test]
fn build_emits_deterministic_evm_wire_abi_with_selectors() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("vault.clear");
    fs::write(
        &source,
        r#"
            contract Vault version 1 {
                state { total: U64; }
                event Deposited { amount: U64; }
                mut function deposit(amount: U64) -> Bool { true }
            }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write source");
    let first = dir.path().join("first.evm-abi.json");
    let second = dir.path().join("second.evm-abi.json");

    for (wire_abi, wasm) in [
        (&first, dir.path().join("first.wasm")),
        (&second, dir.path().join("second.wasm")),
    ] {
        Command::cargo_bin("clg")
            .expect("clg binary")
            .args([
                "build",
                source.to_str().expect("source path"),
                "-o",
                wasm.to_str().expect("wasm path"),
                "--emit-evm-wire-abi",
                wire_abi.to_str().expect("wire ABI path"),
            ])
            .assert()
            .success();
    }

    let first_bytes = fs::read(&first).expect("read first wire ABI");
    assert_eq!(first_bytes, fs::read(&second).expect("read second wire ABI"));
    let wire_abi: Value = serde_json::from_slice(&first_bytes).expect("parse wire ABI");
    assert_eq!(wire_abi["wire_abi"]["format"], "clg.evm-wire-abi.v1");
    assert_eq!(wire_abi["target"]["profile"], "clg.evm-compatible.v1");
    assert_eq!(wire_abi["target"]["bytecode"], "deferred");
    assert_eq!(wire_abi["functions"][0]["signature"], "deposit(uint64)");
    assert_eq!(wire_abi["functions"][0]["selector"], "0x13765838");
    assert_eq!(
        wire_abi["events"][0]["topic0"],
        "0x7f2ef186ad31df7b1c02573d76e20029546e02b4fd1e8a696b110783d5834793"
    );
}

#[test]
fn build_emits_deterministic_deployable_evm_artifact_for_static_pure_contracts() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("constant.clear");
    fs::write(
        &source,
        r#"
            contract Constant version 1 {
                state { total: U64; }
                pure function answer() -> U64 { U64(42) }
            }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write source");
    let first = dir.path().join("first.evm.json");
    let second = dir.path().join("second.evm.json");

    for (artifact, wasm) in [
        (&first, dir.path().join("first.wasm")),
        (&second, dir.path().join("second.wasm")),
    ] {
        Command::cargo_bin("clg")
            .expect("clg binary")
            .args([
                "build",
                source.to_str().expect("source path"),
                "-o",
                wasm.to_str().expect("wasm path"),
                "--emit-evm-artifact",
                artifact.to_str().expect("artifact path"),
            ])
            .assert()
            .success();
    }

    let first_bytes = fs::read(&first).expect("read first artifact");
    assert_eq!(first_bytes, fs::read(&second).expect("read second artifact"));
    let artifact: Value = serde_json::from_slice(&first_bytes).expect("parse artifact");
    assert_eq!(artifact["artifact"]["format"], "clg.evm-artifact.v1");
    assert_eq!(artifact["target"]["profile"], "clg.evm-compatible.v1");
    assert_eq!(artifact["execution_profile"], "clg.evm-static-pure.v1");
    assert!(artifact["bytecode"]
        .as_str()
        .expect("bytecode")
        .starts_with("0x"));
}

#[test]
fn build_emits_stateful_evm_artifact_with_schema_bound_slots_and_transitions() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("counter.clear");
    fs::write(
        &source,
        r#"
            contract Counter version 1 {
                state { total: U64; }
                event Changed { total: U64; }
                init(initial: U64) { state.total = initial; 0 }
                pure function read() -> U64 { state.total }
                mut function set_total(value: U64) -> U64 {
                    state.total = value;
                    emit Changed(value);
                    state.total
                }
            }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write source");
    let artifact_path = dir.path().join("counter.evm.json");
    let replay_path = dir.path().join("counter-replay.evm.json");

    for (artifact, wasm) in [
        (&artifact_path, dir.path().join("counter.wasm")),
        (&replay_path, dir.path().join("counter-replay.wasm")),
    ] {
        Command::cargo_bin("clg")
            .expect("clg binary")
            .args([
                "build",
                source.to_str().expect("source path"),
                "-o",
                wasm.to_str().expect("Wasm path"),
                "--emit-evm-artifact",
                artifact.to_str().expect("artifact path"),
            ])
            .assert()
            .success();
    }

    let artifact_bytes = fs::read(&artifact_path).expect("read artifact");
    assert_eq!(artifact_bytes, fs::read(&replay_path).expect("read replay artifact"));
    let artifact: Value = serde_json::from_slice(&artifact_bytes).expect("parse artifact");
    assert_eq!(artifact["execution_profile"], "clg.evm-stateful-scalar.v1");
    assert_eq!(artifact["state_mapping"]["schema_digest"], artifact["state_schema"]["digest"]);
    let slots = artifact["state_mapping"]["storage_layout"]
        .as_array()
        .expect("storage layout");
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0]["field"], "total");
    assert!(slots[0]["field_id"].as_str().expect("field id").starts_with("sha256:"));
    assert_eq!(slots[0]["slot"].as_str().expect("slot").len(), 66);
    let mappings = artifact["state_mapping"]["transition_mapping"].to_string();
    assert!(mappings.contains("StateRead"));
    assert!(mappings.contains("StateWrite"));
    assert!(mappings.contains("EventEmit"));
}

#[test]
fn stateful_evm_artifact_rejects_unsupported_state_and_wasm_combinations() {
    let dir = tempdir().expect("tempdir");
    let unsupported = dir.path().join("unsupported.clear");
    let artifact = dir.path().join("unsupported.evm.json");
    fs::write(
        &unsupported,
        r#"
            contract Counter version 1 {
                state { owner: Bytes; }
                init(owner: Bytes) { state.owner = owner; 0 }
                pure function read() -> Bytes { state.owner }
            }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write unsupported source");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "build",
            unsupported.to_str().expect("source path"),
            "-o",
            dir.path().join("unsupported.wasm").to_str().expect("Wasm path"),
            "--emit-evm-artifact",
            artifact.to_str().expect("artifact path"),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("does not support state field `owner`"));
    assert!(!artifact.exists(), "unsupported source must not emit an EVM artifact");

    let supported = dir.path().join("supported.clear");
    fs::write(
        &supported,
        r#"
            contract Counter version 1 {
                state { total: U64; }
                init(initial: U64) { state.total = initial; 0 }
                pure function read() -> U64 { state.total }
            }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write supported source");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "build",
            supported.to_str().expect("source path"),
            "-o",
            dir.path().join("supported.wasm").to_str().expect("Wasm path"),
            "--emit-evm-artifact",
            dir.path().join("supported.evm.json").to_str().expect("artifact path"),
            "--emit-vcs",
            dir.path().join("supported.vcs.json").to_str().expect("VC path"),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot emit, validate, or sign a Wasm module"));
}
