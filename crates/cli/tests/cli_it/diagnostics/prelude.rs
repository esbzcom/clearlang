use super::*;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use wasmparser::{Operator, Parser, Payload, TypeRef};

fn assert_single_json_error(v: &Value, code: &str, stage: &str) {
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1, "expected exactly one error, got {errs:?}");
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some(code));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some(stage));
}

fn assert_json_error_codes(v: &Value, stage: &str, expected_codes: &[&str]) -> Vec<Value> {
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    let actual_codes = errs
        .iter()
        .map(|err| err.get("code").and_then(|s| s.as_str()).unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(actual_codes, expected_codes);
    for err in errs {
        assert_eq!(err.get("stage").and_then(|s| s.as_str()), Some(stage));
    }
    errs.to_vec()
}

fn wasm_import_func_index(wasm: &[u8], module: &str, name: &str) -> Option<u32> {
    let mut func_index = 0u32;
    for payload in Parser::new(0).parse_all(wasm) {
        if let Payload::ImportSection(reader) = payload.expect("payload") {
            for item in reader {
                let import = item.expect("import");
                if let TypeRef::Func(_) = import.ty {
                    if import.module == module && import.name == name {
                        return Some(func_index);
                    }
                    func_index += 1;
                }
            }
        }
    }
    None
}

fn wasm_calls_function_index(wasm: &[u8], target_index: u32) -> bool {
    for payload in Parser::new(0).parse_all(wasm) {
        if let Payload::CodeSectionEntry(body) = payload.expect("payload") {
            let mut ops = body.get_operators_reader().expect("operators");
            while !ops.eof() {
                if let Operator::Call { function_index } = ops.read().expect("operator") {
                    if function_index == target_index {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn write_minimal_strict_lockfile(root: &Path) {
    fs::write(
        root.join("clg.lock.json"),
        r#"{"schema_version":0,"dependencies":[]}"#,
    )
    .expect("write strict lockfile");
}

fn write_minimal_strict_trust_policy(root: &Path) {
    fs::write(
        root.join("clg.trust-policy.json"),
        r#"{
  "schema_version": 0,
  "trusted_signers": [
    {
      "key_id": "std-core-release-ed25519-2026q1",
      "scheme": "ed25519",
      "public_key": "hex:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    }
  ],
  "revoked_key_ids": []
}"#,
    )
    .expect("write strict trust policy");
}

fn write_minimal_strict_host_profile(root: &Path) {
    fs::write(
        root.join("clg.host-profile.json"),
        r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": []
}"#,
    )
    .expect("write strict host profile");
}

fn write_strict_host_profile(root: &Path, profile: &str, capabilities: &[&str]) {
    let value = serde_json::json!({
        "schema_version": 0,
        "profile": profile,
        "capabilities": capabilities,
    });
    fs::write(
        root.join("clg.host-profile.json"),
        serde_json::to_vec_pretty(&value).expect("serialize strict host profile"),
    )
    .expect("write strict host profile");
}

fn write_minimal_strict_package_metadata(root: &Path) {
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 0,
  "packages": []
}"#,
    )
    .expect("write strict package metadata");
}

fn write_minimal_strict_package_abi(root: &Path) {
    fs::write(
        root.join("clg.package-abi.json"),
        r#"{
  "schema_version": 0,
  "contracts": []
}"#,
    )
    .expect("write strict package ABI");
}

fn write_minimal_strict_preflight_files(root: &Path) {
    write_minimal_strict_lockfile(root);
    write_minimal_strict_trust_policy(root);
    write_minimal_strict_host_profile(root);
    write_minimal_strict_package_metadata(root);
    write_minimal_strict_package_abi(root);
}

fn write_signed_strict_dependency_fixture(root: &Path, imports_json: &str) {
    let signing = SigningKey::from_bytes(&[7u8; 32]);
    let public_key_hex = hex::encode(signing.verifying_key().to_bytes());

    fs::write(
        root.join("clg.lock.json"),
        r#"{
  "schema_version": 0,
  "dependencies": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }
  ]
}"#,
    )
    .expect("write strict lockfile");
    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 0,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0"
    }
  ]
}"#,
    )
    .expect("write strict package metadata");
    fs::write(
        root.join("clg.package-abi.json"),
        format!(
            r#"{{
  "schema_version": 0,
  "contracts": [
    {{
      "abi_id": "abi:std::core:1.0.0",
      "package": "std::core",
      "version": "1.0.0",
      "imports": {imports_json}
    }}
  ]
}}"#
        ),
    )
    .expect("write strict package abi");
    fs::write(
        root.join("clg.trust-policy.json"),
        format!(
            r#"{{
  "schema_version": 0,
  "trusted_signers": [
    {{
      "key_id": "k1",
      "scheme": "ed25519",
      "public_key": "hex:{public_key_hex}",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    }}
  ],
  "revoked_key_ids": []
}}"#
        ),
    )
    .expect("write strict trust policy");
    fs::write(
        root.join("clg.host-profile.json"),
        r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": []
}"#,
    )
    .expect("write strict host profile");

    let signed_at = "2026-06-01T00:00:00Z";
    let payload = format!(
        "clg-package-signature-v0\n{}\n{}\n{}\n{}\n",
        "std::core",
        "1.0.0",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        signed_at
    );
    let signature = signing.sign(payload.as_bytes());
    fs::write(
        root.join("clg.package-signatures.json"),
        format!(
            r#"{{
  "schema_version": 0,
  "signatures": [
    {{
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "key_id": "k1",
      "signed_at": "{signed_at}",
      "signature_format": "ed25519",
      "signature": "{}"
    }}
  ]
}}"#,
            hex::encode(signature.to_bytes())
        ),
    )
    .expect("write strict package signatures");
}

