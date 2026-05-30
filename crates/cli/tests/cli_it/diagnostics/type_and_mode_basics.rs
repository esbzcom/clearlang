#[test]
fn type_error_reports_json_with_span_and_code() {
    // Create a small source with a type mismatch: add(1, true)
    let src = r#"
        pure function add(x: Int, y: Int) -> Int { x + y }
        function main() -> Int { add(1, true) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert_eq!(errs.len(), 1);
    let e0 = &errs[0];
    // Arg type mismatch should map to T003
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T003"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
    // Span presence
    assert!(e0.get("start").and_then(|n| n.as_u64()).is_some());
    assert!(e0.get("end").and_then(|n| n.as_u64()).is_some());
}

#[test]
fn unsigned_literal_out_of_range_reports_t112_in_json() {
    let src = r#"
        function main() -> U8 { U8(300) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_u8.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T112"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn unsigned_constant_overflow_reports_t113_in_json() {
    let src = r#"
        function main() -> U64 { U64(9223372036854775807) * U64(3) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_u64_overflow.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T113"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn array_index_out_of_bounds_reports_t114_in_json() {
    let src = r#"
        function main() -> Int { [1, 2][3] }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_array_index.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T114"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn tuple_index_requires_constant_reports_t115_in_json() {
    let src = r#"
        function main() -> Int { let i = 1; (1, 2, 3)[i] }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_tuple_index.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T115"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn collections_error_reports_collection_kind_error_in_json() {
    // Calling a collection API with wrong kind should yield T207 via --json-errors
    let src = r#"
        function main() -> Int { std::map::len(0) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_collections.clear");
    std::fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errs.is_empty());
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T207"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn if_branch_mismatch_reports_t301_in_json() {
    let src = r#"
        function main() -> Int { if true { 1 } else { false } }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("bad_if.clear");
    std::fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errs.is_empty());
    let e0 = &errs[0];
    assert_eq!(e0.get("code").and_then(|s| s.as_str()), Some("T301"));
    assert_eq!(e0.get("stage").and_then(|s| s.as_str()), Some("type"));
}

#[test]
fn migration_deferred_ergonomics_type_restrictions_report_stable_codes() {
    let cases = [
        ("migration/02_interface_type_params.clear", "T246"),
        (
            "migration/03_implementation_method_type_params.clear",
            "T245",
        ),
        ("migration/05_set_resource.clear", "T806"),
        ("migration/06_array_resource.clear", "T806"),
    ];

    for (file, expected_code) in cases {
        let tmp = tempdir().unwrap();
        let out = tmp.path().join("out.wasm");
        let mut cmd = Command::cargo_bin("clg").unwrap();
        cmd.args(["--json-errors", "build"])
            .arg(repo_sample(file))
            .args(["-o"])
            .arg(&out);
        let output = cmd.assert().failure().get_output().stdout.clone();
        let v: Value = serde_json::from_slice(&output).expect("json");
        assert_single_json_error(&v, expected_code, "type");
    }
}

#[test]
fn migration_refinement_ergonomics_samples_now_pass() {
    Command::cargo_bin("clg")
        .unwrap()
        .args(["parse"])
        .arg(repo_sample("migration/01_inline_refinement_param.clear"))
        .assert()
        .success();

    let tmp = tempdir().unwrap();
    let out = tmp.path().join("out.wasm");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(repo_sample("migration/04_generic_refinement_alias.clear"))
        .args(["-o"])
        .arg(&out)
        .assert()
        .success();
}

#[test]
fn strict_compiler_mode_requires_emit_vcs_with_c029() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_mode_no_vc.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C029", "build");
}

#[test]
fn precompiled_std_core_mode_requires_strict_compiler_mode_with_c035() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("precompiled_mode_non_strict.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--std-core-link-mode", "precompiled"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C035", "build");
}

#[test]
fn production_release_profile_requires_strict_compiler_mode_with_c120() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("production_release_non_strict.clear");
    fs::write(&file, src).expect("write");
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "standard"])
        .args(["--release-profile", "production"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C120", "build");
}

#[test]
fn production_release_profile_requires_proved_all_with_c121() {
    let src = r#"
        function main() -> Int { 0 }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("production_release_not_proved_all.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--release-profile", "production"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C121", "build");
}

#[test]
fn production_release_profile_rejects_module_graph_test_paths_with_c128() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let file = root.join("main.clear");
    fs::create_dir_all(root.join("tests").join("unit")).expect("create tests/unit");
    fs::write(
        &file,
        r#"
        import tests::unit::helper
        function main() -> Int { helper::value() }
    "#,
    )
    .expect("write");
    fs::write(
        root.join("tests").join("unit").join("helper.clear"),
        "export function value() -> Int { 1 }\n",
    )
    .expect("write helper");
    write_minimal_strict_preflight_files(root);
    let out = root.join("out.wasm");
    let vcs = root.join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--release-profile", "production"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C128", "build");
}

