#[test]
fn simulate_contract_transition_is_deterministic_and_emits_a_trace() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("counter.clear");
    let state = dir.path().join("state.json");
    let args = dir.path().join("args.json");
    let first_state_out = dir.path().join("first-state.json");
    let first_trace_out = dir.path().join("first-trace.json");
    let second_state_out = dir.path().join("second-state.json");
    let second_trace_out = dir.path().join("second-trace.json");
    fs::write(
        &source,
        r#"
            contract Counter version 1 {
                state { total: U64; }
                event Changed { total: U64; }
                mut function set_total(total: U64) -> U64 {
                    state.total = total;
                    emit Changed(total);
                    total
                }
                pure function get_total() -> U64 { state.total }
            }
        "#,
    )
    .expect("write contract source");
    fs::write(&state, r#"{"total":2}"#).expect("write state");
    fs::write(&args, "[5]").expect("write arguments");

    for (state_out, trace_out) in [
        (&first_state_out, &first_trace_out),
        (&second_state_out, &second_trace_out),
    ] {
        Command::cargo_bin("clg")
            .expect("clg binary")
            .args([
                "simulate",
                source.to_str().expect("source path"),
                "--function",
                "set_total",
                "--state",
                state.to_str().expect("state path"),
                "--args",
                args.to_str().expect("args path"),
                "--state-out",
                state_out.to_str().expect("state output path"),
                "--trace-out",
                trace_out.to_str().expect("trace output path"),
                "--caller",
                "clg:test:alice",
                "--value",
                "7",
                "--block-number",
                "9",
                "--timestamp",
                "10",
                "--gas-limit",
                "100",
            ])
            .assert()
            .success();
    }

    assert_eq!(
        fs::read(&first_state_out).expect("read first state"),
        fs::read(&second_state_out).expect("read second state"),
        "simulation state output must be byte-stable"
    );
    assert_eq!(
        fs::read(&first_trace_out).expect("read first trace"),
        fs::read(&second_trace_out).expect("read second trace"),
        "simulation trace output must be byte-stable"
    );
    let state_out: Value = serde_json::from_slice(&fs::read(&first_state_out).expect("state"))
        .expect("parse state output");
    let trace: Value = serde_json::from_slice(&fs::read(&first_trace_out).expect("trace"))
        .expect("parse trace output");
    assert_eq!(state_out, serde_json::json!({ "total": 5 }));
    assert_eq!(trace["trace_format"], "clg.contract-simulation-trace.v1");
    assert_eq!(trace["schema_version"], 1);
    assert_eq!(trace["status"], "success");
    assert_eq!(trace["caller"], "clg:test:alice");
    assert_eq!(trace["value"], 7);
    assert_eq!(trace["block"], serde_json::json!({ "number": 9, "timestamp": 10 }));
    assert_eq!(trace["state_before"], serde_json::json!({ "total": 2 }));
    assert_eq!(trace["state_after"], serde_json::json!({ "total": 5 }));
    assert_eq!(trace["result"], 5);
    assert_eq!(trace["events"], serde_json::json!([{ "event": "Changed", "args": [5] }]));
    assert!(trace["fuel_used"].as_u64().expect("fuel used") <= 100);

    let query_state_out = dir.path().join("query-state.json");
    let query_trace_out = dir.path().join("query-trace.json");
    let query_args = dir.path().join("query-args.json");
    fs::write(&query_args, "[]").expect("write query arguments");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "simulate",
            source.to_str().expect("source path"),
            "--function",
            "get_total",
            "--state",
            state.to_str().expect("state path"),
            "--args",
            query_args.to_str().expect("query arguments path"),
            "--state-out",
            query_state_out.to_str().expect("query state output path"),
            "--trace-out",
            query_trace_out.to_str().expect("query trace output path"),
        ])
        .assert()
        .success();
    let query_trace: Value =
        serde_json::from_slice(&fs::read(&query_trace_out).expect("query trace"))
            .expect("parse query trace");
    assert_eq!(query_trace["result"], 2);
    assert_eq!(query_trace["events"], serde_json::json!([]));
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&query_state_out).expect("query state"))
            .expect("parse query state"),
        serde_json::json!({ "total": 2 })
    );
}

