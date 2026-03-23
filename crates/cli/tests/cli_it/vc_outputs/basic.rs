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
    let assurance_claim = first
        .get("assurance_claim")
        .and_then(|o| o.as_object())
        .expect("assurance_claim obj");
    assert_eq!(
        assurance_claim
            .get("compiler_mode")
            .and_then(|s| s.as_str()),
        Some("standard")
    );
    assert_eq!(
        assurance_claim
            .get("non_strict_evidence_only")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        assurance_claim
            .get("release_grade_trust")
            .and_then(|v| v.as_bool()),
        Some(false)
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
        vec![
            "clause_kind",
            "function",
            "post",
            "pre",
            "status",
            "vc",
            "vc_id"
        ]
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
        nonneg_span_map.get("focus_role").and_then(|v| v.as_str()),
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

