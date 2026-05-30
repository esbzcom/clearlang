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
    assert!(
        assumptions
            .iter()
            .all(|item| item.get("id").and_then(|v| v.as_str()) != Some("unsigned.int_model")),
        "U64-covered paths should not emit unsigned.int_model assumption item"
    );
    assert!(
        assumptions
            .iter()
            .all(|item| item.get("id").and_then(|v| v.as_str()) != Some("bitwise.uninterpreted")),
        "U64-covered bitwise operators should not emit bitwise.uninterpreted assumption item"
    );
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
    assert!(
        vc_assumptions
            .items
            .iter()
            .all(|item| !(item.id == "unsigned.int_model" && item.category == "unsigned")),
        "proof section should not carry unsigned.int_model for U64-covered paths"
    );
    assert!(
        vc_assumptions
            .items
            .iter()
            .all(|item| !(item.id == "bitwise.uninterpreted" && item.category == "bitwise")),
        "proof section should not carry bitwise.uninterpreted for U64-covered operators"
    );
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
fn build_list_readonly_vcs_have_zero_assumptions() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("list_readonly.clear");
    let wasm_path = tmp.path().join("list_readonly.wasm");
    let vcs_path = tmp.path().join("list_readonly.vc.json");
    let src = r#"
        pure function empty_list() -> List<Int> { std::list::new() }

        pure function list_readonly(l: List<Int>) -> Bool
            require { std::list::len(l) >= 0 }
            ensure { std::list::is_empty(empty_list()) }
        {
            match std::list::get(l, 0) {
                Some(_) => false,
                None => std::list::is_empty(l)
            }
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
            entry.get("function").and_then(|v| v.as_str()) == Some("list_readonly")
                && entry.get("vc_id").and_then(|v| v.as_str()) == Some("vc:0")
        })
        .expect("list_readonly vc:0");
    let assumptions = vc
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        assumptions.is_empty(),
        "read-only list VC must be assumption-free"
    );
    let smt2 = vc
        .get("vc")
        .and_then(|v| v.get("smt2"))
        .and_then(|v| v.as_str())
        .expect("vc smt2");
    assert!(
        smt2.contains(
            "(forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i (|std::list::len| l))) (= (cl.variant.tag (|std::list::get| l i)) 1)))"
        ),
        "list read-only VC must include in-range index safety axiom"
    );
    assert!(
        smt2.contains(
            "(forall ((l Int) (i Int)) (=> (or (< i 0) (>= i (|std::list::len| l))) (= (cl.variant.tag (|std::list::get| l i)) 0)))"
        ),
        "list read-only VC must include out-of-range index safety axiom"
    );
    assert!(
        smt2.contains(
            "(forall ((l_before Int) (l_after Int) (i Int)) (=> (and (= l_before l_after) (<= 0 i) (< i (|std::list::len| l_before))) (= (|std::list::get| l_before i) (|std::list::get| l_after i))))"
        ),
        "list read-only VC must include unchanged-state get value-preservation axiom"
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
        .find(|f| f.name == "list_readonly")
        .and_then(|f| f.vcs.iter().find(|vc| vc.vc_id == "vc:0"))
        .and_then(|vc| vc.assumptions.as_ref());
    assert!(
        vc_assumptions
            .map(|assumptions| assumptions.items.is_empty())
            .unwrap_or(true),
        "proof section must also keep read-only list VC assumption-free"
    );
}

