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

    Command::cargo_bin("clg")
        .expect("clg binary")
        .env("CLG_SOLVER_BIN", &solver)
        .args(["contract", "release"])
        .arg(&source)
        .args(["--plan"])
        .arg(&plan)
        .args(["--key"])
        .arg(&key)
        .args(["--pubkey"])
        .arg(&pubkey)
        .args(["--key-id", "reference-release-key", "--out-dir"])
        .arg(&out_dir)
        .assert()
        .success();

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
}
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use sha3::Keccak256;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

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
