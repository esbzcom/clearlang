use super::*;
use serde::Deserialize;
use wasmparser::{Parser, Payload};

#[test]
fn build_emits_vcs_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("contract.clear");
    let wasm_path = tmp.path().join("out.wasm");
    let vcs_path = tmp.path().join("out.vc.json");
    let src = r#"
        pure function inc(x: Int) -> Int
            require { 0 <= x }
            ensure { result > x }
        { x + 1 }
        function main() -> Int { inc(1) }
    "#;
    fs::write(&src_path, src).expect("write contract");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path);
    cmd.assert().success();

    let data = fs::read_to_string(&vcs_path).expect("read vcs");
    let items: Value = serde_json::from_str(&data).expect("json array");
    let arr = items.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    let first = &arr[0];
    assert_eq!(first.get("version").and_then(|n| n.as_u64()), Some(2));
    assert_eq!(first.get("function").and_then(|s| s.as_str()), Some("inc"));
    assert_eq!(
        first.get("status").and_then(|s| s.as_str()),
        Some("generated")
    );
    let assurance = first
        .get("assurance")
        .and_then(|o| o.as_object())
        .expect("assurance obj");
    assert_eq!(assurance.get("tier").and_then(|s| s.as_str()), Some("L1"));
    assert_eq!(
        assurance.get("label").and_then(|s| s.as_str()),
        Some("checked core")
    );
    let levels = assurance
        .get("levels")
        .and_then(|o| o.as_object())
        .expect("levels obj");
    assert_eq!(levels.get("L0").and_then(|s| s.as_str()), Some("assumed"));
    assert_eq!(
        levels.get("L3").and_then(|s| s.as_str()),
        Some("verified package profile")
    );
    let pre = first
        .get("pre")
        .and_then(|o| o.as_object())
        .expect("pre obj");
    assert!(pre
        .get("ast")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .contains("<="));
    let vc = first.get("vc").and_then(|o| o.as_object()).expect("vc obj");
    let vc_text = vc.get("smt2").and_then(|s| s.as_str()).unwrap_or("");
    assert!(vc_text.contains("=>"));
    assert!(vc_text.contains("(+ x 1)"));
    let positions = first
        .get("positions")
        .and_then(|o| o.as_object())
        .expect("positions obj");
    assert_eq!(
        positions.get("file").and_then(|s| s.as_str()),
        Some(src_path.to_string_lossy().as_ref())
    );
    assert!(first.get("refinements").is_none());
    assert!(first.get("assumptions").is_none());
}

