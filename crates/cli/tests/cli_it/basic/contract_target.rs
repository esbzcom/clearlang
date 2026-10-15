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

fn write_canonical_pilot_json(root: &Path, relative: &str, value: &Value) -> PathBuf {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(
        &path,
        clg_cli::commands::helpers::canonical_json_bytes(value),
    )
    .expect("write canonical pilot fixture");
    path
}

fn pilot_fixture_digest(path: &Path) -> String {
    format!(
        "sha256:{}",
        clg_cli::commands::helpers::sha256_hex(&fs::read(path).expect("read pilot fixture"))
    )
}

fn pilot_preflight_record() -> Value {
    serde_json::json!({
        "format": "clg.pilot-exercise-preflight.v1",
        "schema_version": 1,
        "network": {
            "chain_id": 11155111,
            "rpc_url": "https://operator.example.invalid",
            "target_profile": "clg.evm-sepolia-pilot.v1"
        },
        "signer": {
            "reference": "operator/signer.reference",
            "sender": "0x1111111111111111111111111111111111111111"
        },
        "deploy": {
            "constructor_arguments": "release/constructor.json",
            "transaction": {
                "fee": { "kind": "legacy-eip155-gas-price", "gas_price": 3 },
                "gas_limit": 250000,
                "nonce": 7,
                "value": 0
            }
        },
        "invocation": {
            "arguments": "release/invoke.json",
            "contract_address": "0x2222222222222222222222222222222222222222",
            "function_selector": "0x13765838",
            "transaction": {
                "fee": { "kind": "legacy-eip155-gas-price", "gas_price": 3 },
                "gas_limit": 100000,
                "nonce": 8,
                "value": 0
            }
        },
        "release_bundle": "release/bundle.json",
        "safety": {
            "network_contacted": false,
            "signer_material_accessed": false,
            "transaction_signed": false,
            "transaction_submitted": false
        }
    })
}

fn pilot_target_receipt(request: Value, transaction_hash: &str, signature_digest: &str) -> Value {
    let request_digest = format!(
        "sha256:{}",
        clg_cli::commands::helpers::sha256_hex(
            &clg_cli::commands::helpers::canonical_json_bytes(&request)
        )
    );
    serde_json::json!({
        "artifact_digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "block": {
            "hash": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "number": 10
        },
        "caller": request["from"],
        "chain_id": 11155111,
        "command": "invoke",
        "evidence": { "signed_bundle_digest": signature_digest },
        "format": "clg.target-receipt.v1",
        "request": request,
        "request_digest": request_digest,
        "result": { "status": "success" },
        "rpc_endpoint": "https://operator.example.invalid",
        "schema_version": 1,
        "target_profile": "clg.evm-sepolia-pilot.v1",
        "transaction_hash": transaction_hash,
        "wire_abi_digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
    })
}

fn pilot_rpc_observations(transaction_hash: &str, status: &str) -> Value {
    serde_json::json!({
        "format": "clg.pilot-rpc-observations.v1",
        "schema_version": 1,
        "target_profile": "clg.evm-sepolia-pilot.v1",
        "chain_id": 11155111,
        "exchanges": [
            {
                "request": { "method": "eth_chainId", "params": [] },
                "response": { "id": 1, "jsonrpc": "2.0", "result": "0xaa36a7" }
            },
            {
                "request": { "method": "eth_sendRawTransaction", "params": ["0xf86c"] },
                "response": { "id": 1, "jsonrpc": "2.0", "result": transaction_hash }
            },
            {
                "request": { "method": "eth_getTransactionReceipt", "params": [transaction_hash] },
                "response": { "id": 1, "jsonrpc": "2.0", "result": { "status": status } }
            }
        ]
    })
}

