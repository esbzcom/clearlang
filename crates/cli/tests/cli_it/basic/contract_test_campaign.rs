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
}
