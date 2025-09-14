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