fn write_pilot_evidence_capture_fixture(root: &Path) -> (PathBuf, PathBuf) {
    let signature_digest = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    write_canonical_pilot_json(root, "intent.json", &pilot_preflight_record());
    let bundle = write_canonical_pilot_json(
        root,
        "release/bundle.json",
        &serde_json::json!({
            "format": "clg.contract-release-bundle.v1",
            "schema_version": 1,
            "target_profile": "clg.evm-sepolia-pilot.v1",
            "artifacts": { "signature": { "sha256": signature_digest } }
        }),
    );
    let bundle_digest = pilot_fixture_digest(&bundle);
    write_canonical_pilot_json(
        root,
        "verifier.json",
        &serde_json::json!({
            "format": "clg.pilot-release-verifier-result.v1",
            "schema_version": 1,
            "status": "verified",
            "release_bundle_digest": bundle_digest
        }),
    );
    let deploy_hash = "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    let invoke_hash = "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
    let denied_hash = "0xabababababababababababababababababababababababababababababababab";
    write_canonical_pilot_json(
        root,
        "rpc/deploy.json",
        &pilot_rpc_observations(deploy_hash, "0x1"),
    );
    write_canonical_pilot_json(
        root,
        "rpc/authorized.json",
        &pilot_rpc_observations(invoke_hash, "0x1"),
    );
    write_canonical_pilot_json(
        root,
        "rpc/unauthorized.json",
        &pilot_rpc_observations(denied_hash, "0x0"),
    );
    write_canonical_pilot_json(
        root,
        "receipts/deploy.json",
        &pilot_target_receipt(
            serde_json::json!({
                "chain_id": 11155111,
                "data": "0x6000",
                "from": "0x1111111111111111111111111111111111111111",
                "gas_limit": 250000,
                "gas_price": 3,
                "nonce": 7,
                "to": null,
                "value": 0
            }),
            deploy_hash,
            signature_digest,
        ),
    );
    write_canonical_pilot_json(
        root,
        "receipts/authorized.json",
        &pilot_target_receipt(
            serde_json::json!({
                "chain_id": 11155111,
                "data": "0x1376583800000000000000000000000000000000000000000000000000000000000001",
                "from": "0x1111111111111111111111111111111111111111",
                "gas_limit": 100000,
                "gas_price": 3,
                "nonce": 8,
                "to": "0x2222222222222222222222222222222222222222",
                "value": 0
            }),
            invoke_hash,
            signature_digest,
        ),
    );
    let capture = write_canonical_pilot_json(
        root,
        "capture.json",
        &serde_json::json!({
            "format": "clg.pilot-evidence-capture-input.v1",
            "schema_version": 1,
            "submission_intent": "intent.json",
            "release_bundle": "release/bundle.json",
            "verifier_result": "verifier.json",
            "observations": [
                {
                    "kind": "deploy",
                    "caller": "0x1111111111111111111111111111111111111111",
                    "rpc_observations": "rpc/deploy.json",
                    "receipt": "receipts/deploy.json"
                },
                {
                    "kind": "authorized-invoke",
                    "caller": "0x1111111111111111111111111111111111111111",
                    "rpc_observations": "rpc/authorized.json",
                    "receipt": "receipts/authorized.json"
                },
                {
                    "kind": "unauthorized-invoke",
                    "caller": "0x3333333333333333333333333333333333333333",
                    "rpc_observations": "rpc/unauthorized.json",
                    "receipt": null
                }
            ]
        }),
    );
    (capture, bundle)
}

#[test]
fn pilot_evidence_capture_binds_ordered_secret_free_offline_observations() {
    let dir = tempdir().expect("tempdir");
    let (capture, bundle) = write_pilot_evidence_capture_fixture(dir.path());
    let output_path = dir.path().join("pilot-evidence.json");
    let output = Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "target",
            "capture-pilot-evidence",
            "--root",
            dir.path().to_str().expect("root path"),
            "--input",
            capture
                .strip_prefix(dir.path())
                .expect("capture under root")
                .to_str()
                .expect("capture path"),
            "--out",
            output_path.to_str().expect("output path"),
        ])
        .output()
        .expect("capture pilot evidence");
    assert!(
        output.status.success(),
        "capture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = fs::read(&output_path).expect("read evidence");
    assert_eq!(output.stdout, [bytes.as_slice(), b"\n"].concat());
    let evidence: Value = serde_json::from_slice(&bytes).expect("parse evidence");
    assert_eq!(evidence["format"], "clg.pilot-evidence.v1");
    assert_eq!(evidence["release_bundle"]["digest"], pilot_fixture_digest(&bundle));
    assert_eq!(evidence["observations"].as_array().expect("observations").len(), 3);
    assert_eq!(evidence["observations"][0]["kind"], "deploy");
    assert_eq!(evidence["observations"][1]["kind"], "authorized-invoke");
    assert_eq!(evidence["observations"][2]["kind"], "unauthorized-invoke");
    assert!(evidence["observations"][2]["receipt"].is_null());
    assert_eq!(evidence["evidence_signature"]["status"], "not_authorized");
    assert!(!String::from_utf8(bytes)
        .expect("UTF-8 evidence")
        .contains("operator.example.invalid"));
}

