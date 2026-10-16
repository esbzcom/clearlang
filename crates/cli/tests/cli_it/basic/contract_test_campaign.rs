#[test]
fn contract_test_campaign_is_seeded_replayable_and_captures_simulator_traces() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("counter.clear");
    let plan = dir.path().join("campaign.json");
    let report = dir.path().join("campaign.report.json");
    let traces = dir.path().join("campaign-traces");
    fs::write(
        &source,
        r#"
            contract Counter version 1 {
                state { total: U64; }
                mut function set_total(value: U64) -> U64 { state.total = value; state.total }
            }
        "#,
    )
    .expect("write contract source");
    fs::write(
        &plan,
        r#"{
            "format":"clg.contract-test-plan.v1",
            "schema_version":1,
            "seed":42,
            "cases":3,
            "function":"set_total",
            "state":{"total":0},
            "caller":"clg:test:campaign",
            "value":0,
            "block_number":7,
            "timestamp":8,
            "gas_limit":100,
            "memory_limit":1048576,
            "generators":[{"type":"u64","min":1,"max":9}],
            "properties":[
                {"kind":"result_equals_arg","arg":0},
                {"kind":"state_field_equals_arg","field":"total","arg":0}
            ]
        }"#,
    )
    .expect("write campaign plan");

    let run = || {
        Command::cargo_bin("clg")
            .expect("clg binary")
            .args([
                "contract", "test", source.to_str().expect("source path"),
                "--plan", plan.to_str().expect("plan path"),
                "--report-out", report.to_str().expect("report path"),
                "--trace-dir", traces.to_str().expect("trace dir"),
            ])
            .assert()
            .success();
        fs::read(&report).expect("read campaign report")
    };
    let first = run();
    let second = run();
    assert_eq!(first, second, "identical campaign inputs must replay byte-for-byte");

    let report: Value = serde_json::from_slice(&first).expect("parse campaign report");
    assert_eq!(report["format"], "clg.contract-test-report.v1");
    assert_eq!(report["status"], "success");
    assert_eq!(report["cases"].as_array().map(Vec::len), Some(3));
    let first_case = &report["cases"][0];
    assert_eq!(first_case["replay"]["argv"][0], "clg");
    assert_eq!(first_case["replay"]["argv"][1], "simulate");
    let trace: Value = serde_json::from_slice(
        &fs::read(first_case["trace"].as_str().expect("trace path")).expect("read trace"),
    )
    .expect("parse trace");
    assert_eq!(trace["trace_format"], "clg.contract-simulation-trace.v1");
    assert_eq!(trace["status"], "success");
}

#[test]
fn reference_configuration_registry_v1_public_fixture_replays_without_secrets() {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/projects/reference-config-registry/v1");
    let source = project.join("config_registry.clear");
    assert!(
        !fs::read(&source)
            .expect("read public reference contract source")
            .contains(&b'\r'),
        "public reference source must retain LF byte offsets used by its canonical traces"
    );
    let manifest: Value = serde_json::from_slice(
        &fs::read(project.join("fixture-manifest.json")).expect("read fixture manifest"),
    )
    .expect("parse fixture manifest");
    assert_eq!(manifest["format"], "clg.reference-contract-fixture-manifest.v1");
    assert_eq!(manifest["security"]["private_keys"], "forbidden");
    assert_eq!(manifest["security"]["live_endpoints"], "forbidden");
    for path in manifest["files"].as_object().expect("fixture paths").values() {
        let fixture = project.join(path.as_str().expect("fixture path"));
        assert!(fixture.is_file(), "manifested fixture must be checked in");
        assert!(
            !String::from_utf8(fs::read(&fixture).expect("read manifested fixture"))
                .expect("fixture UTF-8")
                .contains("private_key"),
            "public fixture format must not contain a private key"
        );
    }

    let signature_fixture: Value = serde_json::from_slice(
        &fs::read(project.join("release/public-signature-fixture.json"))
            .expect("read public signature fixture"),
    )
    .expect("parse public signature fixture");
    let release_signer: Value = serde_json::from_slice(
        &fs::read(project.join("release/public-release-signer.json"))
            .expect("read public release signer"),
    )
    .expect("parse public release signer");
    assert_eq!(release_signer["scheme"], signature_fixture["scheme"]);
    assert_eq!(release_signer["public_key"], signature_fixture["public_key"]);
    let public_key: [u8; 32] = hex::decode(signature_fixture["public_key"].as_str().expect("public key"))
        .expect("public key hex")
        .try_into()
        .expect("public key width");
    let signature = Signature::from_slice(
        &hex::decode(signature_fixture["signature"].as_str().expect("signature"))
            .expect("signature hex"),
    )
    .expect("signature width");
    VerifyingKey::from_bytes(&public_key)
        .expect("valid public key")
        .verify(signature_fixture["payload"].as_str().expect("payload").as_bytes(), &signature)
        .expect("public test signature must verify");

    let dir = tempdir().expect("tempdir");
    let owner = "0x1111111111111111111111111111111111111111";
    let non_owner = "0x2222222222222222222222222222222222222222";
    let run_simulation = |args: &str, caller: &str, name: &str| {
        let state_out = dir.path().join(format!("{name}.state.json"));
        let trace_out = dir.path().join(format!("{name}.trace.json"));
        let mut command = Command::cargo_bin("clg").expect("clg binary");
        command.args([
            "simulate",
            source.to_str().expect("source path"),
            "--function",
            "set_value",
            "--state",
            project.join("fixtures/initial-state.json").to_str().expect("state path"),
            "--args",
            project.join(args).to_str().expect("arguments path"),
            "--state-out",
            state_out.to_str().expect("state output path"),
            "--trace-out",
            trace_out.to_str().expect("trace output path"),
            "--caller",
            caller,
            "--value",
            "0",
            "--block-number",
            "7",
            "--timestamp",
            "8",
            "--gas-limit",
            "100",
            "--memory-limit",
            "1048576",
        ]);
        (command, state_out, trace_out)
    };

    let (mut owner_command, owner_state_out, owner_trace_out) =
        run_simulation("fixtures/owner-set-value.args.json", owner, "owner");
    owner_command.assert().success();
    assert!(owner_state_out.is_file());
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&owner_trace_out).expect("read owner trace"))
            .expect("parse owner trace"),
        serde_json::from_slice::<Value>(
            &fs::read(project.join("expected/owner-set-value.trace.json"))
                .expect("read expected owner trace"),
        )
        .expect("parse expected owner trace")
    );

    let (mut denied_command, denied_state_out, denied_trace_out) =
        run_simulation("fixtures/non-owner-set-value.args.json", non_owner, "non-owner");
    denied_command
        .assert()
        .failure()
        .stderr(predicate::str::contains("simulator contract guard failed"));
    assert!(!denied_state_out.exists(), "rejected transition must not commit state");
    let mut denied_trace: Value =
        serde_json::from_slice(&fs::read(denied_trace_out).expect("read rejected trace"))
            .expect("parse rejected trace");
    denied_trace["failure_location"]["file"] = Value::String("config_registry.clear".to_string());
    for frame in denied_trace["stack"].as_array_mut().expect("failure stack") {
        frame["file"] = Value::String("config_registry.clear".to_string());
    }
    assert_eq!(
        denied_trace,
        serde_json::from_slice::<Value>(
            &fs::read(project.join("expected/non-owner-set-value.trace.json"))
                .expect("read expected rejected trace"),
        )
        .expect("parse expected rejected trace")
    );

    let campaign_report = dir.path().join("campaign.report.json");
    let campaign_traces = dir.path().join("campaign-traces");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "contract",
            "test",
            source.to_str().expect("source path"),
            "--plan",
            project
                .join("plans/owner-update.campaign.json")
                .to_str()
                .expect("campaign plan path"),
            "--report-out",
            campaign_report.to_str().expect("campaign report path"),
            "--trace-dir",
            campaign_traces.to_str().expect("campaign trace path"),
        ])
        .assert()
        .success();
    let campaign: Value =
        serde_json::from_slice(&fs::read(campaign_report).expect("read campaign report"))
            .expect("parse campaign report");
    assert_eq!(campaign["status"], "success");
    assert_eq!(campaign["cases"].as_array().map(Vec::len), Some(3));
}

