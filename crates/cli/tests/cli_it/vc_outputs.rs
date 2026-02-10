use super::*;

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
