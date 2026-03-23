#[test]
fn build_emits_assumption_boundaries_in_vc_json_and_proof_section() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("assumptions.clear");
    let wasm_path = tmp.path().join("assumptions.wasm");
    let vcs_path = tmp.path().join("assumptions.vc.json");
    let src = r#"
        pure function check(a: Bytes, b: Bytes, x: U64, y: U64) -> Bool
            ensure { result == std::bytes::eq_ct(a, b) }
            ensure { (x & y) == x }
        {
            std::bytes::eq_ct(a, b)
        }
        function main() -> Int { 0 }
    "#;
    fs::write(&src_path, src).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path)
        .assert()
        .success();

    let data = fs::read_to_string(&vcs_path).expect("read vcs");
    let items: Value = serde_json::from_str(&data).expect("json array");
    let arr = items.as_array().expect("array");
    let vc = arr
        .iter()
        .find(|entry| {
            entry.get("function").and_then(|v| v.as_str()) == Some("check")
                && entry.get("vc_id").and_then(|v| v.as_str()) == Some("vc:0")
        })
        .expect("check vc:0");

    let assumptions = vc
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .expect("assumptions items");
    let assurance = vc
        .get("assurance")
        .and_then(|v| v.as_object())
        .expect("assurance");
    assert_eq!(assurance.get("tier").and_then(|v| v.as_str()), Some("L0"));
    assert_eq!(
        assurance.get("label").and_then(|v| v.as_str()),
        Some("assumed")
    );
    assert!(assumptions
        .iter()
        .any(|item| item.get("id").and_then(|v| v.as_str()) == Some("unsigned.int_model")));
    assert!(assumptions
        .iter()
        .any(|item| item.get("id").and_then(|v| v.as_str()) == Some("bitwise.uninterpreted")));
    assert!(assumptions
        .iter()
        .any(|item| item.get("id").and_then(|v| v.as_str()) == Some("primitive.unproved")));
    let crypto = assumptions
        .iter()
        .find(|item| item.get("id").and_then(|v| v.as_str()) == Some("crypto.uninterpreted"))
        .expect("crypto assumption item");
    let crypto_symbols = crypto
        .get("symbols")
        .and_then(|v| v.as_array())
        .expect("crypto symbols");
    assert!(crypto_symbols
        .iter()
        .any(|symbol| symbol.as_str() == Some("std::bytes::eq_ct")));
    let crypto_intrinsic_levels = crypto
        .get("intrinsic_levels")
        .and_then(|v| v.as_array())
        .expect("crypto intrinsic levels");
    assert!(crypto_intrinsic_levels.iter().any(|entry| {
        entry.get("intrinsic").and_then(|v| v.as_str()) == Some("std::bytes::eq_ct")
            && entry.get("tier").and_then(|v| v.as_str()) == Some("L0")
            && entry.get("label").and_then(|v| v.as_str()) == Some("assumed")
    }));

    let wasm = fs::read(&wasm_path).expect("read wasm");
    let mut proof_data = None;
    for payload in Parser::new(0).parse_all(&wasm) {
        let payload = payload.expect("payload");
        if let Payload::CustomSection(section) = payload {
            if section.name() == "clearlang.proof" {
                proof_data = Some(section.data().to_vec());
                break;
            }
        }
    }
    let proof_data = proof_data.expect("proof section");
    let section: ProofAssumptionsSection =
        serde_cbor::from_slice(&proof_data).expect("decode proof");
    let section_assurance = section.assurance.expect("section assurance");
    assert_eq!(section_assurance.tier, "L0");
    assert_eq!(section_assurance.label, "assumed");
    assert_eq!(section_assurance.levels.l2, "verified module");
    assert_eq!(section_assurance.levels.l3, "verified package profile");
    let section_claim = section.assurance_claim.expect("section assurance_claim");
    assert_eq!(section_claim.compiler_mode, "standard");
    assert!(section_claim.non_strict_evidence_only);
    assert!(!section_claim.release_grade_trust);
    let vc_assumptions = section
        .functions
        .iter()
        .find(|f| f.name == "check")
        .and_then(|f| f.vcs.iter().find(|vc| vc.vc_id == "vc:0"))
        .expect("proof vc");
    let vc_assurance = vc_assumptions.assurance.as_ref().expect("vc assurance");
    assert_eq!(vc_assurance.tier, "L0");
    assert_eq!(vc_assurance.levels.l0, "assumed");
    assert_eq!(vc_assurance.levels.l1, "checked core");
    let vc_assumptions = vc_assumptions
        .assumptions
        .as_ref()
        .expect("proof vc assumptions");
    assert!(vc_assumptions
        .items
        .iter()
        .any(|item| item.id == "unsigned.int_model" && item.category == "unsigned"));
    assert!(vc_assumptions
        .items
        .iter()
        .any(|item| item.id == "bitwise.uninterpreted" && item.category == "bitwise"));
    assert!(vc_assumptions.items.iter().any(|item| {
        item.id == "primitive.unproved"
            && item.category == "primitive"
            && item
                .symbols
                .iter()
                .any(|symbol| symbol == "std::bytes::eq_ct")
    }));
    assert!(vc_assumptions.items.iter().any(|item| {
        item.id == "crypto.uninterpreted"
            && item.category == "crypto"
            && item
                .symbols
                .iter()
                .any(|symbol| symbol == "std::bytes::eq_ct")
            && item.intrinsic_levels.iter().any(|level| {
                level.intrinsic == "std::bytes::eq_ct"
                    && level.tier == "L0"
                    && level.label == "assumed"
            })
    }));
}