#[test]
fn reference_configuration_registry_migrates_releases_and_binds_target_receipts() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/projects/reference-config-registry");
    let dir = tempdir().expect("tempdir");
    let v1_source = dir.path().join("config-registry-v1.clear");
    let v1_plan = dir.path().join("config-registry-v1.campaign.json");
    let v2_source = dir.path().join("config-registry-v2.clear");
    let v2_plan = dir.path().join("config-registry-v2.campaign.json");
    fs::copy(root.join("v1/config_registry.clear"), &v1_source).expect("copy v1 source");
    fs::copy(root.join("v1/plans/owner-update.campaign.json"), &v1_plan)
        .expect("copy v1 campaign plan");
    fs::copy(root.join("v2/config_registry.clear"), &v2_source).expect("copy v2 source");
    fs::copy(root.join("v2/plans/owner-update.campaign.json"), &v2_plan)
        .expect("copy v2 campaign plan");
    write_minimal_strict_contract_inputs(&dir);
    let (key, pubkey) = write_release_key_material(&dir);
    let solver = write_verified_unsat_solver(&dir);

    let v1_release = dir.path().join("v1-release");
    run_reference_contract_release(&v1_source, &v1_plan, &key, &pubkey, &v1_release, &solver);
    verify_contract_release_bundle(&v1_release, "config-registry-v1", &pubkey);
    let v1_schema_path = v1_release.join("config-registry-v1.schema.json");
    let v1_schema: Value =
        serde_json::from_slice(&fs::read(&v1_schema_path).expect("read v1 schema"))
            .expect("parse v1 schema");
    let v1_digest = v1_schema["schema"]["digest"]
        .as_str()
        .expect("v1 schema digest");
    let migration: Value = serde_json::from_slice(
        &fs::read(root.join("v2/migration-fixture.json")).expect("read migration fixture"),
    )
    .expect("parse migration fixture");
    assert_eq!(migration["from"]["schema_digest"], v1_digest);
    assert_eq!(
        v1_schema["caller_authorization"]["transitions"][0]["function"],
        "set_value"
    );
    let v1_proof: Value = serde_json::from_slice(
        &fs::read(v1_release.join("config-registry-v1.proof.json")).expect("read v1 proof"),
    )
    .expect("parse v1 proof");
    assert!(
        v1_proof["summary"]["total_vcs"].as_u64().expect("total VCs") > 0,
        "owner-authorized transition must produce proof obligations"
    );
    assert_eq!(
        v1_proof["summary"]["total_vcs"],
        v1_proof["summary"]["proved_count"],
        "all owner-update obligations must be proved"
    );
    for field in ["failed_count", "unknown_count", "timeout_count", "generated_count"] {
        assert_eq!(v1_proof["summary"][field], 0, "v1 proof {field}");
    }

    let rejected_state = dir.path().join("rejected-state.json");
    let rejected_trace = dir.path().join("rejected-trace.json");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "simulate",
            v1_source.to_str().expect("v1 source path"),
            "--function",
            "set_value",
            "--state",
            root.join("v1/fixtures/initial-state.json")
                .to_str()
                .expect("state path"),
            "--args",
            root.join("v1/fixtures/non-owner-set-value.args.json")
                .to_str()
                .expect("arguments path"),
            "--state-out",
            rejected_state.to_str().expect("rejected state path"),
            "--trace-out",
            rejected_trace.to_str().expect("rejected trace path"),
            "--caller",
            "0x2222222222222222222222222222222222222222",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("simulator contract guard failed"));
    assert!(!rejected_state.exists(), "non-owner update must not commit state");

    let v2_release = dir.path().join("v2-release");
    run_contract_release(
        &v2_source,
        &v2_plan,
        &key,
        &pubkey,
        &v2_release,
        &solver,
        Some(&v1_schema_path),
    );
    let v2_schema: Value = serde_json::from_slice(
        &fs::read(v2_release.join("config-registry-v2.schema.json")).expect("read v2 schema"),
    )
    .expect("parse v2 schema");
    assert_eq!(v2_schema["migration"]["from_schema"], v1_digest);
    assert_eq!(v2_schema["fields"][0], v1_schema["fields"][0]);
    assert_eq!(v2_schema["fields"][1], v1_schema["fields"][1]);
    assert_eq!(v2_schema["fields"][2]["name"], "revision");

    let independent_release = dir.path().join("independent-v2-release");
    copy_release_tree(&v2_release, &independent_release);
    let independent_pubkey = dir.path().join("independent-release-pubkey.json");
    fs::copy(&pubkey, &independent_pubkey).expect("copy public key for independent verification");
    verify_contract_release_bundle(
        &independent_release,
        "config-registry-v2",
        &independent_pubkey,
    );

    let artifact = v1_release.join("config-registry-v1.evm.json");
    let wire_abi = v1_release.join("config-registry-v1.evm-wire-abi.json");
    let proof = v1_release.join("config-registry-v1.proof.json");
    let signature = v1_release.join("config-registry-v1.sig.json");
    let target_key = dir.path().join("target-signer.json");
    fs::write(
        &target_key,
        r#"{"format":"clg.evm-signing-key.v1","private_key":"0x0101010101010101010101010101010101010101010101010101010101010101","address":"0x1a642f0e3c3af545e7acbd38b07251b3990914f1"}"#,
    )
    .expect("write target signing key");
    let constructor_args = dir.path().join("constructor.json");
    fs::write(
        &constructor_args,
        br#"["0x1a642f0e3c3af545e7acbd38b07251b3990914f1",0]"#,
    )
    .expect("write constructor arguments");
    let deploy_receipt = dir.path().join("deploy-receipt.json");
    let (deploy_endpoint, deploy_server) = mock_target_rpc(Some(
        "0x2222222222222222222222222222222222222222",
    ));
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["target", "deploy", "--target-profile", "clg.evm-sepolia-pilot.v1"])
        .args(["--rpc-url"])
        .arg(&deploy_endpoint)
        .args(["--chain-id", "11155111", "--sender", "0x1a642f0e3c3af545e7acbd38b07251b3990914f1", "--nonce", "0"])
        .args(["--signing-key"])
        .arg(&target_key)
        .args(["--value", "0", "--gas-limit", "100000", "--gas-price", "1", "--artifact"])
        .arg(&artifact)
        .args(["--abi"])
        .arg(&wire_abi)
        .args(["--contract-state-schema"])
        .arg(&v1_schema_path)
        .args(["--proof-artifact"])
        .arg(&proof)
        .args(["--signed-bundle"])
        .arg(&signature)
        .args(["--args"])
        .arg(&constructor_args)
        .args(["--receipt-out"])
        .arg(&deploy_receipt)
        .assert()
        .success();
    assert_eq!(deploy_server.join().expect("join deploy RPC").len(), 3);

    let wire: Value = serde_json::from_slice(&fs::read(&wire_abi).expect("read wire ABI"))
        .expect("parse wire ABI");
    let selector = wire["functions"]
        .as_array()
        .and_then(|functions| functions.iter().find(|function| function["name"] == "set_value"))
        .and_then(|function| function["selector"].as_str())
        .expect("set_value selector");
    let invoke_args = dir.path().join("invoke.json");
    fs::write(&invoke_args, br#"[7]"#).expect("write invocation arguments");
    let invoke_receipt = dir.path().join("invoke-receipt.json");
    let (invoke_endpoint, invoke_server) = mock_target_rpc(None);
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["target", "invoke", "--target-profile", "clg.evm-sepolia-pilot.v1"])
        .args(["--rpc-url"])
        .arg(&invoke_endpoint)
        .args(["--chain-id", "11155111", "--artifact"])
        .arg(&artifact)
        .args(["--abi"])
        .arg(&wire_abi)
        .args(["--contract-state-schema"])
        .arg(&v1_schema_path)
        .args(["--proof-artifact"])
        .arg(&proof)
        .args(["--signed-bundle"])
        .arg(&signature)
        .args(["--contract-address", "0x2222222222222222222222222222222222222222", "--function"])
        .arg(selector)
        .args(["--args"])
        .arg(&invoke_args)
        .args(["--sender", "0x1a642f0e3c3af545e7acbd38b07251b3990914f1", "--nonce", "1", "--signing-key"])
        .arg(&target_key)
        .args(["--value", "0", "--gas-limit", "100000", "--gas-price", "1", "--receipt-out"])
        .arg(&invoke_receipt)
        .assert()
        .success();
    assert_eq!(invoke_server.join().expect("join invoke RPC").len(), 3);

    let receipt: Value = serde_json::from_slice(&fs::read(&invoke_receipt).expect("read receipt"))
        .expect("parse receipt");
    let artifact_value: Value = serde_json::from_slice(&fs::read(&artifact).expect("read artifact"))
        .expect("parse artifact");
    let proof_value: Value = serde_json::from_slice(&fs::read(&proof).expect("read proof"))
        .expect("parse proof");
    assert_eq!(receipt["caller"], "0x1a642f0e3c3af545e7acbd38b07251b3990914f1");
    assert_eq!(receipt["function"], selector);
    assert_eq!(receipt["artifact_digest"], format!("sha256:{}", hex::encode(Sha256::digest(fs::read(&artifact).expect("read artifact bytes")))));
    assert_eq!(receipt["wire_abi_digest"], wire["wire_abi"]["digest"]);
    assert_eq!(receipt["evidence"]["schema_digest"], v1_schema["schema"]["digest"]);
    assert_eq!(receipt["evidence"]["proof_artifact_digest"], format!("sha256:{}", hex::encode(Sha256::digest(serde_json::to_vec(&proof_value).expect("canonical proof value")))));
    assert_eq!(receipt["evidence"]["source_graph"], artifact_value["source_graph"]);
    assert_eq!(receipt["evidence"]["signed_bundle_digest"], format!("sha256:{}", hex::encode(Sha256::digest(fs::read(&signature).expect("read signature bytes")))));
}

