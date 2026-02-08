use super::*;

#[test]
fn run_option_result_success_paths() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("variants_success.clear");
    let wasm_path = tmp.path().join("variants_success.wasm");

    let src = r#"
        pure function inc(opt: Option<Int>) -> Option<Int> { Some(opt? + 1) }
        pure function twice(res: Result<Int, Int>) -> Result<Int, Int> { Ok(res? * 2) }
        pure function seed_opt() -> Option<Int> { Some(41) }
        pure function seed_res() -> Result<Int, Int> { Ok(21) }
        function main() -> Int {
            if let Some(v) = inc(seed_opt()) {
                if let Ok(w) = twice(seed_res()) { v + w } else { 0 }
            } else { 0 }
        }
    "#;
    fs::write(&src_path, src.trim()).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(&wasm_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("84\n"));
}

#[test]
fn run_option_result_propagation_paths() {
    let tmp = tempdir().unwrap();
    let src_path = tmp.path().join("variants_propagation.clear");
    let wasm_path = tmp.path().join("variants_propagation.wasm");

    let src = r#"
        pure function inc(opt: Option<Int>) -> Option<Int> { Some(opt? + 1) }
        pure function guard(res: Result<Int, Int>) -> Result<Int, Int> { Ok(res? + 5) }
        pure function none() -> Option<Int> { None }
        pure function err_res() -> Result<Int, Int> { Err(9) }
        function main() -> Int {
            if let Some(v) = inc(none()) {
                v
            } else {
                if let Ok(w) = guard(err_res()) { w } else { 17 }
            }
        }
    "#;
    fs::write(&src_path, src.trim()).expect("write source");

    Command::cargo_bin("clg")
        .unwrap()
        .args(["build"])
        .arg(&src_path)
        .args(["-o"])
        .arg(&wasm_path)
        .assert()
        .success();

    Command::cargo_bin("clg")
        .unwrap()
        .args(["run"])
        .arg(&wasm_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("17\n"));
}

#[test]
fn run_reports_r003_invalid_variant_json() {
    use clg_codegen_wasm::emit_from_ir;
    use clg_ir::{
        Function as IrFunction, Instr as IrInstr, IrType, Module as IrModule, Value as IrValue,
        VariantKind,
    };

    let main_fn = IrFunction {
        name: "main".to_string(),
        params: vec![],
        ret: Some(IrType::Int),
        body: vec![
            IrInstr::IConst {
                dst: IrValue(0),
                ty: IrType::Int,
                n: 2,
            },
            IrInstr::IConst {
                dst: IrValue(1),
                ty: IrType::Int,
                n: 0,
            },
            IrInstr::IConst {
                dst: IrValue(2),
                ty: IrType::Int,
                n: 0,
            },
            IrInstr::VariantInit {
                dst: IrValue(3),
                tag: IrValue(0),
                payload_lo: IrValue(1),
                payload_hi: IrValue(2),
            },
            IrInstr::VariantLoadTag {
                dst: IrValue(4),
                variant: IrValue(3),
                kind: VariantKind::Option,
            },
            IrInstr::Ret { val: IrValue(4) },
        ],
    };
    let ir = IrModule {
        funcs: vec![main_fn],
    };
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let tmp = tempdir().unwrap();
    let wasm_path = tmp.path().join("invalid_tag.wasm");
    fs::write(&wasm_path, wasm).expect("write wasm");

    let output = Command::cargo_bin("clg")
        .unwrap()
        .args(["--json-errors", "run"])
        .arg(&wasm_path)
        .output()
        .expect("run command");
    assert!(
        !output.status.success(),
        "invalid variant tag should trap the runtime"
    );
    let v: Value = serde_json::from_slice(&output.stdout).expect("json parse");
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(false));
    let errs = v
        .get("errors")
        .and_then(|e| e.as_array())
        .expect("errors array");
    assert!(!errs.is_empty());
    let first = &errs[0];
    assert_eq!(first.get("code").and_then(|s| s.as_str()), Some("R003"));
    assert_eq!(first.get("stage").and_then(|s| s.as_str()), Some("runtime"));
    let msg = first.get("message").and_then(|s| s.as_str()).unwrap_or("");
    assert!(
        msg.contains("Option"),
        "message should mention the Option kind"
    );
    assert!(
        msg.contains("tag 2"),
        "message should report the invalid tag"
    );
}