#[test]
fn build_list_append_pop_vcs_have_zero_assumptions() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("list_append_pop.clear");
    let wasm_path = tmp.path().join("list_append_pop.wasm");
    let vcs_path = tmp.path().join("list_append_pop.vc.json");
    let src = r#"
        pure function list_append_pop(l: List<Int>, x: Int) -> Bool
            ensure { std::list::len(std::list::push(l, x)) == std::list::len(l) + 1 }
            ensure {
                if std::list::len(l) > 0 {
                    match std::list::pop(l) {
                        Some(_) => true,
                        None => false
                    }
                } else {
                    match std::list::pop(l) {
                        Some(_) => false,
                        None => true
                    }
                }
            }
        {
            true
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
            entry.get("function").and_then(|v| v.as_str()) == Some("list_append_pop")
                && entry.get("vc_id").and_then(|v| v.as_str()) == Some("vc:0")
        })
        .expect("list_append_pop vc:0");
    let assumptions = vc
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        assumptions.is_empty(),
        "append/pop list VC must be assumption-free"
    );
    let smt2 = vc
        .get("vc")
        .and_then(|v| v.get("smt2"))
        .and_then(|v| v.as_str())
        .expect("vc smt2");
    assert!(
        smt2.contains(
            "(assert (forall ((l Int) (x Int)) (= (|std::list::len| (|std::list::push| l x)) (+ (|std::list::len| l) 1))))"
        ),
        "append/pop list VC must include push length-delta axiom"
    );
    assert!(
        smt2.contains(
            "(assert (forall ((l Int)) (=> (> (|std::list::len| l) 0) (= (cl.variant.tag (|std::list::pop| l)) 1))))"
        ),
        "append/pop list VC must include pop Some axiom"
    );
    assert!(
        smt2.contains(
            "(assert (forall ((l Int)) (=> (<= (|std::list::len| l) 0) (= (cl.variant.tag (|std::list::pop| l)) 0))))"
        ),
        "append/pop list VC must include pop None axiom"
    );
}

#[test]
fn build_list_indexed_mutation_vcs_have_zero_assumptions() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("list_indexed_mutation.clear");
    let wasm_path = tmp.path().join("list_indexed_mutation.wasm");
    let vcs_path = tmp.path().join("list_indexed_mutation.vc.json");
    let src = r#"
        pure function list_indexed_mutation(l: List<Int>, x: Int, i: Int) -> Bool
            require { 0 <= i && i < std::list::len(l) }
            ensure { std::list::len(std::list::insert(l, x, i)) == std::list::len(l) + 1 }
            ensure { std::list::len(std::list::remove(l, i)) == std::list::len(l) - 1 }
            ensure { std::list::len(std::list::remove_take(l, i)[0]) == std::list::len(l) - 1 }
        {
            match std::list::remove_take(l, i)[1] {
                Some(v) => std::list::get(l, i) == Some(v),
                None => false
            }
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
            entry.get("function").and_then(|v| v.as_str()) == Some("list_indexed_mutation")
                && entry.get("vc_id").and_then(|v| v.as_str()) == Some("vc:0")
        })
        .expect("list_indexed_mutation vc:0");
    let assumptions = vc
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        assumptions.is_empty(),
        "indexed list mutation VC must be assumption-free"
    );
    let smt2 = vc
        .get("vc")
        .and_then(|v| v.get("smt2"))
        .and_then(|v| v.as_str())
        .expect("vc smt2");
    assert!(
        smt2.contains(
            "(assert (forall ((l Int) (x Int) (i Int)) (=> (and (<= 0 i) (<= i (|std::list::len| l))) (= (|std::list::len| (|std::list::insert| l x i)) (+ (|std::list::len| l) 1)))))"
        ),
        "indexed list mutation VC must include insert length-delta axiom"
    );
    assert!(
        smt2.contains(
            "(assert (forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i (|std::list::len| l))) (= (|std::list::len| (|std::list::remove| l i)) (- (|std::list::len| l) 1)))))"
        ),
        "indexed list mutation VC must include remove length-delta axiom"
    );
    assert!(
        smt2.contains("(declare-fun cl.list.remove_take.list (Int) Int)"),
        "indexed list mutation VC must include remove_take list projection helper"
    );
    assert!(
        smt2.contains(
            "(assert (forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i (|std::list::len| l))) (= (|std::list::len| (cl.list.remove_take.list (|std::list::remove_take| l i))) (- (|std::list::len| l) 1)))))"
        ),
        "indexed list mutation VC must include in-range remove_take length axiom"
    );
}