#[test]
fn caller_aware_campaign_replays_deterministically_and_rejects_substituted_callers() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("config_registry.clear");
    let plan = dir.path().join("caller-campaign.json");
    let report = dir.path().join("caller-campaign.report.json");
    let traces = dir.path().join("caller-campaign-traces");
    let owner = "0x1111111111111111111111111111111111111111";
    let substituted_caller = "0x2222222222222222222222222222222222222222";
    fs::write(
        &source,
        r#"
            import std::eth::{Address}

            contract ConfigRegistry version 1 {
                state { owner: std::eth::Address; value: U64; }
                init(owner: std::eth::Address, value: U64) {
                    state.owner = owner;
                    state.value = value;
                    0
                }
                mut function set_value(next: U64) -> U64
                    require { std::eth::address::equals(std::eth::context::caller(), state.owner) }
                {
                    state.value = next;
                    next
                }
            }
        "#,
    )
    .expect("write caller-aware contract source");
    fs::write(
        &plan,
        format!(
            r#"{{
                "format":"clg.contract-test-plan.v1",
                "schema_version":1,
                "seed":71,
                "cases":3,
                "function":"set_value",
                "state":{{"owner":"{owner}","value":0}},
                "caller":"{owner}",
                "value":0,
                "block_number":7,
                "timestamp":8,
                "gas_limit":100,
                "memory_limit":1048576,
                "generators":[{{"type":"u64","min":1,"max":9}}],
                "properties":[
                    {{"kind":"result_equals_arg","arg":0}},
                    {{"kind":"state_field_equals_arg","field":"value","arg":0}}
                ]
            }}"#,
        ),
    )
    .expect("write caller-aware campaign plan");

    let run_campaign = |report: &Path, trace_dir: &Path| {
        Command::cargo_bin("clg")
            .expect("clg binary")
            .args([
                "contract",
                "test",
                source.to_str().expect("source path"),
                "--plan",
                plan.to_str().expect("plan path"),
                "--report-out",
                report.to_str().expect("report path"),
                "--trace-dir",
                trace_dir.to_str().expect("trace directory path"),
            ])
            .assert()
            .success();
        fs::read(report).expect("read campaign report")
    };

    let first_report = run_campaign(&report, &traces);
    let first_trace = fs::read(traces.join("case-0000.trace.json")).expect("read first trace");
    let second_report = run_campaign(&report, &traces);
    let second_trace = fs::read(traces.join("case-0000.trace.json")).expect("read replay trace");
    assert_eq!(
        first_report, second_report,
        "identical caller-aware campaign inputs must reproduce byte-identical evidence"
    );
    assert_eq!(
        first_trace, second_trace,
        "identical caller-aware campaign inputs must reproduce byte-identical traces"
    );

    let report_value: Value = serde_json::from_slice(&first_report).expect("parse campaign report");
    assert_eq!(report_value["status"], "success");
    let trace_value: Value = serde_json::from_slice(&first_trace).expect("parse successful trace");
    assert_eq!(trace_value["status"], "success");
    assert_eq!(trace_value["caller"], owner);
    assert_eq!(
        report_value["cases"][0]["replay"]["argv"],
        serde_json::json!([
            "clg",
            "simulate",
            source.to_str().expect("source path"),
            "--function",
            "set_value",
            "--state",
            traces.join("case-0000.state.json").to_str().expect("state path"),
            "--args",
            traces.join("case-0000.args.json").to_str().expect("arguments path"),
            "--state-out",
            traces.join("case-0000.state-out.json").to_str().expect("state output path"),
            "--trace-out",
            traces.join("case-0000.trace.json").to_str().expect("trace path"),
            "--caller",
            owner,
            "--value",
            "0",
            "--block-number",
            "7",
            "--timestamp",
            "8",
            "--gas-limit",
            "100",
            "--memory-limit",
            "1048576"
        ]),
        "replay evidence must bind the canonical caller"
    );

    let mut substituted_plan: Value =
        serde_json::from_slice(&fs::read(&plan).expect("read campaign plan"))
            .expect("parse campaign plan");
    substituted_plan["caller"] = serde_json::json!(substituted_caller);
    fs::write(
        &plan,
        serde_json::to_vec(&substituted_plan).expect("serialize substituted campaign plan"),
    )
    .expect("write substituted campaign plan");
    let substituted_report = dir.path().join("substituted-caller.report.json");
    let substituted_traces = dir.path().join("substituted-caller-traces");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "contract",
            "test",
            source.to_str().expect("source path"),
            "--plan",
            plan.to_str().expect("plan path"),
            "--report-out",
            substituted_report.to_str().expect("report path"),
            "--trace-dir",
            substituted_traces.to_str().expect("trace directory path"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("contract campaign failed"))
        .stderr(predicate::str::contains("contract guard failed"));
    assert!(
        !substituted_traces.join("case-0000.state-out.json").exists(),
        "an unauthorized caller must not produce committed state"
    );
    let substituted_trace: Value = serde_json::from_slice(
        &fs::read(substituted_traces.join("case-0000.trace.json"))
            .expect("read substituted caller trace"),
    )
    .expect("parse substituted caller trace");
    assert_eq!(substituted_trace["status"], "failure");
    assert_eq!(substituted_trace["caller"], substituted_caller);
    assert_eq!(substituted_trace["state_before"], substituted_trace["state_after"]);
    assert_eq!(substituted_trace["state_changes"], serde_json::json!([]));
    assert_eq!(substituted_trace["events"], serde_json::json!([]));
}