#[test]
fn build_labels_external_dependencies_as_assumed_boundaries() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let pkg_dir = root.join("pkg");
    fs::create_dir_all(&pkg_dir).expect("create pkg dir");
    fs::write(pkg_dir.join("extpkg.wasm"), [0u8]).expect("write package artifact");

    fs::write(
        root.join("clg.package-metadata.json"),
        r#"{
  "schema_version": 0,
  "packages": [
    {
      "name": "extpkg",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "pkg/extpkg.wasm" },
      "abi_id": "abi:extpkg:1.0.0"
    }
  ]
}"#,
    )
    .expect("write metadata");
    fs::write(
        root.join("clg.package-abi.json"),
        r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:extpkg:1.0.0",
      "package": "extpkg",
      "version": "1.0.0",
      "imports": [
        {
          "symbol": "extpkg::math::add2",
          "effect": "pure",
          "params": ["Int", "Int"],
          "ret": "Int",
          "capability": null
        }
      ]
    }
  ]
}"#,
    )
    .expect("write abi");

    let src = r#"
        import extpkg::math::{add2}

        pure function rely(a: Int, b: Int) -> Int
            ensure { result == add2(a, b) }
        { add2(a, b) }

        function main() -> Int { rely(1, 2) }
    "#;
    let src_path = root.join("main.clear");
    let wasm_path = root.join("out.wasm");
    let vcs_path = root.join("out.vc.json");
    fs::write(&src_path, src).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .args(["--emit-vcs"])
        .arg(&vcs_path)
        .assert()
        .success();

    let data = fs::read_to_string(&vcs_path).expect("read vcs");
    let items: Value = serde_json::from_str(&data).expect("json array");
    let arr = items.as_array().expect("array");
    let vc = arr
        .iter()
        .find(|entry| {
            entry.get("function").and_then(|v| v.as_str()) == Some("rely")
                && entry.get("vc_id").and_then(|v| v.as_str()) == Some("vc:0")
        })
        .expect("rely vc:0");
    let assumptions = vc
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .expect("assumptions items");
    let ext = assumptions
        .iter()
        .find(|item| item.get("id").and_then(|v| v.as_str()) == Some("external.dependency"))
        .expect("external assumption item");
    assert_eq!(
        ext.get("category").and_then(|v| v.as_str()),
        Some("external")
    );
    let ext_symbols = ext
        .get("symbols")
        .and_then(|v| v.as_array())
        .expect("external symbols");
    assert!(
        ext_symbols
            .iter()
            .any(|symbol| symbol.as_str().map(|s| s.contains("add2")).unwrap_or(false)),
        "expected add2 symbol in external dependency boundary"
    );

    let wasm = fs::read(&wasm_path).expect("read wasm");
    let mut proof_data = None;
    for payload in Parser::new(0).parse_all(&wasm) {
        let payload = payload.expect("payload");
        if let Payload::CustomSection(section) = payload {
            if section.name() == "clearlang.proof" {
                proof_data = Some(section.data().to_vec());
                break;
            }
        }
    }
    let proof_data = proof_data.expect("proof section");
    let section: ProofAssumptionsSection =
        serde_cbor::from_slice(&proof_data).expect("decode proof");
    let vc_assumptions = section
        .functions
        .iter()
        .find(|f| f.name == "rely")
        .and_then(|f| f.vcs.iter().find(|vc| vc.vc_id == "vc:0"))
        .and_then(|vc| vc.assumptions.as_ref())
        .expect("proof vc assumptions");
    assert!(vc_assumptions.items.iter().any(|item| {
        item.id == "external.dependency"
            && item.category == "external"
            && item.symbols.iter().any(|symbol| symbol.contains("add2"))
    }));
}

