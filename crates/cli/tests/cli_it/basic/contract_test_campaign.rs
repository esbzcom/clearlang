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
        .stdout(predicate::str::contains("\"status\": \"verified\""));

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
        .args(["target", "deploy", "--target-profile", "clg.evm-compatible.v1"])
        .args(["--rpc-url"])
        .arg(&deploy_endpoint)
        .args(["--chain-id", "1", "--sender", "0x1a642f0e3c3af545e7acbd38b07251b3990914f1", "--nonce", "0"])
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
        .args(["target", "invoke", "--target-profile", "clg.evm-compatible.v1"])
        .args(["--rpc-url"])
        .arg(&invoke_endpoint)
        .args(["--chain-id", "1", "--artifact"])
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
    command.env("CLG_SOLVER_BIN", solver);
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
        .stdout(predicate::str::contains("\"status\": \"verified\""));
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
                Some("eth_chainId") => serde_json::json!("0x1"),
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