fn run_strict_build_json_failure(root: &Path, file: &Path) -> Value {
    let out = root.join("out.wasm");
    let vcs = root.join("out.vc.json");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--std-core-link-mode", "precompiled"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    serde_json::from_slice(&output).expect("json")
}

fn run_strict_build_success(root: &Path, file: &Path) {
    let out = root.join("out.wasm");
    let vcs = root.join("out.vc.json");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["build"])
        .arg(file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--std-core-link-mode", "precompiled"]);
    cmd.assert().success();
}

fn write_solver_integrity_sidecars(solver: &Path) {
    let solver_bytes = fs::read(solver).expect("read solver bytes");
    let checksum = format!("sha256:{}", hex::encode(Sha256::digest(&solver_bytes)));
    let checksum_path = solver.with_file_name(format!(
        "{}.sha256",
        solver
            .file_name()
            .expect("solver filename")
            .to_string_lossy()
    ));
    fs::write(&checksum_path, format!("{checksum}\n")).expect("write checksum sidecar");

    let signing = SigningKey::from_bytes(&[7u8; 32]);
    let signature = hex::encode(signing.sign(checksum.as_bytes()).to_bytes());
    let signature_path = solver.with_file_name(format!(
        "{}.sig",
        solver
            .file_name()
            .expect("solver filename")
            .to_string_lossy()
    ));
    fs::write(
        &signature_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "key_id": "z3-vendor-k7-2026q2",
            "scheme": "ed25519",
            "signed_payload": checksum,
            "signature": signature
        }))
        .expect("serialize signature sidecar"),
    )
    .expect("write signature sidecar");
}

fn write_fake_solver_to(path: &Path, source_body: &str) -> PathBuf {
    let source = path.with_extension("rs");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fake solver dir");
    }
    fs::write(&source, source_body).expect("write fake solver source");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let output = Command::new(rustc)
        .arg(&source)
        .arg("-O")
        .arg("-o")
        .arg(path)
        .output()
        .expect("run rustc for fake solver");
    assert!(
        output.status.success(),
        "fake solver compile failed: {}",
        String::from_utf8_lossy(output.stderr.as_slice())
    );
    path.to_path_buf()
}

fn write_fake_timeout_solver(dir: &Path) -> PathBuf {
    let solver = if cfg!(windows) {
        dir.join("fake-timeout-z3.exe")
    } else {
        dir.join("fake-timeout-z3")
    };
    let source = r#"
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-version") {
        println!("Z3 version 4.16.0 - fake-timeout");
        return;
    }
    sleep(Duration::from_millis(7_000));
    println!("unsat");
}
"#;
    write_fake_solver_to(&solver, source)
}

fn write_fake_flaky_solver(dir: &Path) -> PathBuf {
    let solver = if cfg!(windows) {
        dir.join("fake-flaky-z3.exe")
    } else {
        dir.join("fake-flaky-z3")
    };
    let source = r#"
use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-version") {
        println!("Z3 version 4.16.0 - fake-flaky");
        return;
    }
    let counter_path = std::env::var("CLG_FAKE_Z3_COUNTER_FILE")
        .map(PathBuf::from)
        .expect("CLG_FAKE_Z3_COUNTER_FILE");
    let current = fs::read_to_string(&counter_path)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let next = current + 1;
    fs::write(&counter_path, next.to_string()).expect("write counter");
    if current % 2 == 0 {
        println!("unsat");
    } else {
        println!("sat");
    }
}
"#;
    write_fake_solver_to(&solver, source)
}

fn strict_import_map_artifact_path(out: &Path) -> PathBuf {
    let file_name = out
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("out.wasm");
    let artifact_name = if let Some(stem) = file_name.strip_suffix(".wasm") {
        format!("{stem}.strict-import-map.json")
    } else {
        format!("{file_name}.strict-import-map.json")
    };
    match out.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(artifact_name),
        _ => PathBuf::from(artifact_name),
    }
}