#[test]
fn simulate_rejects_transitions_that_exceed_the_fuel_limit() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("counter.clear");
    let state = dir.path().join("state.json");
    let args = dir.path().join("args.json");
    let state_out = dir.path().join("state-out.json");
    let trace_out = dir.path().join("trace-out.json");
    fs::write(
        &source,
        r#"
            contract Counter version 1 {
                state { total: U64; }
                mut function set_total(value: U64) -> U64 {
                    state.total = value;
                    state.total
                }
            }
        "#,
    )
    .expect("write contract source");
    fs::write(&state, r#"{"total":0}"#).expect("write state");
    fs::write(&args, "[1]").expect("write arguments");

    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "simulate",
            source.to_str().expect("source path"),
            "--function",
            "set_total",
            "--state",
            state.to_str().expect("state path"),
            "--args",
            args.to_str().expect("args path"),
            "--state-out",
            state_out.to_str().expect("state output path"),
            "--trace-out",
            trace_out.to_str().expect("trace output path"),
            "--gas-limit",
            "0",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("simulator fuel limit exceeded"));
    assert!(!state_out.exists());
    let trace: Value = serde_json::from_slice(&fs::read(&trace_out).expect("failure trace"))
        .expect("parse failure trace");
    assert_eq!(trace["status"], "failure");
    assert_eq!(trace["result"], Value::Null);
    assert!(trace["failure"]
        .as_str()
        .expect("failure message")
        .contains("simulator fuel limit exceeded"));
}

#[test]
fn simulate_failure_trace_and_json_diagnostic_share_a_source_location() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("guarded.clear");
    let state = dir.path().join("state.json");
    let args = dir.path().join("args.json");
    let trace_out = dir.path().join("trace-out.json");
    let source_text = r#"
        contract Counter version 1 {
            state { total: U64; }
            mut function set_total(value: U64) -> U64
                require { value > U64(0) }
            {
                state.total = value;
                state.total
            }
        }
    "#;
    fs::write(&source, source_text).expect("write contract source");
    fs::write(&state, r#"{"total":0}"#).expect("write state");
    fs::write(&args, "[0]").expect("write arguments");

    let output = Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "--non-interactive",
            "--json-errors",
            "--json-events",
            "simulate",
            source.to_str().expect("source path"),
            "--function",
            "set_total",
            "--state",
            state.to_str().expect("state path"),
            "--args",
            args.to_str().expect("args path"),
            "--state-out",
            dir.path().join("state-out.json").to_str().expect("state output path"),
            "--trace-out",
            trace_out.to_str().expect("trace output path"),
        ])
        .output()
        .expect("run simulator");
    assert!(!output.status.success());
    let diagnostic: Value = serde_json::from_slice(&output.stdout).expect("JSON diagnostic");
    let error = &diagnostic["errors"][0];
    assert_eq!(error["code"], "C142");
    assert_eq!(error["stage"], "simulate");
    assert_eq!(error["function"], "set_total");
    assert_eq!(error["file"], source.display().to_string());

    let trace: Value = serde_json::from_slice(&fs::read(&trace_out).expect("failure trace"))
        .expect("parse failure trace");
    assert_eq!(trace["status"], "failure");
    assert_eq!(trace["failure_location"]["function"], "set_total");
    assert_eq!(trace["failure_location"]["file"], source.display().to_string());
    assert_eq!(trace["failure_location"]["start"], error["start"]);
    assert_eq!(trace["failure_location"]["end"], error["end"]);
    assert_eq!(trace["stack"].as_array().map(Vec::len), Some(1));
    assert!(trace["stack"][0]["instruction"].as_u64().is_some());
    assert_eq!(trace["stack"][0]["function"], "set_total");
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"command\":\"simulate\""));
}

#[test]
fn simulate_rejects_json_inputs_over_the_memory_limit() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("counter.clear");
    let state = dir.path().join("state.json");
    let args = dir.path().join("args.json");
    fs::write(
        &source,
        "contract Counter version 1 { state { total: U64; } mut function set(value: U64) -> U64 { state.total = value; value } }",
    )
    .expect("write source");
    fs::write(&state, r#"{"total":0}"#).expect("write state");
    fs::write(&args, "[1]").expect("write args");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "simulate",
            source.to_str().expect("source"),
            "--function",
            "set",
            "--state",
            state.to_str().expect("state"),
            "--args",
            args.to_str().expect("args"),
            "--state-out",
            dir.path().join("out.json").to_str().expect("out"),
            "--trace-out",
            dir.path().join("trace.json").to_str().expect("trace"),
            "--memory-limit",
            "1",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("simulator state exceeds memory limit"));
}

