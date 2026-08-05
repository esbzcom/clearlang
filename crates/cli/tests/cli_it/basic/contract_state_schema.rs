#[test]
fn build_emits_deterministic_contract_state_schema() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("vault.clear");
    fs::write(
        &source,
        r#"
            contract Vault version 1 {
                state { owner: Bytes; total: U64; balances: Map<Bytes, U64>; }
            }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write source");
    let first_schema = dir.path().join("first.schema.json");
    let second_schema = dir.path().join("second.schema.json");

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
}