#[test]
fn contract_release_help_exposes_the_complete_evidence_contract() {
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["contract", "release", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--plan <FILE>"))
        .stdout(predicate::str::contains("--key <FILE>"))
        .stdout(predicate::str::contains("--pubkey <FILE>"))
        .stdout(predicate::str::contains("--out-dir <DIR>"))
        .stdout(predicate::str::contains("--prior-state-schema <FILE>"));

    Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["contract", "verify-release", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--bundle <FILE>"))
        .stdout(predicate::str::contains("--pubkey <FILE>"));

    Command::cargo_bin("clg-contract-verifier")
        .expect("independent verifier binary")
        .args(["verify-release", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--bundle <FILE>"))
        .stdout(predicate::str::contains("--pubkey <FILE>"));
}

#[test]
fn independent_verifier_matches_primary_verifier_canonical_result() {
    let (_workspace, pubkey, out_dir) = prepared_reference_release();
    let bundle = out_dir.join("counter.contract-release-bundle.json");

    let primary = Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["contract", "verify-release", "--bundle"])
        .arg(&bundle)
        .args(["--pubkey"])
        .arg(&pubkey)
        .output()
        .expect("run primary verifier");
    assert!(primary.status.success(), "primary verifier must succeed");

    let independent = Command::cargo_bin("clg-contract-verifier")
        .expect("independent verifier binary")
        .args(["verify-release", "--bundle"])
        .arg(&bundle)
        .args(["--pubkey"])
        .arg(&pubkey)
        .output()
        .expect("run independent verifier");
    assert!(
        independent.status.success(),
        "independent verifier must succeed"
    );
    assert_eq!(
        independent.stdout, primary.stdout,
        "both commands must render the shared verifier result byte-for-byte"
    );

    let result: Value = serde_json::from_slice(&independent.stdout).expect("parse verifier result");
    let canonical_result = serde_json::to_vec(&result).expect("serialize canonical verifier result");
    assert_eq!(
        independent.stdout,
        [canonical_result.as_slice(), b"\n"].concat(),
        "the verifier result must be canonical bytes for offline pilot-evidence capture"
    );
    assert_eq!(result["format"], "clg.contract-release-verifier-result.v1");
    assert_eq!(result["schema_version"], 1);
    assert_eq!(result["status"], "verified");
    assert_eq!(result["target_profile"], "clg.evm-sepolia-pilot.v1");
    assert_eq!(result["verifier_version"], env!("CARGO_PKG_VERSION"));
    assert!(
        result["bundle_sha256"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "the canonical result must bind the exact bundle bytes"
    );
    assert!(
        !String::from_utf8_lossy(&independent.stdout).contains(out_dir.to_string_lossy().as_ref()),
        "the canonical result must not leak local paths"
    );
}

#[test]
fn published_offline_verifier_walkthrough_is_current_and_reproducible_without_network() {
    const WALKTHROUGH: &str = include_str!("../../../../../docs/release/contract-verifier-offline-walkthrough.md");
    for required in [
        "clg-contract-verifier-v$Version-windows-x86_64.zip",
        "clg-contract-verifier-v$Version-linux-x86_64.tar.gz",
        "clg-contract-verifier-v$Version-macos-x86_64.tar.gz",
        "clg-contract-verifier-v$Version-checksums.txt",
        "clg-contract-verifier-v$Version-manifest.json",
        "clg-contract-verifier-v$Version-manifest.sig.json",
        "clg.contract-verifier-distribution-manifest.v1",
        "clg.contract-verifier-distribution-signature.v1",
        "clg.contract-release-verifier-result.v1",
        "--json-errors verify-release --bundle $BundlePath --pubkey $ReleasePublicKeyPath",
        "It never invokes the verifier until those checks pass.",
        "REJECTED: C140",
    ] {
        assert!(
            WALKTHROUGH.contains(required),
            "offline verifier walkthrough is missing required current contract `{required}`"
        );
    }

    let (workspace, pubkey, release) = prepared_reference_release();
    let independent = tempdir().expect("independent walkthrough directory");
    let evidence_root = independent.path().join("received-contract-release");
    let verifier_root = independent.path().join("received-verifier-package");
    fs::create_dir_all(&verifier_root).expect("create independent verifier package directory");
    copy_release_tree(&release, &evidence_root);
    let independent_pubkey = independent.path().join("release-public-key.json");
    fs::copy(&pubkey, &independent_pubkey).expect("copy explicit release public key");
    let verifier = verifier_root.join(if cfg!(windows) {
        "clg-contract-verifier.exe"
    } else {
        "clg-contract-verifier"
    });
    fs::copy(assert_cmd::cargo::cargo_bin("clg-contract-verifier"), &verifier)
        .expect("copy independent verifier executable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&verifier)
            .expect("read verifier permissions")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&verifier, permissions).expect("set verifier executable bit");
    }
    fs::remove_file(workspace.path().join("counter.clear"))
        .expect("remove source outside independent walkthrough directory");

    let proxy = std::net::TcpListener::bind("127.0.0.1:0").expect("bind offline proxy trap");
    proxy.set_nonblocking(true).expect("make proxy trap nonblocking");
    let proxy_url = format!("http://{}", proxy.local_addr().expect("proxy address"));
    let output = Command::new(&verifier)
        .current_dir(independent.path())
        .env("HTTP_PROXY", &proxy_url)
        .env("HTTPS_PROXY", &proxy_url)
        .env("ALL_PROXY", &proxy_url)
        .env("NO_PROXY", "")
        .args(["--json-errors", "verify-release", "--bundle"])
        .arg(evidence_root.join("counter.contract-release-bundle.json"))
        .args(["--pubkey"])
        .arg(&independent_pubkey)
        .output()
        .expect("run documented independent verifier command");
    assert!(
        output.status.success(),
        "documented offline verifier command must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        matches!(proxy.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "documented offline verifier command must not contact an ambient proxy"
    );

    let retained = independent
        .path()
        .join("contract-verification-result.json");
    let canonical_output = output
        .stdout
        .strip_suffix(b"\n")
        .expect("verifier stdout has its terminal newline");
    fs::write(&retained, canonical_output).expect("retain canonical walkthrough result");
    let result: Value = serde_json::from_slice(canonical_output).expect("parse verifier result");
    assert_eq!(result["format"], "clg.contract-release-verifier-result.v1");
    assert_eq!(result["schema_version"], 1);
    assert_eq!(result["status"], "verified");
    assert_eq!(
        fs::read(&retained).expect("read retained walkthrough result"),
        serde_json::to_vec(&result).expect("serialize canonical walkthrough result"),
        "the walkthrough retention step must leave canonical JSON bytes without a trailing newline"
    );
}

#[test]
fn contract_release_reference_fixture_completes_release_target_and_independent_verification() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("counter.clear");
    let plan = dir.path().join("campaign.json");
    let out_dir = dir.path().join("release");
    fs::write(
        &source,
        r#"
            contract Counter version 1 {
                state { total: U64; }
                init(initial: U64) { state.total = initial; 0 }
                mut function set_total(value: U64) -> U64 { state.total = value; state.total }
            }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write contract source");
    fs::write(
        &plan,
        r#"{
            "format":"clg.contract-test-plan.v1",
            "schema_version":1,
            "seed":42,
            "cases":1,
            "function":"set_total",
            "state":{"total":0},
            "caller":"clg:test:campaign",
            "value":0,
            "block_number":7,
            "timestamp":8,
            "gas_limit":100,
            "memory_limit":1048576,
            "generators":[{"type":"u64","min":1,"max":9}],
            "properties":[
                {"kind":"result_equals_arg","arg":0},
                {"kind":"state_field_equals_arg","field":"total","arg":0}
            ]
        }"#,
    )
    .expect("write campaign plan");
    write_minimal_strict_contract_inputs(&dir);
    let (key, pubkey) = write_release_key_material(&dir);
    let solver = write_verified_unsat_solver(&dir);

    run_reference_contract_release(&source, &plan, &key, &pubkey, &out_dir, &solver);

    let bundle = out_dir.join("counter.contract-release-bundle.json");
    assert!(bundle.is_file(), "release must emit a bundle");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["contract", "verify-release", "--bundle"])
        .arg(&bundle)
        .args(["--pubkey"])
        .arg(&pubkey)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"verified\""));

    let artifact = out_dir.join("counter.evm.json");
    let wire_abi = out_dir.join("counter.evm-wire-abi.json");
    let schema = out_dir.join("counter.schema.json");
    let proof = out_dir.join("counter.proof.json");
    let signature = out_dir.join("counter.sig.json");
    let target_key = dir.path().join("target-signer.json");
    fs::write(
        &target_key,
        r#"{"format":"clg.evm-signing-key.v1","private_key":"0x0101010101010101010101010101010101010101010101010101010101010101","address":"0x1a642f0e3c3af545e7acbd38b07251b3990914f1"}"#,
    )
    .expect("write target signing key");
    let constructor_args = dir.path().join("constructor.json");
    let deploy_receipt = dir.path().join("deploy-receipt.json");
    fs::write(&constructor_args, br#"{"initial":0}"#).expect("write constructor arguments");
    let (deploy_endpoint, deploy_server) = mock_target_rpc(Some(
        "0x2222222222222222222222222222222222222222",
    ));
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["target", "deploy", "--target-profile", "clg.evm-sepolia-pilot.v1"])
        .args(["--rpc-url"])
        .arg(&deploy_endpoint)
        .args(["--chain-id", "11155111", "--sender", "0x1a642f0e3c3af545e7acbd38b07251b3990914f1", "--nonce", "0"])
        .args(["--signing-key"])
        .arg(&target_key)
        .args(["--value", "0", "--gas-limit", "100000", "--gas-price", "1", "--artifact"])
        .arg(&artifact)
        .args(["--abi"])
        .arg(&wire_abi)
        .args(["--contract-state-schema"])
        .arg(&schema)
        .args(["--proof-artifact"])
        .arg(&proof)
        .args(["--signed-bundle"])
        .arg(&signature)
        .args(["--args"])
        .arg(&constructor_args)
        .args(["--receipt-out"])
        .arg(&deploy_receipt)
        .assert()
        .success();
    assert_eq!(
        deploy_server
            .join()
            .expect("join deploy RPC")
            .iter()
            .map(|request| request["method"].as_str())
            .collect::<Vec<_>>(),
        [
            Some("eth_chainId"),
            Some("eth_sendRawTransaction"),
            Some("eth_getTransactionReceipt"),
        ]
    );

    let wire: Value = serde_json::from_slice(&fs::read(&wire_abi).expect("read wire ABI"))
        .expect("parse wire ABI");
    let selector = wire["functions"]
        .as_array()
        .and_then(|functions| functions.iter().find(|function| function["name"] == "set_total"))
        .and_then(|function| function["selector"].as_str())
        .expect("set_total selector")
        .to_string();
    let invoke_args = dir.path().join("invoke.json");
    let invoke_receipt = dir.path().join("invoke-receipt.json");
    fs::write(&invoke_args, br#"[4]"#).expect("write invocation arguments");
    let (invoke_endpoint, invoke_server) = mock_target_rpc(None);
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["target", "invoke", "--target-profile", "clg.evm-sepolia-pilot.v1"])
        .args(["--rpc-url"])
        .arg(&invoke_endpoint)
        .args(["--chain-id", "11155111", "--artifact"])
        .arg(&artifact)
        .args(["--abi"])
        .arg(&wire_abi)
        .args(["--contract-state-schema"])
        .arg(&schema)
        .args(["--proof-artifact"])
        .arg(&proof)
        .args(["--signed-bundle"])
        .arg(&signature)
        .args(["--contract-address", "0x2222222222222222222222222222222222222222", "--function"])
        .arg(&selector)
        .args(["--args"])
        .arg(&invoke_args)
        .args(["--sender", "0x1a642f0e3c3af545e7acbd38b07251b3990914f1", "--nonce", "1", "--signing-key"])
        .arg(&target_key)
        .args(["--value", "0", "--gas-limit", "100000", "--gas-price", "1", "--receipt-out"])
        .arg(&invoke_receipt)
        .assert()
        .success();
    assert!(deploy_receipt.is_file());
    assert!(invoke_receipt.is_file());
    assert_eq!(invoke_server.join().expect("join invoke RPC").len(), 3);
    retain_release_fixture("happy-path", &out_dir);
}