#[test]
fn strict_mode_reports_c124_when_solver_is_unavailable() {
    let src = r#"
        pure function inc(x: Int) -> Int
            require { x >= 0 }
            ensure { result >= 0 }
        { x + 1 }
        function main() -> Int { inc(1) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_solver_unavailable.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let missing_solver = if cfg!(windows) {
        tmp.path().join("missing-z3.exe")
    } else {
        tmp.path().join("missing-z3")
    };

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.env("CLG_SOLVER_BIN", &missing_solver)
        .args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C124", "build");
}

#[test]
fn strict_mode_reports_c125_when_solver_times_out() {
    let src = r#"
        pure function inc(x: Int) -> Int
            require { x >= 0 }
            ensure { result >= 0 }
        { x + 1 }
        function main() -> Int { inc(1) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_solver_timeout.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let solver = write_fake_timeout_solver(tmp.path());
    write_solver_integrity_sidecars(&solver);

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.env("CLG_SOLVER_BIN", &solver)
        .args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C125", "build");
}

#[test]
fn production_release_reports_c127_on_solver_replay_mismatch() {
    let src = r#"
        pure function inc(x: Int) -> Int
            require { x >= 0 }
            ensure { result >= 0 }
        { x + 1 }
        function main() -> Int { inc(1) }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("production_solver_replay_mismatch.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());
    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let solver = write_fake_flaky_solver(tmp.path());
    write_solver_integrity_sidecars(&solver);
    let counter = tmp.path().join("solver-counter.txt");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.env("CLG_SOLVER_BIN", &solver)
        .env("CLG_FAKE_Z3_COUNTER_FILE", &counter)
        .args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--release-profile", "production"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C127", "build");
}

#[test]
fn production_release_profile_rejects_non_proved_std_surface_with_c122() {
    let src = r#"
        function main() -> Int { std::str::len("abc") }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("production_release_disallowed_surface.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": null
        }
      ]"#,
    );

    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let matrix_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/proofs/proof-coverage-matrix.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.env("CLG_PROOF_MATRIX_PATH", matrix_path)
        .args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--std-core-link-mode", "precompiled"])
        .args(["--release-profile", "production"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C122", "build");
}

#[test]
fn production_release_profile_reports_matrix_load_failures_as_c122() {
    let src = r#"
        function main() -> Int { std::str::len("abc") }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("production_release_matrix_missing.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": null
        }
      ]"#,
    );

    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let matrix_path = tmp.path().join("missing-proof-matrix.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.env("CLG_PROOF_MATRIX_PATH", matrix_path)
        .args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--std-core-link-mode", "precompiled"])
        .args(["--release-profile", "production"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C122", "build");
}

#[test]
fn production_release_profile_rejects_crypto_assumption_boundary_with_c123() {
    let src = r#"
        pure function check(a: Bytes, b: Bytes) -> Bool
            ensure { result == std::bytes::eq_ct(a, b) }
        { std::bytes::eq_ct(a, b) }
        function main() -> Int {
            if check(std::bytes::from_string("a"), std::bytes::from_string("b")) { 1 } else { 0 }
        }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("production_release_crypto_boundary.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());

    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["--json-errors", "build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--release-profile", "production"]);
    let output = cmd.assert().failure().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_single_json_error(&v, "C123", "build");
}

#[test]
fn strict_compiler_mode_accepts_list_proofs_with_zero_assumptions() {
    let src = r#"
        pure function empty_list() -> List<Int> { std::list::new() }

        pure function list_gate_demo(l: List<Int>, x: Int) -> Bool
            ensure { result == result }
        {
            let pushed = std::list::push(l, x);
            let inserted = std::list::insert(pushed, x, 0);
            let removed = std::list::remove(inserted, 0);
            std::list::len(std::list::remove_take(removed, 0)[0]) >= 0
        }

        function main() -> Int {
            if list_gate_demo(std::list::push(empty_list(), 7), 9) { 1 } else { 0 }
        }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("production_release_list_zero_assumptions.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());

    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"]);
    cmd
        .assert()
        .success();

    let data = fs::read_to_string(&vcs).expect("read vcs");
    let items: Value = serde_json::from_str(&data).expect("json array");
    let arr = items.as_array().expect("array");
    let list_vcs: Vec<&Value> = arr
        .iter()
        .filter(|entry| {
            entry.get("function").and_then(|v| v.as_str()) == Some("list_gate_demo")
        })
        .collect();
    assert!(
        !list_vcs.is_empty(),
        "expected list_gate_demo VC rows in strict-mode vcs output"
    );
    for vc in list_vcs {
        let assumptions = vc
            .get("assumptions")
            .and_then(|v| v.get("items"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(
            assumptions.is_empty(),
            "strict production list VCs must remain assumption-free"
        );
    }
}

#[test]
fn strict_compiler_mode_accepts_map_proofs_with_zero_assumptions() {
    let src = r#"
        pure function empty_map() -> Map<Int, Int> { std::map::new() }

        pure function map_gate_demo(m: Map<Int, Int>, k: Int, v: Int) -> Bool
            ensure { result == result }
        {
            let inserted = std::map::insert(m, k, v);
            let inserted_take = std::map::insert_take(inserted, k, v + 1);
            let inserted_map = inserted_take[0];
            let removed = std::map::remove(inserted_map, k);
            let removed_take = std::map::remove_take(removed, k);
            let removed_map = removed_take[0];
            if std::map::contains(inserted_map, k) {
                match std::map::get(inserted_map, k) {
                    Some(got) => got >= v && std::map::len(removed_map) >= 0,
                    None => false
                }
            } else {
                false
            }
        }

        function main() -> Int {
            if map_gate_demo(std::map::insert(empty_map(), 1, 7), 1, 9) { 1 } else { 0 }
        }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("production_release_map_zero_assumptions.clear");
    fs::write(&file, src).expect("write");
    write_minimal_strict_preflight_files(tmp.path());
    let solver_path = if cfg!(windows) {
        tmp.path().join("fake-z3.exe")
    } else {
        tmp.path().join("fake-z3")
    };
    let solver = write_fake_solver_to(
        &solver_path,
        r#"
use std::io::{self, Read};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-version") {
        println!("Z3 version 4.16.0 - fake");
        return;
    }
    let mut stdin = Vec::new();
    let _ = io::stdin().read_to_end(&mut stdin);
    println!("unsat");
}
"#,
    );
    write_solver_integrity_sidecars(&solver);

    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    let matrix_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/proofs/proof-coverage-matrix.json");

    let mut cmd = Command::cargo_bin("clg").unwrap();
    cmd.env("CLG_SOLVER_BIN", &solver)
        .env("CLG_PROOF_MATRIX_PATH", matrix_path)
        .args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--release-profile", "production"]);
    cmd
        .assert()
        .success();

    let data = fs::read_to_string(&vcs).expect("read vcs");
    let items: Value = serde_json::from_str(&data).expect("json array");
    let arr = items.as_array().expect("array");
    let map_vcs: Vec<&Value> = arr
        .iter()
        .filter(|entry| {
            entry.get("function").and_then(|v| v.as_str()) == Some("map_gate_demo")
        })
        .collect();
    assert!(
        !map_vcs.is_empty(),
        "expected map_gate_demo VC rows in strict-mode vcs output"
    );
    for vc in map_vcs {
        let assumptions = vc
            .get("assumptions")
            .and_then(|v| v.get("items"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(
            assumptions.is_empty(),
            "strict production map VCs must remain assumption-free"
        );
    }
}

#[test]
fn strict_acceptance_positive_all_gates_pass_with_signed_fixture() {
    let src = r#"
        function main() -> Int { std::str::len("abc") }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("strict_acceptance_ok.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": null
        }
      ]"#,
    );
    run_strict_build_success(tmp.path(), &file);
}

#[test]
fn strict_acceptance_precompiled_std_core_symbol_links_via_package_import() {
    let src = r#"
        function main() -> Int { std::str::len("abc") }
    "#;
    let tmp = tempdir().unwrap();
    let file = tmp
        .path()
        .join("strict_acceptance_precompiled_std_core_link.clear");
    fs::write(&file, src).expect("write");
    write_signed_strict_dependency_fixture(
        tmp.path(),
        r#"[
        {
          "symbol": "std::str::len",
          "effect": "pure",
          "params": ["String"],
          "ret": "Int",
          "capability": null
        }
      ]"#,
    );

    let out = tmp.path().join("out.wasm");
    let vcs = tmp.path().join("out.vc.json");
    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&file)
        .args(["-o"])
        .arg(&out)
        .args(["--emit-vcs"])
        .arg(&vcs)
        .args(["--compiler-mode", "strict"])
        .args(["--std-core-link-mode", "precompiled"])
        .assert()
        .success();

    let wasm = fs::read(&out).expect("read wasm");
    let import_index = wasm_import_func_index(&wasm, "std::str", "len")
        .expect("expected std::str::len package import in precompiled mode");
    assert!(
        wasm_calls_function_index(&wasm, import_index),
        "expected Wasm code to call imported std::str::len (not intrinsic-only lowering)"
    );
}