#[test]
fn pilot_evidence_capture_rejects_identity_drift_and_secret_rpc_fields() {
    let cases = [
        (
            "signature drift",
            "receipt",
            "pilot target receipt does not bind the reviewed identity evidence",
        ),
        (
            "secret rpc field",
            "rpc",
            "contains forbidden secret or endpoint field `rpc_url`",
        ),
        (
            "unsafe capture path",
            "path",
            "pilot release bundle must be a non-empty canonical relative path",
        ),
        (
            "reordered observations",
            "order",
            "pilot evidence observations must be ordered",
        ),
    ];
    for (label, tamper, expected) in cases {
        let dir = tempdir().expect("tempdir");
        let (capture, _) = write_pilot_evidence_capture_fixture(dir.path());
        if tamper == "receipt" {
            let path = dir.path().join("receipts/deploy.json");
            let mut receipt: Value = serde_json::from_slice(&fs::read(&path).expect("read receipt"))
                .expect("parse receipt");
            receipt["evidence"]["signed_bundle_digest"] = serde_json::json!(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            );
            write_canonical_pilot_json(dir.path(), "receipts/deploy.json", &receipt);
        } else if tamper == "rpc" {
            let path = dir.path().join("rpc/deploy.json");
            let mut rpc: Value = serde_json::from_slice(&fs::read(&path).expect("read rpc"))
                .expect("parse rpc");
            rpc["rpc_url"] = serde_json::json!("https://secret.example.invalid");
            write_canonical_pilot_json(dir.path(), "rpc/deploy.json", &rpc);
        } else {
            let mut capture_value: Value =
                serde_json::from_slice(&fs::read(&capture).expect("read capture"))
                    .expect("parse capture");
            if tamper == "path" {
                capture_value["release_bundle"] = serde_json::json!("../release/bundle.json");
            } else {
                let observations = capture_value["observations"]
                    .as_array_mut()
                    .expect("capture observations");
                observations.swap(0, 1);
            }
            write_canonical_pilot_json(dir.path(), "capture.json", &capture_value);
        }
        Command::cargo_bin("clg")
            .expect("clg binary")
            .args([
                "target",
                "capture-pilot-evidence",
                "--root",
                dir.path().to_str().expect("root path"),
                "--input",
                capture
                    .strip_prefix(dir.path())
                    .expect("capture under root")
                    .to_str()
                    .expect("capture path"),
                "--out",
                dir.path().join("evidence.json").to_str().expect("output path"),
            ])
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected));
        assert!(
            !dir.path().join("evidence.json").exists(),
            "{label} must not create an evidence record"
        );
    }
}

fn capture_pilot_fixture(root: &Path) -> PathBuf {
    let (capture, _) = write_pilot_evidence_capture_fixture(root);
    let evidence = root.join("pilot-evidence.json");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "target",
            "capture-pilot-evidence",
            "--root",
            root.to_str().expect("root path"),
            "--input",
            capture
                .strip_prefix(root)
                .expect("capture under root")
                .to_str()
                .expect("capture path"),
            "--out",
            evidence.to_str().expect("evidence path"),
        ])
        .assert()
        .success();
    evidence
}