#[test]
fn contract_release_reference_fixture_is_reproducible_under_controlled_inputs() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("counter.clear");
    let plan = dir.path().join("campaign.json");
    let out_dir = dir.path().join("release");
    write_reference_counter_source_and_plan(&source, &plan);
    write_minimal_strict_contract_inputs(&dir);
    let (key, pubkey) = write_release_key_material(&dir);
    let solver = write_verified_unsat_solver(&dir);

    run_reference_contract_release(&source, &plan, &key, &pubkey, &out_dir, &solver);
    verify_reference_release_bundle(&out_dir, &pubkey);
    let stable_names = [
        "counter.evm.json",
        "counter.schema.json",
        "counter.abi.json",
        "counter.evm-wire-abi.json",
        "counter.vcs.json",
        "counter.proof.json",
        "counter.campaign.json",
        "counter.campaign-evidence.json",
    ];
    let first_stable_artifacts = stable_names
        .iter()
        .map(|name| (*name, fs::read(out_dir.join(name)).expect("read stable release artifact")))
        .collect::<Vec<_>>();
    let bundle = out_dir.join("counter.contract-release-bundle.json");
    let first_bundle_projection = reproducible_bundle_projection(&bundle);

    run_reference_contract_release(&source, &plan, &key, &pubkey, &out_dir, &solver);
    verify_reference_release_bundle(&out_dir, &pubkey);
    for (name, first) in first_stable_artifacts {
        assert_eq!(
            first,
            fs::read(out_dir.join(name)).expect("read replayed stable release artifact"),
            "controlled release inputs must reproduce {name} byte-for-byte"
        );
    }
    assert_eq!(
        first_bundle_projection,
        reproducible_bundle_projection(&bundle),
        "bundle manifest must reproduce after excluding documented time-derived evidence"
    );
    retain_release_fixture("reproducible", &out_dir);
}

#[test]
fn contract_release_verification_rejects_every_packaged_evidence_tamper_class() {
    let (workspace, pubkey, release) = prepared_reference_release();

    with_tampered_release(workspace.path(), &release, &pubkey, "source-graph", |copied| {
        let schema = copied.join("counter.schema.json");
        rewrite_json(&schema, |value| {
            value["source_graph"]["files"][0]["path"] = serde_json::json!("tampered.clear");
        });
        update_bundle_artifact_digest(
            &copied.join("counter.contract-release-bundle.json"),
            "contract_state_schema",
            &schema,
        );
    });
    with_tampered_release(workspace.path(), &release, &pubkey, "target-profile", |copied| {
        let artifact = copied.join("counter.evm.json");
        rewrite_json(&artifact, |value| {
            value["target"]["profile"] = serde_json::json!("clg.evm-compatible.tampered");
        });
        update_bundle_artifact_digest(
            &copied.join("counter.contract-release-bundle.json"),
            "evm_artifact",
            &artifact,
        );
    });
    with_tampered_release(workspace.path(), &release, &pubkey, "wire-abi", |copied| {
        let wire = copied.join("counter.evm-wire-abi.json");
        rewrite_json(&wire, |value| {
            value["wire_abi"]["digest"] = serde_json::json!("sha256:tampered");
        });
        update_bundle_artifact_digest(
            &copied.join("counter.contract-release-bundle.json"),
            "evm_wire_abi",
            &wire,
        );
    });
    with_tampered_release(workspace.path(), &release, &pubkey, "proof", |copied| {
        let proof = copied.join("counter.proof.json");
        rewrite_json(&proof, |value| {
            value["compiler_mode"] = serde_json::json!("tampered");
        });
        update_bundle_artifact_digest(
            &copied.join("counter.contract-release-bundle.json"),
            "proof_artifact",
            &proof,
        );
    });
    with_tampered_release(
        workspace.path(),
        &release,
        &pubkey,
        "signature-payload",
        |copied| {
            let signature = copied.join("counter.sig.json");
            rewrite_json(&signature, |value| {
                value["payload"]["compiler"]["version"] = serde_json::json!("tampered");
            });
            update_bundle_artifact_digest(
                &copied.join("counter.contract-release-bundle.json"),
                "signature",
                &signature,
            );
        },
    );
    with_tampered_release(workspace.path(), &release, &pubkey, "assurance", |copied| {
        let assurance = copied.join("counter.assurance.json");
        rewrite_json(&assurance, |value| {
            value["payload"]["build"]["compiler_mode"] = serde_json::json!("tampered");
        });
        update_bundle_artifact_digest(
            &copied.join("counter.contract-release-bundle.json"),
            "signed_assurance_manifest",
            &assurance,
        );
    });
    with_tampered_release(workspace.path(), &release, &pubkey, "campaign-trace", |copied| {
        fs::write(
            copied
                .join("counter.campaign-traces")
                .join("case-0000.trace.json"),
            br#"{"status":"tampered"}"#,
        )
        .expect("tamper campaign trace");
    });
    with_tampered_release(workspace.path(), &release, &pubkey, "campaign-input", |copied| {
        fs::write(
            copied
                .join("counter.campaign-traces")
                .join("case-0000.args.json"),
            br#"[999]"#,
        )
        .expect("tamper campaign input");
    });
    with_tampered_release(workspace.path(), &release, &pubkey, "campaign-replay", |copied| {
        let report = copied.join("counter.campaign.json");
        rewrite_json(&report, |value| {
            value["cases"][0]["replay"]["argv"]
                .as_array_mut()
                .expect("replay argv")
                .push(serde_json::json!("--tampered"));
        });
        update_bundle_artifact_digest(
            &copied.join("counter.contract-release-bundle.json"),
            "contract_test_report",
            &report,
        );
    });
    with_tampered_release(workspace.path(), &release, &pubkey, "bundle-path", |copied| {
        rewrite_json(
            &copied.join("counter.contract-release-bundle.json"),
            |value| {
                value["artifacts"]["evm_artifact"]["path"] =
                    serde_json::json!("../counter.evm.json");
            },
        );
    });
    retain_release_fixture("tamper-baseline", &release);
}