#[test]
fn standard_compiler_mode_accepts_labeled_assumptions_for_l3_gate() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("strict_l3_gate.clear");
    let wasm_path = tmp.path().join("strict_l3_gate.wasm");
    let vcs_path = tmp.path().join("strict_l3_gate.vc.json");
    let src = r#"
        pure function check(a: Bytes, b: Bytes, x: U64, y: U64) -> Bool
            ensure { result == std::bytes::eq_ct(a, b) }
            ensure { (x & y) == x }
        {
            std::bytes::eq_ct(a, b)
        }
        function main() -> Int { 0 }
    "#;
    fs::write(&src_path, src).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .args(["--emit-vcs"])
        .arg(&vcs_path)
        .args(["--compiler-mode", "standard"])
        .assert()
        .success();

    assert!(wasm_path.exists(), "build should emit wasm");
    assert!(
        vcs_path.exists(),
        "standard-mode build should emit vcs when assumptions are labeled"
    );
}

#[test]
fn build_strict_proof_mode_ignores_marker_like_string_literals() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("strict_marker.clear");
    let wasm_path = tmp.path().join("strict_marker.wasm");
    let vcs_path = tmp.path().join("strict_marker.vc.json");
    let src = r#"
        pure function marker() -> String
            ensure { result == "clg.bit_" }
        { "clg.bit_" }
        function main() -> Int { 0 }
    "#;
    fs::write(&src_path, src).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path)
        .assert()
        .success();

    assert!(wasm_path.exists(), "build should emit wasm");
    assert!(
        vcs_path.exists(),
        "strict mode should not fail on marker-like string literals"
    );
}

#[test]
fn build_allows_opt_out_of_strict_proof_mode() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("strict_marker_opt_out.clear");
    let wasm_path = tmp.path().join("strict_marker_opt_out.wasm");
    let vcs_path = tmp.path().join("strict_marker_opt_out.vc.json");
    let src = r#"
        pure function marker() -> String
            ensure { result == "clg.bit_" }
        { "clg.bit_" }
        function main() -> Int { 0 }
    "#;
    fs::write(&src_path, src).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path)
        .arg("--proof-strict=false")
        .assert()
        .success();

    assert!(wasm_path.exists(), "build should still emit wasm");
    assert!(
        vcs_path.exists(),
        "build with strict-mode opt-out should emit vcs output"
    );
}

#[derive(Deserialize)]
struct ProofFunctionTraceEntry {
    name: String,
    #[serde(default)]
    canonical_name: Option<String>,
}

#[derive(Deserialize)]
struct ProofTraceSection {
    functions: Vec<ProofFunctionTraceEntry>,
}

#[test]
fn build_emits_shortened_name_traceability_mapping() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("trace.clear");
    let wasm_path = tmp.path().join("trace.wasm");
    let vcs_path = tmp.path().join("trace.vc.json");
    let src = r#"
        pure function incredibly_descriptive_generic_identity_for_traceability_demo<T>(x: T) -> T
            ensure { result == result }
        { x }
        function main() -> Int {
            incredibly_descriptive_generic_identity_for_traceability_demo(1)
        }
    "#;
    fs::write(&src_path, src).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .env("CLG_MANGLE_MAX_LEN", "24")
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path)
        .assert()
        .success();

    let data = fs::read_to_string(&vcs_path).expect("read vcs");
    let items: Value = serde_json::from_str(&data).expect("json array");
    let arr = items.as_array().expect("array");
    let mapped_vc = arr
        .iter()
        .find_map(|entry| {
            let emitted = entry.get("function")?.as_str()?;
            let canonical = entry.get("canonical_function")?.as_str()?;
            Some((emitted.to_string(), canonical.to_string()))
        })
        .expect("vc with canonical_function");

    assert!(
        mapped_vc.0.len() <= 24,
        "emitted name should respect CLG_MANGLE_MAX_LEN"
    );
    assert!(
        mapped_vc.1.len() > mapped_vc.0.len(),
        "canonical name should preserve full mangled path"
    );
    assert!(
        mapped_vc
            .1
            .contains("incredibly_descriptive_generic_identity_for_traceability_demo"),
        "canonical name should keep full symbol prefix"
    );
    assert!(
        mapped_vc.1.ends_with("$Int"),
        "canonical name should keep type instantiation suffix"
    );

    let wasm = fs::read(&wasm_path).expect("read wasm");
    let mut proof_data = None;
    for payload in Parser::new(0).parse_all(&wasm) {
        let payload = payload.expect("payload");
        if let Payload::CustomSection(section) = payload {
            if section.name() == "clearlang.proof" {
                proof_data = Some(section.data().to_vec());
                break;
            }
        }
    }

    let proof_data = proof_data.expect("proof section");
    let section: ProofTraceSection = serde_cbor::from_slice(&proof_data).expect("decode proof");
    let mapped_fn = section
        .functions
        .iter()
        .find_map(|f| {
            let canonical = f.canonical_name.as_ref()?;
            Some((f.name.as_str(), canonical.as_str()))
        })
        .expect("proof function with canonical_name");

    assert_eq!(mapped_fn.0, mapped_vc.0);
    assert_eq!(mapped_fn.1, mapped_vc.1);
}
