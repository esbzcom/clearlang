#[test]
fn target_commands_require_explicit_configuration_and_deploy_signing_policy() {
    let dir = tempdir().expect("tempdir");
    let artifact = dir.path().join("contract.wasm");
    let abi = dir.path().join("contract.abi.json");
    let receipt = dir.path().join("receipt.json");
    fs::write(
        &artifact,
        br#"{"bytecode":"0x6000","target":{"profile":"clg.evm-compatible.v1"}}"#,
    )
    .expect("write artifact");
    fs::write(
        &abi,
        br#"{"functions":[],"target":{"profile":"clg.evm-compatible.v1"},"wire_abi":{"digest":"sha256:test","format":"clg.evm-wire-abi.v1"}}"#,
    )
    .expect("write ABI");

    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "target",
            "deploy",
            "--rpc-url",
            "https://rpc.example.invalid",
            "--chain-id",
            "1",
            "--artifact",
            artifact.to_str().expect("artifact path"),
            "--abi",
            abi.to_str().expect("ABI path"),
            "--receipt-out",
            receipt.to_str().expect("receipt path"),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("--target-profile"))
        .stderr(predicates::str::contains("--sender"))
        .stderr(predicates::str::contains("--nonce"))
        .stderr(predicates::str::contains("--signing-key"))
        .stderr(predicates::str::contains("--value"))
        .stderr(predicates::str::contains("--gas-limit"))
        .stderr(predicates::str::contains("--gas-price"))
        .stderr(predicates::str::contains("--contract-state-schema"))
        .stderr(predicates::str::contains("--proof-artifact"))
        .stderr(predicates::str::contains("--signed-bundle"));
    assert!(!receipt.exists(), "an incomplete command must not create a receipt");
}

#[test]
fn target_invoke_requires_explicit_signing_sender_value_and_gas_policy() {
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["target", "invoke"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("--target-profile"))
        .stderr(predicates::str::contains("--sender"))
        .stderr(predicates::str::contains("--nonce"))
        .stderr(predicates::str::contains("--signing-key"))
        .stderr(predicates::str::contains("--value"))
        .stderr(predicates::str::contains("--gas-limit"))
        .stderr(predicates::str::contains("--gas-price"))
        .stderr(predicates::str::contains("--contract-state-schema"))
        .stderr(predicates::str::contains("--proof-artifact"))
        .stderr(predicates::str::contains("--signed-bundle"));
}
