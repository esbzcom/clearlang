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