#[test]
fn independent_verifier_clean_directory_rejects_tampering_and_never_uses_a_network() {
    let (workspace, pubkey, release) = prepared_reference_release();
    let independent = workspace.path().join("independent-verifier");
    let evidence_root = independent.join("evidence");
    let trust_root = independent.join("trust");
    let verifier_root = independent.join("bin");
    fs::create_dir_all(&trust_root).expect("create independent trust directory");
    fs::create_dir_all(&verifier_root).expect("create independent verifier directory");
    copy_release_tree(&release, &evidence_root);
    let independent_pubkey = trust_root.join("release-public-key.json");
    fs::copy(&pubkey, &independent_pubkey).expect("copy explicit public key");
    let verifier = verifier_root.join(if cfg!(windows) {
        "clg-contract-verifier.exe"
    } else {
        "clg-contract-verifier"
    });
    fs::copy(assert_cmd::cargo::cargo_bin("clg-contract-verifier"), &verifier)
        .expect("copy independent verifier executable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&verifier)
            .expect("read verifier permissions")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&verifier, permissions).expect("set verifier executable bit");
    }
    fs::remove_file(workspace.path().join("counter.clear"))
        .expect("remove source outside independent directory");

    let bundle = evidence_root.join("counter.contract-release-bundle.json");
    let happy = run_independent_verifier(&verifier, &independent, &bundle, &independent_pubkey, false);
    assert!(happy.status.success(), "valid copied evidence must verify");
    let result: Value = serde_json::from_slice(&happy.stdout).expect("parse verifier result");
    assert_eq!(result["format"], "clg.contract-release-verifier-result.v1");
    assert_eq!(result["status"], "verified");
    assert!(
        !String::from_utf8_lossy(&happy.stdout).contains(workspace.path().to_string_lossy().as_ref()),
        "result must not expose the original workspace"
    );

    let proxy = std::net::TcpListener::bind("127.0.0.1:0").expect("bind offline proxy trap");
    proxy.set_nonblocking(true).expect("make proxy trap nonblocking");
    let proxy_url = format!("http://{}", proxy.local_addr().expect("proxy address"));
    let offline = run_independent_verifier_with_proxy_trap(
        &verifier,
        &independent,
        &bundle,
        &independent_pubkey,
        &proxy_url,
    );
    assert!(offline.status.success(), "verification must remain offline");
    assert!(
        matches!(proxy.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "verifier must not contact an ambient proxy"
    );

    let case_root = independent.join("tamper-cases");
    let reject = |name: &str, tamper: &dyn Fn(&std::path::Path)| {
        let copied = case_root.join(name);
        copy_release_tree(&evidence_root, &copied);
        tamper(&copied);
        let output = run_independent_verifier(
            &verifier,
            &independent,
            &copied.join("counter.contract-release-bundle.json"),
            &independent_pubkey,
            true,
        );
        assert!(!output.status.success(), "{name} must be rejected");
        let diagnostic: Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("{name} must emit JSON diagnostics"));
        assert_eq!(diagnostic["errors"][0]["code"], "C140", "{name} code");
    };
    reject("source-graph", &|copied| {
        let schema = copied.join("counter.schema.json");
        rewrite_json(&schema, |value| {
            value["source_graph"]["files"][0]["path"] = serde_json::json!("tampered.clear");
        });
        update_bundle_artifact_digest(
            &copied.join("counter.contract-release-bundle.json"),
            "contract_state_schema",
            &schema,
        );
    });
    reject("target-profile", &|copied| {
        let artifact = copied.join("counter.evm.json");
        rewrite_json(&artifact, |value| {
            value["target"]["profile"] = serde_json::json!("clg.evm-compatible.tampered");
        });
        update_bundle_artifact_digest(
            &copied.join("counter.contract-release-bundle.json"),
            "evm_artifact",
            &artifact,
        );
    });
    reject("wire-abi", &|copied| {
        let wire = copied.join("counter.evm-wire-abi.json");
        rewrite_json(&wire, |value| {
            value["wire_abi"]["digest"] = serde_json::json!("sha256:tampered");
        });
        update_bundle_artifact_digest(
            &copied.join("counter.contract-release-bundle.json"),
            "evm_wire_abi",
            &wire,
        );
    });
    reject("proof", &|copied| {
        let proof = copied.join("counter.proof.json");
        rewrite_json(&proof, |value| {
            value["compiler_mode"] = serde_json::json!("tampered");
        });
        update_bundle_artifact_digest(
            &copied.join("counter.contract-release-bundle.json"),
            "proof_artifact",
            &proof,
        );
    });
    reject("signature", &|copied| {
        let signature = copied.join("counter.sig.json");
        rewrite_json(&signature, |value| {
            value["payload"]["compiler"]["version"] = serde_json::json!("tampered");
        });
        update_bundle_artifact_digest(
            &copied.join("counter.contract-release-bundle.json"),
            "signature",
            &signature,
        );
    });
    reject("assurance", &|copied| {
        let assurance = copied.join("counter.assurance.json");
        rewrite_json(&assurance, |value| {
            value["payload"]["build"]["compiler_mode"] = serde_json::json!("tampered");
        });
        update_bundle_artifact_digest(
            &copied.join("counter.contract-release-bundle.json"),
            "signed_assurance_manifest",
            &assurance,
        );
    });
    reject("campaign-trace", &|copied| {
        fs::write(
            copied
                .join("counter.campaign-traces")
                .join("case-0000.trace.json"),
            br#"{"status":"tampered"}"#,
        )
        .expect("tamper campaign trace");
    });
    reject("campaign-input", &|copied| {
        fs::write(
            copied
                .join("counter.campaign-traces")
                .join("case-0000.args.json"),
            br#"[999]"#,
        )
        .expect("tamper campaign input");
    });
    reject("campaign-replay", &|copied| {
        let report = copied.join("counter.campaign.json");
        rewrite_json(&report, |value| {
            value["cases"][0]["replay"]["argv"]
                .as_array_mut()
                .expect("replay argv")
                .push(serde_json::json!("--tampered"));
        });
        update_bundle_artifact_digest(
            &copied.join("counter.contract-release-bundle.json"),
            "contract_test_report",
            &report,
        );
    });
    reject("unsafe-path", &|copied| {
        rewrite_json(
            &copied.join("counter.contract-release-bundle.json"),
            |value| value["artifacts"]["evm_artifact"]["path"] = serde_json::json!("../counter.evm.json"),
        );
    });
    reject("missing-evidence", &|copied| {
        fs::remove_file(copied.join("counter.proof.json")).expect("remove proof evidence");
    });
    reject("unknown-schema", &|copied| {
        rewrite_json(
            &copied.join("counter.contract-release-bundle.json"),
            |value| value["schema_version"] = serde_json::json!(2),
        );
    });
    reject("unknown-profile", &|copied| {
        rewrite_json(
            &copied.join("counter.contract-release-bundle.json"),
            |value| value["target_profile"] = serde_json::json!("clg.evm.unknown.v1"),
        );
    });
    reject("incompatible-verifier", &|copied| {
        rewrite_json(
            &copied.join("counter.contract-release-bundle.json"),
            |value| value["minimum_verifier_version"] = serde_json::json!("0.2.0"),
        );
    });
    reject("malformed-minimum-verifier", &|copied| {
        rewrite_json(
            &copied.join("counter.contract-release-bundle.json"),
            |value| value["minimum_verifier_version"] = serde_json::json!("not-a-version"),
        );
    });
    let wrong_pubkey = trust_root.join("wrong-public-key.json");
    fs::write(
        &wrong_pubkey,
        r#"{"scheme":"ed25519","public_key":"1111111111111111111111111111111111111111111111111111111111111111"}"#,
    )
    .expect("write wrong public key");
    let wrong_key = run_independent_verifier(&verifier, &independent, &bundle, &wrong_pubkey, true);
    assert!(!wrong_key.status.success(), "wrong key must be rejected");
    let diagnostic: Value = serde_json::from_slice(&wrong_key.stdout).expect("parse wrong-key diagnostic");
    assert_eq!(diagnostic["errors"][0]["code"], "C140");
}

