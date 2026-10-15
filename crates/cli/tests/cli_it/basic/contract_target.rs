fn pilot_exercise_input() -> Value {
    serde_json::json!({
        "format": "clg.pilot-exercise-input.v1",
        "schema_version": 1,
        "target_profile": "clg.evm-sepolia-pilot.v1",
        "rpc_url": "http://127.0.0.1:9/",
        "chain_id": 11155111,
        "signer_reference": "operator/testnet-signer.reference",
        "sender": "0x1111111111111111111111111111111111111111",
        "release_bundle": "release/contract-release-bundle.json",
        "deploy": {
            "nonce": 7,
            "value": 0,
            "gas_limit": 250000,
            "fee": { "kind": "legacy-eip155-gas-price", "gas_price": 3 },
            "constructor_arguments": "release/constructor-args.json"
        },
        "invocation": {
            "contract_address": "0x2222222222222222222222222222222222222222",
            "function_selector": "0x13765838",
            "arguments": "release/authorized-set-value.args.json",
            "nonce": 8,
            "value": 0,
            "gas_limit": 100000,
            "fee": { "kind": "legacy-eip155-gas-price", "gas_price": 3 }
        }
    })
}

#[test]
fn pilot_preflight_displays_a_canonical_offline_operator_plan() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("pilot-input.json");
    let report = dir.path().join("pilot-report.json");
    fs::write(&input, serde_json::to_vec(&pilot_exercise_input()).expect("serialize input"))
        .expect("write input");

    // Port 9 is intentionally not served. Success proves preflight does not contact the endpoint.
    let output = Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "target",
            "pilot-preflight",
            "--input",
            input.to_str().expect("input path"),
            "--report-out",
            report.to_str().expect("report path"),
        ])
        .output()
        .expect("run pilot preflight");
    assert!(
        output.status.success(),
        "pilot preflight failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: Value = serde_json::from_slice(&output.stdout).expect("preflight stdout JSON");
    let report_bytes = fs::read(&report).expect("read report");
    let stored: Value = serde_json::from_slice(&report_bytes).expect("preflight report JSON");
    assert_eq!(stdout, stored, "stdout must display the written review record");
    assert_eq!(
        output.stdout,
        [report_bytes.as_slice(), b"\n"].concat(),
        "stdout must display the exact canonical report"
    );
    assert_eq!(stored["format"], "clg.pilot-exercise-preflight.v1");
    assert_eq!(stored["network"]["rpc_url"], "http://127.0.0.1:9");
    assert_eq!(stored["signer"]["reference"], "operator/testnet-signer.reference");
    assert_eq!(stored["release_bundle"], "release/contract-release-bundle.json");
    assert_eq!(stored["deploy"]["constructor_arguments"], "release/constructor-args.json");
    assert_eq!(stored["deploy"]["transaction"]["nonce"], 7);
    assert_eq!(stored["deploy"]["transaction"]["gas_limit"], 250000);
    assert_eq!(stored["deploy"]["transaction"]["fee"]["gas_price"], 3);
    assert_eq!(
        stored["invocation"]["arguments"],
        "release/authorized-set-value.args.json"
    );
    assert_eq!(
        stored["invocation"]["contract_address"],
        "0x2222222222222222222222222222222222222222"
    );
    assert_eq!(stored["invocation"]["function_selector"], "0x13765838");
    assert_eq!(stored["invocation"]["transaction"]["nonce"], 8);
    assert_eq!(stored["safety"]["network_contacted"], false);
    assert_eq!(stored["safety"]["signer_material_accessed"], false);
    assert_eq!(stored["safety"]["transaction_signed"], false);
    assert_eq!(stored["safety"]["transaction_submitted"], false);
}

#[test]
fn pilot_preflight_rejects_unsafe_or_unreviewable_operator_values() {
    let cases = [
        (
            "unsafe path",
            serde_json::json!({ "signer_reference": "../operator/key.reference" }),
            "pilot signer_reference must be a non-empty canonical relative path",
        ),
        (
            "credential endpoint",
            serde_json::json!({ "rpc_url": "https://token@rpc.example.invalid" }),
            "RPC endpoint credentials are unsupported",
        ),
        (
            "wrong chain",
            serde_json::json!({ "chain_id": 1 }),
            "requires explicit Sepolia chain ID `11155111`",
        ),
        (
            "invalid selector",
            serde_json::json!({ "invocation": { "function_selector": "0x01" } }),
            "function_selector must contain exactly four bytes",
        ),
    ];
    for (label, replacement, expected) in cases {
        let dir = tempdir().expect("tempdir");
        let input = dir.path().join("pilot-input.json");
        let report = dir.path().join("pilot-report.json");
        let mut value = pilot_exercise_input();
        for (key, replacement) in replacement.as_object().expect("replacement object") {
            if key == "invocation" {
                let selector = replacement["function_selector"].clone();
                value["invocation"]["function_selector"] = selector;
            } else {
                value[key] = replacement.clone();
            }
        }
        fs::write(&input, serde_json::to_vec(&value).expect("serialize input"))
            .expect("write input");
        Command::cargo_bin("clg")
            .expect("clg binary")
            .args([
                "target",
                "pilot-preflight",
                "--input",
                input.to_str().expect("input path"),
                "--report-out",
                report.to_str().expect("report path"),
            ])
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected));
        assert!(!report.exists(), "{label} must not create a review record");
    }
}

#[test]
fn target_commands_require_explicit_configuration_and_deploy_signing_policy() {
    let dir = tempdir().expect("tempdir");
    let artifact = dir.path().join("contract.wasm");
    let abi = dir.path().join("contract.abi.json");
    let receipt = dir.path().join("receipt.json");
    fs::write(
        &artifact,
        br#"{"bytecode":"0x6000","target":{"profile":"clg.evm-sepolia-pilot.v1"}}"#,
    )
    .expect("write artifact");
    fs::write(
        &abi,
        br#"{"functions":[],"target":{"profile":"clg.evm-sepolia-pilot.v1"},"wire_abi":{"digest":"sha256:test","format":"clg.evm-wire-abi.v1"}}"#,
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

#[test]
fn sepolia_pilot_profile_rejects_a_different_chain_before_rpc_contact() {
    let dir = tempdir().expect("tempdir");
    let receipt = dir.path().join("receipt.json");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "target",
            "deploy",
            "--target-profile",
            "clg.evm-sepolia-pilot.v1",
            "--rpc-url",
            "http://127.0.0.1:9",
            "--chain-id",
            "1",
            "--sender",
            "0x1111111111111111111111111111111111111111",
            "--nonce",
            "0",
            "--signing-key",
            "missing-signer.json",
            "--value",
            "0",
            "--gas-limit",
            "100000",
            "--gas-price",
            "1",
            "--artifact",
            "missing-artifact.json",
            "--abi",
            "missing-abi.json",
            "--contract-state-schema",
            "missing-schema.json",
            "--proof-artifact",
            "missing-proof.json",
            "--signed-bundle",
            "missing-bundle.json",
            "--receipt-out",
            receipt.to_str().expect("receipt path"),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "requires explicit Sepolia chain ID `11155111`",
        ));
    assert!(
        !receipt.exists(),
        "a rejected chain must not create a target receipt"
    );
}