#[test]
fn simulate_rejects_external_calls_without_committing_partial_state() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("vault.clear");
    let state = dir.path().join("state.json");
    let args = dir.path().join("args.json");
    let state_out = dir.path().join("state-out.json");
    let trace_out = dir.path().join("trace-out.json");
    fs::write(
        &source,
        r#"
            interface Receiver { io function receive(amount: U64) -> Bool; }
            contract Vault version 1 {
                state { total: U64; }
                event Paid { amount: U64; }
                mut function pay(amount: U64) -> U64 {
                    state.total = amount;
                    emit Paid(amount);
                    external_call Receiver.receive(amount);
                    amount
                }
            }
        "#,
    )
    .expect("write contract source");
    fs::write(&state, r#"{"total":0}"#).expect("write state");
    fs::write(&args, "[1]").expect("write arguments");

    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "simulate",
            source.to_str().expect("source path"),
            "--function",
            "pay",
            "--state",
            state.to_str().expect("state path"),
            "--args",
            args.to_str().expect("args path"),
            "--state-out",
            state_out.to_str().expect("state output path"),
            "--trace-out",
            trace_out.to_str().expect("trace output path"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "simulator does not support external calls",
        ));
    assert!(!state_out.exists());
    let trace: Value = serde_json::from_slice(&fs::read(&trace_out).expect("failure trace"))
        .expect("parse failure trace");
    assert_eq!(trace["status"], "failure");
    assert_eq!(trace["state_changes"], serde_json::json!([{ "field": "total", "value": 1 }]));
    assert_eq!(trace["events"], serde_json::json!([{ "event": "Paid", "args": [1] }]));
}

#[test]
fn simulate_init_requires_empty_state_and_produces_initialized_state() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("counter.clear");
    let empty_state = dir.path().join("empty-state.json");
    let args = dir.path().join("args.json");
    let initialized_state = dir.path().join("initialized-state.json");
    let init_trace = dir.path().join("init-trace.json");
    fs::write(
        &source,
        r#"
            contract Counter version 1 {
                state { total: U64; }
                invariant { state.total >= U64(0) }
                init(initial: U64) {
                    state.total = initial;
                    0
                }
            }
        "#,
    )
    .expect("write contract source");
    fs::write(&empty_state, "{}").expect("write empty state");
    fs::write(&args, "[5]").expect("write arguments");

    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "simulate",
            source.to_str().expect("source path"),
            "--init",
            "--state",
            empty_state.to_str().expect("state path"),
            "--args",
            args.to_str().expect("args path"),
            "--state-out",
            initialized_state.to_str().expect("state output path"),
            "--trace-out",
            init_trace.to_str().expect("trace output path"),
        ])
        .assert()
        .success();
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&initialized_state).expect("initialized state"))
            .expect("parse initialized state"),
        serde_json::json!({ "total": 5 })
    );
    let trace: Value = serde_json::from_slice(&fs::read(&init_trace).expect("init trace"))
        .expect("parse init trace");
    assert_eq!(trace["lifecycle"], "init");
    assert_eq!(trace["function"], "init");
    assert_eq!(trace["result"], Value::Null);
    assert_eq!(trace["status"], "success");

    let rejected_state_out = dir.path().join("rejected-state.json");
    let rejected_trace_out = dir.path().join("rejected-trace.json");
    Command::cargo_bin("clg")
        .expect("clg binary")
        .args([
            "simulate",
            source.to_str().expect("source path"),
            "--init",
            "--state",
            initialized_state.to_str().expect("initialized state path"),
            "--args",
            args.to_str().expect("args path"),
            "--state-out",
            rejected_state_out.to_str().expect("rejected state path"),
            "--trace-out",
            rejected_trace_out.to_str().expect("rejected trace path"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "simulator init may only execute against an empty state object",
        ));
    assert!(!rejected_state_out.exists());
    assert!(!rejected_trace_out.exists());
}