#[test]
fn contract_release_fixture_packages_supported_migration_and_rejects_reentrancy_candidate() {
    let dir = tempdir().expect("tempdir");
    let v1_source = dir.path().join("counter-v1.clear");
    let v1_plan = dir.path().join("counter-v1.campaign.json");
    let v1_release = dir.path().join("counter-v1-release");
    write_reference_counter_source_and_plan(&v1_source, &v1_plan);
    write_minimal_strict_contract_inputs(&dir);
    let (key, pubkey) = write_release_key_material(&dir);
    let solver = write_verified_unsat_solver(&dir);
    run_reference_contract_release(&v1_source, &v1_plan, &key, &pubkey, &v1_release, &solver);
    verify_contract_release_bundle(&v1_release, "counter-v1", &pubkey);

    let prior_schema = v1_release.join("counter-v1.schema.json");
    let prior: Value = serde_json::from_slice(&fs::read(&prior_schema).expect("read v1 schema"))
        .expect("parse v1 schema");
    let prior_digest = prior["schema"]["digest"]
        .as_str()
        .expect("v1 schema digest");
    let v2_source = dir.path().join("counter-v2.clear");
    let v2_plan = dir.path().join("counter-v2.campaign.json");
    let v2_release = dir.path().join("counter-v2-release");
    fs::write(
        &v2_source,
        format!(
            r#"
                contract Counter version 2 {{
                    state {{ total: U64; revision: U64; }}
                    init(initial: U64) {{
                        state.total = initial;
                        state.revision = U64(0);
                        0
                    }}
                    migrate from schema "{prior_digest}" {{
                        state.revision = U64(0);
                        0
                    }}
                    mut function set_total(value: U64) -> U64 {{ state.total = value; state.total }}
                }}
                function main() -> Int {{ 0 }}
            "#
        ),
    )
    .expect("write v2 migration contract");
    fs::write(
        &v2_plan,
        r#"{
            "format":"clg.contract-test-plan.v1",
            "schema_version":1,
            "seed":42,
            "cases":1,
            "function":"set_total",
            "state":{"total":0,"revision":0},
            "caller":"clg:test:campaign",
            "value":0,
            "block_number":7,
            "timestamp":8,
            "gas_limit":100,
            "memory_limit":1048576,
            "generators":[{"type":"u64","min":1,"max":9}],
            "properties":[
                {"kind":"result_equals_arg","arg":0},
                {"kind":"state_field_equals_arg","field":"total","arg":0}
            ]
        }"#,
    )
    .expect("write v2 campaign plan");
    run_contract_release(
        &v2_source,
        &v2_plan,
        &key,
        &pubkey,
        &v2_release,
        &solver,
        Some(&prior_schema),
    );
    verify_contract_release_bundle(&v2_release, "counter-v2", &pubkey);

    let v2_schema: Value = serde_json::from_slice(
        &fs::read(v2_release.join("counter-v2.schema.json")).expect("read v2 schema"),
    )
    .expect("parse v2 schema");
    assert_eq!(v2_schema["contract"]["version"], 2);
    assert_eq!(v2_schema["fields"][1]["name"], "revision");

    let reentrant_source = dir.path().join("counter-reentrant.clear");
    let rejected_release = dir.path().join("counter-reentrant-release");
    fs::write(
        &reentrant_source,
        r#"
            interface Receiver { io function receive(amount: U64) -> Bool; }
            contract Counter version 3 {
                state { total: U64; revision: U64; }
                mut function notify(amount: U64) -> U64 {
                    state.total = amount;
                    external_call Receiver.receive(amount);
                    state.revision = U64(1);
                    amount
                }
            }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write CEI-negative candidate");
    let output = Command::cargo_bin("clg")
        .expect("clg binary")
        .env("CLG_SOLVER_BIN", &solver)
        .args(["--json-errors", "contract", "release"])
        .arg(&reentrant_source)
        .args(["--plan"])
        .arg(&v2_plan)
        .args(["--key"])
        .arg(&key)
        .args(["--pubkey"])
        .arg(&pubkey)
        .args(["--key-id", "reference-release-key", "--out-dir"])
        .arg(&rejected_release)
        .args(["--prior-state-schema"])
        .arg(v2_release.join("counter-v2.schema.json"))
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let diagnostic: Value = serde_json::from_slice(&output).expect("parse CEI diagnostic");
    assert_eq!(diagnostic["errors"][0]["code"], "T832");
    assert!(
        !rejected_release.join("counter-reentrant.contract-release-bundle.json").exists(),
        "a CEI-violating candidate must never produce a packaged release bundle"
    );
    retain_release_fixture("migration-v1", &v1_release);
    retain_release_fixture("migration-v2", &v2_release);
}

use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use sha3::Keccak256;
use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn write_reference_counter_source_and_plan(source: &Path, plan: &Path) {
    fs::write(
        source,
        r#"
            contract Counter version 1 {
                state { total: U64; }
                init(initial: U64) { state.total = initial; 0 }
                mut function set_total(value: U64) -> U64 { state.total = value; state.total }
            }
            function main() -> Int { 0 }
        "#,
    )
    .expect("write reference contract source");
    fs::write(
        plan,
        r#"{
            "format":"clg.contract-test-plan.v1",
            "schema_version":1,
            "seed":42,
            "cases":1,
            "function":"set_total",
            "state":{"total":0},
            "caller":"clg:test:campaign",
            "value":0,
            "block_number":7,
            "timestamp":8,
            "gas_limit":100,
            "memory_limit":1048576,
            "generators":[{"type":"u64","min":1,"max":9}],
            "properties":[
                {"kind":"result_equals_arg","arg":0},
                {"kind":"state_field_equals_arg","field":"total","arg":0}
            ]
        }"#,
    )
    .expect("write reference campaign plan");
}

fn run_reference_contract_release(
    source: &Path,
    plan: &Path,
    key: &Path,
    pubkey: &Path,
    out_dir: &Path,
    solver: &Path,
) {
    run_contract_release(source, plan, key, pubkey, out_dir, solver, None);
}

fn run_contract_release(
    source: &Path,
    plan: &Path,
    key: &Path,
    pubkey: &Path,
    out_dir: &Path,
    solver: &Path,
    prior_state_schema: Option<&Path>,
) {
    let mut command = Command::cargo_bin("clg").expect("clg binary");
    command
        .env("CLG_SOLVER_BIN", solver)
        .env(
            "CLG_PROOF_MATRIX_PATH",
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../docs/proofs/proof-coverage-matrix.json"),
        );
    command
        .args(["contract", "release"])
        .arg(source)
        .args(["--plan"])
        .arg(plan)
        .args(["--key"])
        .arg(key)
        .args(["--pubkey"])
        .arg(pubkey)
        .args(["--key-id", "reference-release-key", "--out-dir"])
        .arg(out_dir);
    if let Some(prior_state_schema) = prior_state_schema {
        command.args(["--prior-state-schema"]).arg(prior_state_schema);
    }
    command.assert().success();
}

fn verify_reference_release_bundle(out_dir: &Path, pubkey: &Path) {
    verify_contract_release_bundle(out_dir, "counter", pubkey);
}

fn verify_contract_release_bundle(out_dir: &Path, stem: &str, pubkey: &Path) {
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["contract", "verify-release", "--bundle"])
        .arg(out_dir.join(format!("{stem}.contract-release-bundle.json")))
        .args(["--pubkey"])
        .arg(pubkey)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"verified\""));
}

fn reproducible_bundle_projection(bundle: &Path) -> Value {
    let mut bundle: Value = serde_json::from_slice(&fs::read(bundle).expect("read release bundle"))
        .expect("parse release bundle");
    let artifacts = bundle["artifacts"]
        .as_object_mut()
        .expect("release bundle artifacts");
    artifacts.remove("signature");
    artifacts.remove("signed_assurance_manifest");
    bundle
}

fn prepared_reference_release() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("counter.clear");
    let plan = dir.path().join("campaign.json");
    let out_dir = dir.path().join("release");
    write_reference_counter_source_and_plan(&source, &plan);
    write_minimal_strict_contract_inputs(&dir);
    let (key, pubkey) = write_release_key_material(&dir);
    let solver = write_verified_unsat_solver(&dir);
    run_reference_contract_release(&source, &plan, &key, &pubkey, &out_dir, &solver);
    (dir, pubkey, out_dir)
}

fn run_independent_verifier(
    verifier: &Path,
    working_directory: &Path,
    bundle: &Path,
    pubkey: &Path,
    json_errors: bool,
) -> std::process::Output {
    let mut command = Command::new(verifier);
    command
        .current_dir(working_directory)
        .args(["verify-release", "--bundle"])
        .arg(bundle)
        .args(["--pubkey"])
        .arg(pubkey);
    if json_errors {
        command.arg("--json-errors");
    }
    command.output().expect("run independent verifier")
}

