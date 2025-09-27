use clg_parser::parse;
use clg_typer::type_check_only;

#[test]
fn typechecks_if_let_option() {
    let src = r#"
        pure function unwrap(opt: Option<Int>) -> Int {
            if let Some(v) = opt { v } else { 0 }
        }
    "#;
    let ast = parse(src).expect("parsed");
    type_check_only(&ast).expect("type-check if let");
}

#[test]
fn typechecks_option_coalesce() {
    let src = r#"
        pure function default_age(age: Option<Int>) -> Int { age ?? 18 }
    "#;
    let ast = parse(src).expect("parsed");
    type_check_only(&ast).expect("type-check coalesce");
}

#[test]
fn typechecks_option_try() {
    let src = r#"
        pure function inc(opt: Option<Int>) -> Option<Int> { Some(opt? + 1) }
    "#;
    let ast = parse(src).expect("parsed");
    type_check_only(&ast).expect("type-check try");
}

#[test]
fn try_on_non_option_errors() {
    let src = r#"
        pure function bad(x: Int) -> Int { x? }
    "#;
    let ast = parse(src).expect("parsed");
    let err = type_check_only(&ast).expect_err("non-option try should fail");
    assert!(format!("{err:#}").contains("T602"));
}

#[test]
fn try_requires_matching_return_option() {
    let src = r#"
        pure function bad(opt: Option<Int>) -> Int { opt? }
    "#;
    let ast = parse(src).expect("parsed");
    let err = type_check_only(&ast).expect_err("try return mismatch");
    assert!(format!("{err:#}").contains("T601"));
}

#[test]
fn try_option_inner_mismatch_errors() {
    let src = r#"
        pure function mismatch(opt: Option<Int>) -> Option<Bool> {
            if opt? > 0 { Some(true) } else { Some(false) }
        }
    "#;
    let ast = parse(src).expect("parsed");
    let err = type_check_only(&ast).expect_err("try inner mismatch");
    assert!(format!("{err:#}").contains("T603"));
}
#[test]
fn typechecks_if_let_result_ok() {
    let src = r#"
        pure function unwrap(res: Result<Int, String>) -> Int {
            if let Ok(v) = res { v } else { 0 }
        }
    "#;
    let ast = parse(src).expect("parsed");
    type_check_only(&ast).expect("type-check if let result ok");
}

#[test]
fn typechecks_if_let_result_err() {
    let src = r#"
        pure function handle(res: Result<Int, String>) -> Result<Int, String> {
            if let Err(e) = res { Err(e) } else { res }
        }
    "#;
    let ast = parse(src).expect("parsed");
    type_check_only(&ast).expect("type-check if let result err");
}

#[test]
fn typechecks_result_try() {
    let src = r#"
        pure function inc(res: Result<Int, String>) -> Result<Int, String> {
            Ok(res? + 1)
        }
    "#;
    let ast = parse(src).expect("parsed");
    type_check_only(&ast).expect("type-check result try");
}

#[test]
fn try_result_requires_result_return() {
    let src = r#"
        pure function bad(res: Result<Int, String>) -> Option<Int> { res? }
    "#;
    let ast = parse(src).expect("parsed");
    let err = type_check_only(&ast).expect_err("result try return mismatch");
    assert!(format!("{err:#}").contains("T604"));
}

#[test]
fn try_result_mismatch_errors() {
    let src = r#"
        pure function mismatch(res: Result<Int, String>) -> Result<Int, Bool> {
            Ok(res? + 1)
        }
    "#;
    let ast = parse(src).expect("parsed");
    let err = type_check_only(&ast).expect_err("result try mismatch");
    assert!(format!("{err:#}").contains("T605"));
}