fn refresh_recorded_rpc_digest(evidence: &mut Value, observation: usize) {
    let chain_id = evidence["submission_intent"]["chain_id"].clone();
    let target_profile = evidence["submission_intent"]["target_profile"].clone();
    let exchanges = evidence["observations"][observation]["rpc_observations"]["exchanges"].clone();
    let fixture = serde_json::json!({
        "chain_id": chain_id,
        "exchanges": exchanges,
        "format": "clg.pilot-rpc-observations.v1",
        "schema_version": 1,
        "target_profile": target_profile
    });
    evidence["observations"][observation]["rpc_observations"]["digest"] = serde_json::json!(
        format!(
            "sha256:{}",
            clg_cli::commands::helpers::sha256_hex(
                &clg_cli::commands::helpers::canonical_json_bytes(&fixture)
            )
        )
    );
}

#[test]
fn pilot_evidence_replay_is_offline_and_serves_only_recorded_responses() {
    let dir = tempdir().expect("tempdir");
    let evidence = capture_pilot_fixture(dir.path());
    let report = dir.path().join("replay-report.json");
    let output = Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "target",
            "replay-pilot-evidence",
            "--input",
            evidence.to_str().expect("evidence path"),
            "--report-out",
            report.to_str().expect("report path"),
        ])
        .output()
        .expect("replay pilot evidence");
    assert!(
        output.status.success(),
        "replay failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = fs::read(&report).expect("read replay report");
    assert_eq!(output.stdout, [bytes.as_slice(), b"\n"].concat());
    let replay: Value = serde_json::from_slice(&bytes).expect("parse replay report");
    assert_eq!(replay["format"], "clg.pilot-evidence-replay-report.v1");
    assert_eq!(replay["status"], "verified");
    assert_eq!(replay["observations"][0]["exchanges_served"], 3);
    assert_eq!(replay["observations"][2]["final_receipt_status"], "0x0");
    assert_eq!(replay["safety"]["network_contacted"], false);
    assert_eq!(
        replay["safety"]["responses_served_only_from_recorded_fixtures"],
        true
    );
}

#[test]
fn pilot_evidence_replay_rejects_omitted_substituted_reordered_and_cross_chain_records() {
    let cases = [
        ("omitted", "pilot replay fixture has an unexpected final receipt status"),
        (
            "substituted",
            "pilot replay submission response does not match the recorded transaction digest",
        ),
        (
            "reordered",
            "pilot evidence observations are omitted, substituted, or reordered",
        ),
        ("cross-chain", "pilot replay fixture is for a different chain"),
    ];
    for (case, expected) in cases {
        let dir = tempdir().expect("tempdir");
        let evidence = capture_pilot_fixture(dir.path());
        let mut value: Value = serde_json::from_slice(&fs::read(&evidence).expect("read evidence"))
            .expect("parse evidence");
        match case {
            "omitted" => {
                value["observations"][0]["rpc_observations"]["exchanges"]
                    .as_array_mut()
                    .expect("exchanges")
                    .pop();
                refresh_recorded_rpc_digest(&mut value, 0);
            }
            "substituted" => {
                value["observations"][0]["rpc_observations"]["exchanges"][1]["response"]
                    ["result"] = serde_json::json!(
                    "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                );
                refresh_recorded_rpc_digest(&mut value, 0);
            }
            "reordered" => {
                value["observations"]
                    .as_array_mut()
                    .expect("observations")
                    .swap(0, 1);
            }
            "cross-chain" => {
                value["observations"][0]["rpc_observations"]["exchanges"][0]["response"]
                    ["result"] = serde_json::json!("0x1");
                refresh_recorded_rpc_digest(&mut value, 0);
            }
            _ => unreachable!("known replay tamper case"),
        }
        write_canonical_pilot_json(dir.path(), "pilot-evidence.json", &value);
        let report = dir.path().join("replay-report.json");
        Command::cargo_bin("clg")
            .expect("clg binary")
            .args([
                "target",
                "replay-pilot-evidence",
                "--input",
                evidence.to_str().expect("evidence path"),
                "--report-out",
                report.to_str().expect("report path"),
            ])
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected));
        assert!(!report.exists(), "{case} record must not produce a replay report");
    }
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