fn run_independent_verifier_with_proxy_trap(
    verifier: &Path,
    working_directory: &Path,
    bundle: &Path,
    pubkey: &Path,
    proxy_url: &str,
) -> std::process::Output {
    let mut command = Command::new(verifier);
    command
        .current_dir(working_directory)
        .env("HTTP_PROXY", proxy_url)
        .env("HTTPS_PROXY", proxy_url)
        .env("ALL_PROXY", proxy_url)
        .env("NO_PROXY", "")
        .args(["verify-release", "--bundle"])
        .arg(bundle)
        .args(["--pubkey"])
        .arg(pubkey);
    command.output().expect("run independent verifier with proxy trap")
}

fn copy_release_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create copied release directory");
    for entry in fs::read_dir(source).expect("list release directory") {
        let entry = entry.expect("read release entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type().expect("release entry type").is_dir() {
            copy_release_tree(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).expect("copy release evidence file");
        }
    }
}

fn retain_release_fixture(name: &str, release: &Path) {
    let Some(root) = env::var_os("CLG_CONTRACT_RELEASE_EVIDENCE_DIR") else {
        return;
    };
    let destination = PathBuf::from(root).join(name);
    assert!(
        !destination.exists(),
        "refusing to overwrite retained contract release fixture `{}`",
        destination.display()
    );
    copy_release_tree(release, &destination);
}

fn rewrite_json(path: &Path, change: impl FnOnce(&mut Value)) {
    let mut value: Value = serde_json::from_slice(&fs::read(path).expect("read JSON evidence"))
        .expect("parse JSON evidence");
    change(&mut value);
    fs::write(path, serde_json::to_vec(&value).expect("serialize JSON evidence"))
        .expect("write JSON evidence");
}

fn update_bundle_artifact_digest(bundle: &Path, artifact_name: &str, artifact_path: &Path) {
    rewrite_json(bundle, |value| {
        value["artifacts"][artifact_name]["sha256"] = serde_json::json!(format!(
            "sha256:{}",
            hex::encode(Sha256::digest(
                fs::read(artifact_path).expect("read changed release artifact")
            ))
        ));
    });
}

fn assert_release_verification_rejects(bundle: &Path, pubkey: &Path) {
    let output = Command::cargo_bin("clg")
        .expect("clg binary")
        .args(["--json-errors", "contract", "verify-release", "--bundle"])
        .arg(bundle)
        .args(["--pubkey"])
        .arg(pubkey)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let diagnostic: Value = serde_json::from_slice(&output).expect("parse release diagnostic");
    assert_eq!(
        diagnostic["errors"][0]["code"], "C140",
        "tampered release evidence must have a stable release-verification diagnostic"
    );
}

fn with_tampered_release(
    workspace: &Path,
    release: &Path,
    pubkey: &Path,
    case_name: &str,
    tamper: impl FnOnce(&Path),
) {
    let copied_release = workspace.join(case_name);
    copy_release_tree(release, &copied_release);
    tamper(&copied_release);
    assert_release_verification_rejects(
        &copied_release.join("counter.contract-release-bundle.json"),
        pubkey,
    );
}

fn write_release_key_material(dir: &tempfile::TempDir) -> (PathBuf, PathBuf) {
    let signing = SigningKey::from_bytes(&[11_u8; 32]);
    let key = dir.path().join("release-key.json");
    let pubkey = dir.path().join("release-pubkey.json");
    fs::write(
        &key,
        serde_json::to_vec(&serde_json::json!({
            "scheme": "ed25519",
            "private_key": hex::encode(signing.to_bytes()),
            "public_key": hex::encode(signing.verifying_key().to_bytes()),
        }))
        .expect("serialize release signing key"),
    )
    .expect("write release signing key");
    fs::write(
        &pubkey,
        serde_json::to_vec(&serde_json::json!({
            "scheme": "ed25519",
            "public_key": hex::encode(signing.verifying_key().to_bytes()),
        }))
        .expect("serialize release public key"),
    )
    .expect("write release public key");
    (key, pubkey)
}

fn write_minimal_strict_contract_inputs(dir: &tempfile::TempDir) {
    for (name, contents) in [
        ("clg.lock.json", r#"{"schema_version":0,"dependencies":[]}"#),
        (
            "clg.trust-policy.json",
            r#"{"schema_version":0,"trusted_signers":[{"key_id":"std-core-release-ed25519-2026q1","scheme":"ed25519","public_key":"hex:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","not_before":"2026-01-01T00:00:00Z","not_after":"2027-01-01T00:00:00Z"}],"revoked_key_ids":[]}"#,
        ),
        (
            "clg.host-profile.json",
            r#"{"schema_version":0,"profile":"contract_static","capabilities":[]}"#,
        ),
        ("clg.package-metadata.json", r#"{"schema_version":0,"packages":[]}"#),
        ("clg.package-abi.json", r#"{"schema_version":0,"contracts":[]}"#),
    ] {
        fs::write(dir.path().join(name), contents).expect("write strict contract input");
    }
}

fn write_verified_unsat_solver(dir: &tempfile::TempDir) -> PathBuf {
    let solver = if cfg!(windows) {
        dir.path().join("reference-z3.exe")
    } else {
        dir.path().join("reference-z3")
    };
    let source = solver.with_extension("rs");
    fs::write(
        &source,
        r#"
            fn main() {
                if std::env::args().any(|arg| arg == "--version" || arg == "-version") {
                    println!("Z3 version 4.16.0 - reference fixture");
                } else {
                    println!("unsat");
                }
            }
        "#,
    )
    .expect("write reference solver source");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let output = Command::new(rustc)
        .arg(&source)
        .arg("-O")
        .arg("-o")
        .arg(&solver)
        .output()
        .expect("compile reference solver");
    assert!(
        output.status.success(),
        "reference solver compile failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let checksum = format!("sha256:{}", hex::encode(Sha256::digest(fs::read(&solver).expect("read solver"))));
    let signing = SigningKey::from_bytes(&[7_u8; 32]);
    fs::write(
        solver.with_file_name(format!(
            "{}.sha256",
            solver.file_name().expect("solver file name").to_string_lossy()
        )),
        format!("{checksum}\n"),
    )
    .expect("write solver checksum");
    fs::write(
        solver.with_file_name(format!(
            "{}.sig",
            solver.file_name().expect("solver file name").to_string_lossy()
        )),
        serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "key_id": "z3-vendor-k7-2026q2",
            "scheme": "ed25519",
            "signed_payload": checksum,
            "signature": hex::encode(signing.sign(checksum.as_bytes()).to_bytes()),
        }))
        .expect("serialize solver signature"),
    )
    .expect("write solver signature");
    solver
}

fn mock_target_rpc(
    contract_address: Option<&'static str>,
) -> (String, thread::JoinHandle<Vec<Value>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind target RPC");
    let address = listener.local_addr().expect("target RPC address");
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().expect("accept target RPC request");
            let request = read_target_rpc_request(&mut stream);
            let response = match request["method"].as_str() {
                Some("eth_chainId") => serde_json::json!("0xaa36a7"),
                Some("eth_sendRawTransaction") => {
                    let raw = request["params"][0]
                        .as_str()
                        .and_then(|value| value.strip_prefix("0x"))
                        .expect("raw signed transaction");
                    serde_json::json!(format!(
                        "0x{}",
                        hex::encode(Keccak256::digest(
                            hex::decode(raw).expect("decode raw transaction")
                        ))
                    ))
                }
                Some("eth_getTransactionReceipt") => serde_json::json!({
                    "blockHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "blockNumber": "0x2",
                    "contractAddress": contract_address,
                    "status": "0x1",
                    "transactionHash": request["params"][0],
                }),
                other => panic!("unexpected target RPC method {other:?}"),
            };
            let body = serde_json::to_vec(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": response,
            }))
            .expect("serialize target RPC response");
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("write target RPC headers");
            stream.write_all(&body).expect("write target RPC body");
            requests.push(request);
        }
        requests
    });
    (format!("http://{address}"), server)
}

fn read_target_rpc_request(stream: &mut std::net::TcpStream) -> Value {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut buffer).expect("read target RPC request");
        request.extend_from_slice(&buffer[..read]);
    }
    let header_end = request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("target RPC header end")
        + 4;
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length: "))
        .expect("target RPC content length")
        .parse::<usize>()
        .expect("numeric target RPC content length");
    while request.len() - header_end < content_length {
        let read = stream.read(&mut buffer).expect("read target RPC body");
        request.extend_from_slice(&buffer[..read]);
    }
    serde_json::from_slice(&request[header_end..header_end + content_length])
        .expect("parse target RPC request")
}
