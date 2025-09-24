use clg_parser::parse;
use clg_typer::{type_check_only, TyperError};

fn type_ok(src: &str) {
    let ast = parse(src).expect("parse");
    type_check_only(&ast).expect("type ok");
}

fn type_err_code(src: &str, code: &str) {
    let ast = parse(src).expect("parse");
    let err = type_check_only(&ast).expect_err("expected type error");
    let te = err.downcast_ref::<TyperError>().expect("typer error");
    assert_eq!(te.code, code);
}

#[test]
fn option_match_types() {
    let src = r#"
        pure function f(m: Option<Int>) -> Int {
            match m { Some(x) => x, None => 0 }
        }
    "#;
    type_ok(src);
}

#[test]
fn result_match_types() {
    let src = r#"
        function g(r: Result<Int, Int>) -> Int {
            match r { Ok(v) => v, Err(e) => 0 }
        }
    "#;
    type_ok(src);
}

#[test]
fn non_exhaustive_match() {
    let src = r#"
        function f(m: Option<Int>) -> Int { match m { Some(x) => x } }
    "#;
    type_err_code(src, "T201");
}

#[test]
fn duplicate_arm() {
    let src = r#"
        function f(m: Option<Int>) -> Int { match m { None => 0, None => 1 } }
    "#;
    type_err_code(src, "T202");
}

#[test]
fn invalid_scrutinee() {
    let src = r#"
        function f(n: Int) -> Int { match n { Some(x) => x, None => 0 } }
    "#;
    type_err_code(src, "T203");
}

#[test]
fn arm_type_mismatch() {
    let src = r#"
        function f(m: Option<Int>) -> Int { match m { Some(x) => x, None => true } }
    "#;
    type_err_code(src, "T204");
}

#[test]
fn binder_conflict() {
    let src = r#"
        function f(x: Int) -> Int {
            match Some(x) { Some(x) => x, None => 0 }
        }
    "#;
    type_err_code(src, "T205");
}

#[test]
fn some_constructor_infers_option() {
    let src = r#"
        function h() -> Option<Int> { Some(1) }
    "#;
    type_ok(src);
}

#[test]
fn option_id_function() {
    let src = r#"
        pure function id_opt(x: Option<String>) -> Option<String> { x }
    "#;
    type_ok(src);
}