#[test]
fn build_list_checked_mutation_vcs_have_zero_assumptions() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("list_checked_mutation.clear");
    let wasm_path = tmp.path().join("list_checked_mutation.wasm");
    let vcs_path = tmp.path().join("list_checked_mutation.vc.json");
    let src = r#"
        pure function list_checked_mutation(l: List<Int>, x: Int, i: Int) -> Bool
            ensure { result == result }
        {
            match std::list::insert_checked(l, x, i) {
                Ok(updated) => std::list::len(updated) >= std::list::len(l),
                Err(_err_insert) => match std::list::remove_checked(l, i) {
                    Ok(updated2) => std::list::len(updated2) <= std::list::len(l),
                    Err(_err_remove) => true
                }
            }
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
            entry.get("function").and_then(|v| v.as_str()) == Some("list_checked_mutation")
                && entry.get("vc_id").and_then(|v| v.as_str()) == Some("vc:0")
        })
        .expect("list_checked_mutation vc:0");
    let assumptions = vc
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        assumptions.is_empty(),
        "checked list mutation VC must be assumption-free"
    );
    let smt2 = vc
        .get("vc")
        .and_then(|v| v.get("smt2"))
        .and_then(|v| v.as_str())
        .expect("vc smt2");
    assert!(
        smt2.contains(
            "(assert (forall ((l Int) (x Int) (i Int)) (=> (and (<= 0 i) (<= i (|std::list::len| l))) (= (cl.variant.tag (|std::list::insert_checked| l x i)) 1))))"
        ),
        "checked list mutation VC must include insert_checked Ok-tag axiom"
    );
    assert!(
        smt2.contains(
            "(assert (forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i (|std::list::len| l))) (= (cl.variant.tag (|std::list::remove_checked| l i)) 1))))"
        ),
        "checked list mutation VC must include remove_checked Ok-tag axiom"
    );
    assert!(
        smt2.contains(
            "(assert (forall ((l Int) (x Int) (i Int)) (=> (and (<= 0 i) (<= i (|std::list::len| l))) (= (cl.variant.payload_lo (|std::list::insert_checked| l x i)) (|std::list::insert| l x i)))))"
        ),
        "checked list mutation VC must include insert_checked payload-to-insert axiom"
    );
}

#[test]
fn build_map_readonly_vcs_have_zero_assumptions() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("map_readonly.clear");
    let wasm_path = tmp.path().join("map_readonly.wasm");
    let vcs_path = tmp.path().join("map_readonly.vc.json");
    let src = r#"
        pure function empty_map() -> Map<Int, Int> {
            std::map::new()
        }

        pure function map_readonly(m: Map<Int, Int>, k: Int) -> Bool
            ensure { result == result }
        {
            if std::map::contains(m, k) {
                match std::map::get(m, k) {
                    Some(v) => v == v,
                    None => false
                }
            } else {
                match std::map::get(m, k) {
                    Some(_) => false,
                    None => std::map::len(m) >= 0 && (std::map::is_empty(empty_map()) || std::map::is_empty(empty_map()) == false)
                }
            }
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
            entry.get("function").and_then(|v| v.as_str()) == Some("map_readonly")
                && entry.get("vc_id").and_then(|v| v.as_str()) == Some("vc:0")
        })
        .expect("map_readonly vc:0");
    let assumptions = vc
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        assumptions.is_empty(),
        "map read-only VC must be assumption-free"
    );
    let smt2 = vc
        .get("vc")
        .and_then(|v| v.get("smt2"))
        .and_then(|v| v.as_str())
        .expect("vc smt2");
    assert!(
        smt2.contains("(assert (= (|std::map::len| (|std::map::new|)) 0))"),
        "map read-only VC must include new->len axiom"
    );
    assert!(
        smt2.contains(
            "(assert (forall ((m Int) (k Int)) (=> (|std::map::contains| m k) (= (cl.variant.tag (|std::map::get| m k)) 1))))"
        ),
        "map read-only VC must include contains=>get(Some) axiom"
    );
}