#[test]
fn build_emits_refinement_premises_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("refined.clear");
    let wasm_path = tmp.path().join("refined.wasm");
    let vcs_path = tmp.path().join("refined.vc.json");
    let src = r#"
        type Nat = Int where n >= 0;
        pure function inc(a: Nat) -> Nat { a + 1 }
        function main() -> Int { 0 }
    "#;
    fs::write(&src_path, src).expect("write refined source");

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
    assert_eq!(arr.len(), 1);
    let first = &arr[0];
    let refinements = first
        .get("refinements")
        .and_then(|o| o.as_object())
        .expect("refinements obj");
    let premises = refinements
        .get("premises")
        .and_then(|p| p.as_array())
        .expect("premises array");
    assert_eq!(premises.len(), 2);

    let param = &premises[0];
    assert_eq!(param.get("id").and_then(|s| s.as_str()), Some("ref:0"));
    assert_eq!(param.get("alias").and_then(|s| s.as_str()), Some("Nat"));
    assert_eq!(param.get("binder").and_then(|s| s.as_str()), Some("n"));
    let substitution = param
        .get("substitution")
        .and_then(|o| o.as_object())
        .expect("substitution obj");
    assert_eq!(substitution.get("ast").and_then(|s| s.as_str()), Some("a"));
    let predicate = param
        .get("predicate")
        .and_then(|o| o.as_object())
        .expect("predicate obj");
    assert_eq!(
        predicate.get("ast").and_then(|s| s.as_str()),
        Some("a >= 0")
    );
    let attachment = param
        .get("attachment")
        .and_then(|o| o.as_object())
        .expect("attachment obj");
    assert_eq!(
        attachment.get("kind").and_then(|s| s.as_str()),
        Some("param")
    );
    let detail = attachment
        .get("detail")
        .and_then(|o| o.as_object())
        .expect("detail obj");
    assert_eq!(detail.get("param").and_then(|s| s.as_str()), Some("a"));

    let ret = &premises[1];
    assert_eq!(ret.get("id").and_then(|s| s.as_str()), Some("ref:1"));
    let attachment = ret
        .get("attachment")
        .and_then(|o| o.as_object())
        .expect("attachment obj");
    assert_eq!(
        attachment.get("kind").and_then(|s| s.as_str()),
        Some("return")
    );
    let detail = attachment
        .get("detail")
        .and_then(|o| o.as_object())
        .expect("detail obj");
    assert_eq!(
        detail.get("result").and_then(|s| s.as_str()),
        Some("result")
    );
}
#[test]
fn build_emits_variant_vcs_json() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("coalesce.clear");
    let wasm_path = tmp.path().join("coalesce.wasm");
    let vcs_path = tmp.path().join("coalesce.vc.json");
    let src = r#"
        pure function pick(opt: Option<Int>) -> Option<Int>
            ensure { result == result }
        { Some((opt ?? 7) + 1) }
        function main() -> Int { if let Some(v) = pick(Some(1)) { v } else { 0 } }
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
    assert_eq!(arr.len(), 1);
    let vc = arr[0]
        .get("vc")
        .and_then(|o| o.as_object())
        .and_then(|o| o.get("smt2"))
        .and_then(|s| s.as_str())
        .expect("vc smt2");
    assert!(vc.contains("cl.variant.tag"));
    assert!(!vc.contains("unsupported"));
}

#[derive(Deserialize)]
struct ProofAssumptionEntry {
    id: String,
    category: String,
    #[serde(default)]
    symbols: Vec<String>,
}

#[derive(Deserialize)]
struct AssuranceLevelsEntry {
    #[serde(rename = "L0")]
    l0: String,
    #[serde(rename = "L1")]
    l1: String,
    #[serde(rename = "L2")]
    l2: String,
    #[serde(rename = "L3")]
    l3: String,
}

#[derive(Deserialize)]
struct AssuranceEntry {
    tier: String,
    label: String,
    levels: AssuranceLevelsEntry,
}

#[derive(Deserialize)]
struct ProofAssumptionsEntry {
    items: Vec<ProofAssumptionEntry>,
}

#[derive(Deserialize)]
struct ProofVcAssumptionsEntry {
    vc_id: String,
    #[serde(default)]
    assumptions: Option<ProofAssumptionsEntry>,
    #[serde(default)]
    assurance: Option<AssuranceEntry>,
}

#[derive(Deserialize)]
struct ProofFunctionAssumptionsEntry {
    name: String,
    vcs: Vec<ProofVcAssumptionsEntry>,
}

#[derive(Deserialize)]
struct ProofAssumptionsSection {
    functions: Vec<ProofFunctionAssumptionsEntry>,
    #[serde(default)]
    assurance: Option<AssuranceEntry>,
}

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
    }));
}

#[test]
fn build_labels_external_dependencies_as_assumed_boundaries() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let pkg_dir = root.join("pkg");
    fs::create_dir_all(&pkg_dir).expect("create pkg dir");
    fs::write(pkg_dir.join("extpkg.wasm"), [0u8]).expect("write package artifact");

    let metadata = r#"
{
  "schema_version": 1,
  "packages": [
    {
      "name": "extpkg",
      "version": "1.0.0",
      "artifact": { "format": "wasm", "path": "pkg/extpkg.wasm" },
      "modules": [
        {
          "path": "extpkg::math",
          "exports": [
            {
              "name": "add2",
              "kind": "value",
              "effect": "pure",
              "params": [
                { "name": "a", "type": "Int" },
                { "name": "b", "type": "Int" }
              ],
              "ret": "Int",
              "import": { "module": "extpkg_math", "name": "add2" }
            }
          ]
        }
      ]
    }
  ]
}
    "#;
    fs::write(root.join("clg-packages.json"), metadata.trim()).expect("write metadata");

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
