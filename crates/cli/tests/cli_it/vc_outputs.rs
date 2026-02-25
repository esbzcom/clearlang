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
    let repair_hints = first
        .get("diagnostics")
        .and_then(|o| o.get("repair_hints"))
        .and_then(|o| o.as_object())
        .expect("repair_hints obj");
    assert_eq!(
        repair_hints.get("on_status").and_then(|s| s.as_str()),
        Some("failed")
    );
    let hint_items = repair_hints
        .get("items")
        .and_then(|v| v.as_array())
        .expect("repair_hints.items");
    assert!(!hint_items.is_empty(), "expected at least one repair hint");
    let first_hint = hint_items[0].as_object().expect("first hint obj");
    assert_eq!(
        first_hint.get("kind").and_then(|s| s.as_str()),
        Some("contract.ensure")
    );
    assert_eq!(
        first_hint.get("minimal_clause").and_then(|s| s.as_str()),
        Some("ensure { result > x }")
    );
    let failure_slice = first
        .get("diagnostics")
        .and_then(|o| o.get("failure_slice"))
        .and_then(|o| o.as_object())
        .expect("failure_slice obj");
    assert_eq!(
        failure_slice.get("on_status").and_then(|s| s.as_str()),
        Some("failed")
    );
    assert_eq!(
        failure_slice.get("clause_kind").and_then(|s| s.as_str()),
        Some("ensure")
    );
    let counterexample = first
        .get("diagnostics")
        .and_then(|o| o.get("counterexample"))
        .and_then(|o| o.as_object())
        .expect("counterexample obj");
    assert_eq!(
        counterexample.get("state").and_then(|s| s.as_str()),
        Some("solver_unavailable")
    );
    let bindings = counterexample
        .get("bindings")
        .and_then(|v| v.as_array())
        .expect("counterexample bindings");
    assert!(
        bindings.iter().any(|b| {
            b.get("symbol").and_then(|v| v.as_str()) == Some("result")
                && b.get("value").is_some_and(|v| v.is_null())
        }),
        "expected counterexample bindings to include `result`"
    );
    let proof_context = first
        .get("diagnostics")
        .and_then(|o| o.get("proof_context"))
        .and_then(|o| o.as_object())
        .expect("proof_context obj");
    let mut proof_context_keys: Vec<&str> = proof_context.keys().map(|k| k.as_str()).collect();
    proof_context_keys.sort_unstable();
    assert_eq!(
        proof_context_keys,
        vec![
            "assumptions",
            "format",
            "model_snippet",
            "on_status",
            "span_map",
            "vc"
        ]
    );
    assert_eq!(
        proof_context.get("format").and_then(|s| s.as_str()),
        Some("clg.proof_context.v1")
    );
    let proof_context_vc = proof_context
        .get("vc")
        .and_then(|o| o.as_object())
        .expect("proof_context vc");
    let mut proof_context_vc_keys: Vec<&str> =
        proof_context_vc.keys().map(|k| k.as_str()).collect();
    proof_context_vc_keys.sort_unstable();
    assert_eq!(
        proof_context_vc_keys,
        vec!["clause_kind", "function", "post", "pre", "status", "vc", "vc_id"]
    );
    assert!(
        proof_context_vc.get("assurance").is_none(),
        "proof_context.vc should not duplicate assurance payload"
    );
    assert!(
        proof_context_vc.get("diagnostics").is_none(),
        "proof_context.vc should not recursively embed diagnostics"
    );
    assert_eq!(
        proof_context_vc.get("vc_id").and_then(|s| s.as_str()),
        Some("vc:0")
    );
    assert_eq!(
        proof_context_vc.get("clause_kind").and_then(|s| s.as_str()),
        Some("ensure")
    );
    let proof_context_assumptions = proof_context
        .get("assumptions")
        .and_then(|o| o.get("items"))
        .and_then(|o| o.as_array())
        .expect("proof_context assumptions");
    assert!(
        proof_context_assumptions.is_empty(),
        "expected empty assumptions for checked-core vc"
    );
    let proof_context_span_map = proof_context
        .get("span_map")
        .and_then(|o| o.as_object())
        .expect("proof_context span_map");
    assert_eq!(
        proof_context_span_map
            .get("focus_role")
            .and_then(|s| s.as_str()),
        Some("post")
    );
    let span_map_pre = proof_context_span_map
        .get("pre")
        .and_then(|o| o.as_object())
        .expect("proof_context span_map.pre");
    let span_map_post = proof_context_span_map
        .get("post")
        .and_then(|o| o.as_object())
        .expect("proof_context span_map.post");
    assert_eq!(
        span_map_pre.get("start").and_then(|n| n.as_u64()),
        positions.get("pre_start").and_then(|n| n.as_u64())
    );
    assert_eq!(
        span_map_pre.get("end").and_then(|n| n.as_u64()),
        positions.get("pre_end").and_then(|n| n.as_u64())
    );
    assert_eq!(
        span_map_post.get("start").and_then(|n| n.as_u64()),
        positions.get("post_start").and_then(|n| n.as_u64())
    );
    assert_eq!(
        span_map_post.get("end").and_then(|n| n.as_u64()),
        positions.get("post_end").and_then(|n| n.as_u64())
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

#[test]
fn build_emits_loop_repair_hints() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("loop_hints.clear");
    let wasm_path = tmp.path().join("loop_hints.wasm");
    let vcs_path = tmp.path().join("loop_hints.vc.json");
    let src = r#"
        pure function countdown(n: Int) -> Int
            require { n > 1 }
            ensure { result >= 0 }
        {
            while n > 0 invariant { n >= 0 } variant { n } { n; }
            n + 0
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

    let find_hint = |vc_id: &str| -> &serde_json::Map<String, Value> {
        arr.iter()
            .find(|entry| entry.get("vc_id").and_then(|v| v.as_str()) == Some(vc_id))
            .and_then(|entry| entry.get("diagnostics"))
            .and_then(|d| d.get("repair_hints"))
            .and_then(|h| h.get("items"))
            .and_then(|items| items.as_array())
            .and_then(|items| items.first())
            .and_then(|hint| hint.as_object())
            .expect("hint object")
    };

    let invariant = find_hint("loop:0:invariant");
    assert_eq!(
        invariant.get("kind").and_then(|v| v.as_str()),
        Some("loop.invariant")
    );
    assert_eq!(
        invariant.get("minimal_clause").and_then(|v| v.as_str()),
        Some("invariant { n >= 0 }")
    );
    let invariant_slice = arr
        .iter()
        .find(|entry| entry.get("vc_id").and_then(|v| v.as_str()) == Some("loop:0:invariant"))
        .and_then(|entry| entry.get("diagnostics"))
        .and_then(|d| d.get("failure_slice"))
        .and_then(|s| s.as_object())
        .expect("invariant failure_slice");
    assert_eq!(
        invariant_slice.get("clause_kind").and_then(|v| v.as_str()),
        Some("invariant")
    );

    let nonneg = find_hint("loop:0:variant_nonneg");
    assert_eq!(
        nonneg.get("kind").and_then(|v| v.as_str()),
        Some("loop.variant_nonneg")
    );
    assert_eq!(
        nonneg.get("minimal_clause").and_then(|v| v.as_str()),
        Some("variant { n }")
    );
    let nonneg_counterexample = arr
        .iter()
        .find(|entry| entry.get("vc_id").and_then(|v| v.as_str()) == Some("loop:0:variant_nonneg"))
        .and_then(|entry| entry.get("diagnostics"))
        .and_then(|d| d.get("counterexample"))
        .and_then(|s| s.as_object())
        .expect("variant_nonneg counterexample");
    assert!(
        nonneg_counterexample
            .get("bindings")
            .and_then(|v| v.as_array())
            .is_some_and(|bindings| {
                bindings
                    .iter()
                    .any(|entry| entry.get("symbol").and_then(|v| v.as_str()) == Some("n"))
            }),
        "expected loop variant counterexample bindings to include `n`"
    );
    let nonneg_proof_context = arr
        .iter()
        .find(|entry| entry.get("vc_id").and_then(|v| v.as_str()) == Some("loop:0:variant_nonneg"))
        .and_then(|entry| entry.get("diagnostics"))
        .and_then(|d| d.get("proof_context"))
        .and_then(|p| p.as_object())
        .expect("variant_nonneg proof_context");
    let nonneg_model = nonneg_proof_context
        .get("model_snippet")
        .and_then(|m| m.as_object())
        .expect("variant_nonneg model_snippet");
    assert_eq!(
        nonneg_model.get("state").and_then(|v| v.as_str()),
        Some("solver_unavailable")
    );
    assert!(
        nonneg_model
            .get("bindings")
            .and_then(|v| v.as_array())
            .is_some_and(|bindings| {
                bindings
                    .iter()
                    .any(|entry| entry.get("symbol").and_then(|v| v.as_str()) == Some("n"))
            }),
        "expected proof_context.model_snippet bindings to include `n`"
    );
    let nonneg_span_map = nonneg_proof_context
        .get("span_map")
        .and_then(|s| s.as_object())
        .expect("variant_nonneg span_map");
    assert_eq!(
        nonneg_span_map
            .get("focus_role")
            .and_then(|v| v.as_str()),
        Some("post")
    );
    let nonneg_pre = nonneg_span_map
        .get("pre")
        .and_then(|s| s.as_object())
        .expect("variant_nonneg span_map.pre");
    let nonneg_post = nonneg_span_map
        .get("post")
        .and_then(|s| s.as_object())
        .expect("variant_nonneg span_map.post");
    assert!(
        nonneg_pre
            .get("start")
            .and_then(|v| v.as_u64())
            .is_some_and(|v| v > 0),
        "variant_nonneg span_map.pre.start should be present and non-zero"
    );
    assert!(
        nonneg_post
            .get("start")
            .and_then(|v| v.as_u64())
            .is_some_and(|v| v > 0),
        "variant_nonneg span_map.post.start should be present and non-zero"
    );

    let decrease = find_hint("loop:0:variant_decrease");
    assert_eq!(
        decrease.get("kind").and_then(|v| v.as_str()),
        Some("loop.variant_decrease")
    );
    assert_eq!(
        decrease.get("minimal_clause").and_then(|v| v.as_str()),
        Some("variant { n }")
    );
}

#[derive(Deserialize)]
struct IntrinsicLevelEntry {
    intrinsic: String,
    tier: String,
    label: String,
}

#[derive(Deserialize)]
struct ProofAssumptionEntry {
    id: String,
    category: String,
    #[serde(default)]
    symbols: Vec<String>,
    #[serde(default)]
    intrinsic_levels: Vec<IntrinsicLevelEntry>,
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