#[test]
fn build_map_membership_overwrite_vcs_have_zero_assumptions() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("map_membership_overwrite.clear");
    let wasm_path = tmp.path().join("map_membership_overwrite.wasm");
    let vcs_path = tmp.path().join("map_membership_overwrite.vc.json");
    let src = r#"
        pure function map_membership_overwrite(m: Map<Int, Int>, k: Int, j: Int, v: Int) -> Bool
            ensure { result == result }
        {
            let after_insert = std::map::insert(m, k, v);
            let after_take = std::map::insert_take(m, k, v);
            let after_take_map = after_take[0];
            if k == j {
                match std::map::get(after_insert, j) {
                    Some(got) => got == v,
                    None => false
                }
            } else {
                std::map::contains(after_insert, j) == std::map::contains(m, j)
                    && std::map::contains(after_take_map, j) == std::map::contains(m, j)
            }
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
            entry.get("function").and_then(|v| v.as_str()) == Some("map_membership_overwrite")
                && entry.get("vc_id").and_then(|v| v.as_str()) == Some("vc:0")
        })
        .expect("map_membership_overwrite vc:0");
    let assumptions = vc
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        assumptions.is_empty(),
        "map membership/overwrite VC must be assumption-free"
    );
    let smt2 = vc
        .get("vc")
        .and_then(|v| v.get("smt2"))
        .and_then(|v| v.as_str())
        .expect("vc smt2");
    assert!(
        smt2.contains(
            "(assert (forall ((m Int) (k Int) (v Int)) (|std::map::contains| (|std::map::insert| m k v) k)))"
        ),
        "map membership/overwrite VC must include insert=>contains axiom"
    );
    assert!(
        smt2.contains(
            "(assert (forall ((m Int) (k Int) (v Int)) (= (cl.map.insert_take.map (|std::map::insert_take| m k v)) (|std::map::insert| m k v))))"
        ),
        "map membership/overwrite VC must include insert_take map projection axiom"
    );
}

#[test]
fn build_map_mutation_take_vcs_have_zero_assumptions() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("map_mutation_take.clear");
    let wasm_path = tmp.path().join("map_mutation_take.wasm");
    let vcs_path = tmp.path().join("map_mutation_take.vc.json");
    let src = r#"
        pure function map_mutation_take(m: Map<Int, Int>, k: Int, j: Int) -> Bool
            ensure { result == result }
        {
            let after_remove = std::map::remove(m, k);
            let after_take = std::map::remove_take(m, k);
            let after_take_map = after_take[0];
            if k == j {
                std::map::contains(after_remove, j) == false
                    && std::map::contains(after_take_map, j) == false
            } else {
                std::map::contains(after_remove, j) == std::map::contains(m, j)
                    && std::map::contains(after_take_map, j) == std::map::contains(m, j)
            }
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
            entry.get("function").and_then(|v| v.as_str()) == Some("map_mutation_take")
                && entry.get("vc_id").and_then(|v| v.as_str()) == Some("vc:0")
        })
        .expect("map_mutation_take vc:0");
    let assumptions = vc
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        assumptions.is_empty(),
        "map mutation/take VC must be assumption-free"
    );
    let smt2 = vc
        .get("vc")
        .and_then(|v| v.get("smt2"))
        .and_then(|v| v.as_str())
        .expect("vc smt2");
    assert!(
        smt2.contains(
            "(assert (forall ((m Int) (k Int)) (not (|std::map::contains| (|std::map::remove| m k) k))))"
        ),
        "map mutation/take VC must include remove clears-key axiom"
    );
    assert!(
        smt2.contains(
            "(assert (forall ((m Int) (k Int)) (= (cl.map.remove_take.map (|std::map::remove_take| m k)) (|std::map::remove| m k))))"
        ),
        "map mutation/take VC must include remove_take map projection axiom"
    );
}

