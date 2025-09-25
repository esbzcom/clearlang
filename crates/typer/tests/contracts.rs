use clg_parser::parse;
use clg_typer::check;

#[test]
fn contracts_type_check_for_pure_functions() {
    let src = r#"
        pure function bounded_inc(x: Int) -> Int
            require { 0 <= x && x < 10 }
            ensure { result > x }
        { x + 1 }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

#[test]
fn contracts_reject_non_bool_ensure() {
    let src = r#"
        pure function bad(x: Int) -> Int
            ensure { x }
        { x }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = check(&ast).expect_err("should fail ensure predicate");
    let msg = format!("{err:#}");
    assert!(msg.contains("ensure"));
    assert!(msg.contains("Bool"));
}

#[test]
fn contracts_reject_non_bool_predicates() {
    let src = r#"
        pure function bad(x: Int) -> Int
            require { x }
        { x }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = check(&ast).expect_err("should fail require predicate");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("require"),
        "expected error about require being Bool, got {msg}"
    );
    assert!(msg.contains("Bool"));
}

#[test]
fn contracts_allow_mut_effect() {
    let src = r#"
        mut function ok(x: Int) -> Int
            require { true }
        { x }
    "#;
    check(&parse(src).expect("parse ok")).expect("mut functions are allowed");
}

#[test]
fn contracts_reject_mismatched_comparison_operands() {
    let src = r#"
        pure function bad(x: Int) -> Int
            ensure { x == true }
        { x }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = check(&ast).expect_err("mismatched operands should fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("operands of `==` must have the same type"),
        "expected equality operand mismatch error, got {msg}"
    );
}
