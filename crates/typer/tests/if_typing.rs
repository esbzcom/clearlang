use lumi_parser::parse;
use lumi_typer::{type_check_only, TyperError};

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
fn if_branches_unify() {
    let src = r#"
        pure function f(b: Bool) -> Int {
            if b { 1 } else { 2 }
        }
    "#;
    type_ok(src);
}

#[test]
fn if_branch_mismatch_errors() {
    let src = r#"
        function g(b: Bool) -> Int {
            if b { 1 } else { true }
        }
    "#;
    type_err_code(src, "T301");
}

#[test]
fn non_bool_condition_errors() {
    let src = r#"
        function f() -> Int { if 1 { 1 } else { 2 } }
    "#;
    type_err_code(src, "T003");
}

#[test]
fn non_bool_condition_in_else_if_errors() {
    let src = r#"
        function f(b: Bool) -> Int { if b { 1 } else if 1 { 2 } else { 3 } }
    "#;
    type_err_code(src, "T003");
}

#[test]
fn unify_string_branches() {
    let src = r#"
        pure function f(b: Bool) -> String { if b { "a" } else { "b" } }
    "#;
    type_ok(src);
}

#[test]
fn unify_option_branches() {
    let src = r#"
        pure function f(b: Bool) -> Option<Int> { if b { Some(1) } else { Some(0) } }
    "#;
    type_ok(src);
}

#[test]
fn chain_unify_ints() {
    let src = r#"
        pure function f(b1: Bool, b2: Bool) -> Int { if b1 { 1 } else if b2 { 1 } else { 1 } }
    "#;
    type_ok(src);
}