#[test]
fn build_set_subset_membership_vcs_have_zero_assumptions() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("set_subset.clear");
    let wasm_path = tmp.path().join("set_subset.wasm");
    let vcs_path = tmp.path().join("set_subset.vc.json");
    let src = r#"
        pure function subset_membership(a: Set<Int>, b: Set<Int>, x: Int) -> Bool
            require { std::set::subset(a, b) && std::set::contains(a, x) }
            ensure { result == std::set::contains(b, x) }
        {
            std::set::contains(b, x)
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
            entry.get("function").and_then(|v| v.as_str()) == Some("subset_membership")
                && entry.get("vc_id").and_then(|v| v.as_str()) == Some("vc:0")
        })
        .expect("subset_membership vc:0");
    let assumptions = vc
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        assumptions.is_empty(),
        "set subset/membership VC must be assumption-free"
    );
    let smt2 = vc
        .get("vc")
        .and_then(|v| v.get("smt2"))
        .and_then(|v| v.as_str())
        .expect("vc smt2");
    assert!(
        smt2.contains("(forall ((a Int) (b Int)) (= (|std::set::subset| a b)"),
        "subset VCs must include finite-set subset axiom"
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
        .find(|f| f.name == "subset_membership")
        .and_then(|f| f.vcs.iter().find(|vc| vc.vc_id == "vc:0"))
        .and_then(|vc| vc.assumptions.as_ref());
    assert!(
        vc_assumptions
            .map(|assumptions| assumptions.items.is_empty())
            .unwrap_or(true),
        "proof section must also keep set subset/membership VC assumption-free"
    );
}

#[test]
fn build_set_algebra_vcs_have_zero_assumptions() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("set_algebra.clear");
    let wasm_path = tmp.path().join("set_algebra.wasm");
    let vcs_path = tmp.path().join("set_algebra.vc.json");
    let src = r#"
        pure function set_algebra(a: Set<Int>, b: Set<Int>) -> Bool
            ensure { std::set::subset(std::set::intersect(a, b), std::set::union(a, b)) }
            ensure { result == std::set::subset(std::set::diff(a, b), a) }
        {
            std::set::subset(std::set::diff(a, b), a)
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
            entry.get("function").and_then(|v| v.as_str()) == Some("set_algebra")
                && entry.get("vc_id").and_then(|v| v.as_str()) == Some("vc:0")
        })
        .expect("set_algebra vc:0");
    let assumptions = vc
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(assumptions.is_empty(), "set algebra VC must be assumption-free");
    let smt2 = vc
        .get("vc")
        .and_then(|v| v.get("smt2"))
        .and_then(|v| v.as_str())
        .expect("vc smt2");
    assert!(
        smt2.contains(
            "(forall ((a Int) (b Int) (x Int)) (= (|std::set::contains| (|std::set::union| a b) x)"
        ),
        "set algebra VCs must include union membership axiom"
    );
}

#[test]
fn build_set_cardinality_vcs_have_zero_assumptions() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("set_cardinality.clear");
    let wasm_path = tmp.path().join("set_cardinality.wasm");
    let vcs_path = tmp.path().join("set_cardinality.vc.json");
    let src = r#"
        pure function set_cardinality(a: Set<Int>, b: Set<Int>) -> Bool
            ensure { std::set::len(std::set::intersect(a, b)) <= std::set::len(a) }
            ensure { std::set::len(std::set::union(a, b)) >= std::set::len(a) }
            ensure { std::set::len(std::set::diff(a, b)) <= std::set::len(a) }
        {
            true
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
            entry.get("function").and_then(|v| v.as_str()) == Some("set_cardinality")
                && entry.get("vc_id").and_then(|v| v.as_str()) == Some("vc:0")
        })
        .expect("set_cardinality vc:0");
    let assumptions = vc
        .get("assumptions")
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        assumptions.is_empty(),
        "set cardinality VC must be assumption-free"
    );
    let smt2 = vc
        .get("vc")
        .and_then(|v| v.get("smt2"))
        .and_then(|v| v.as_str())
        .expect("vc smt2");
    assert!(
        smt2.contains(
            "(forall ((a Int) (b Int)) (<= (|std::set::len| (|std::set::intersect| a b)) (|std::set::len| a)))"
        ),
        "set cardinality VCs must include intersect cardinality axiom"
    );
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
