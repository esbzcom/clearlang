use lumi_parser::parse;
use lumi_typer::check;

// Phase 3.1 — purpose: successful type-check on Bool-returning function
#[test]
fn accepts_bool_return_function() {
    let src = r#"
        pure fn t() -> Bool { true }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

// Phase 3.1 — purpose: detect return type mismatch (declared Bool, body Int)
#[test]
fn errors_on_return_type_mismatch() {
    let src = r#"
        pure fn bad() -> Bool { 1 }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail return type mismatch");
    assert!(format!("{err:#}").contains("return type mismatch"));
}

// Phase 3.1 — purpose: detect unknown variable usage in expressions
#[test]
fn errors_on_unknown_variable() {
    let src = r#"
        pure fn f(a: Int) -> Int { a + b }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail unknown variable");
    assert!(format!("{err:#}").contains("unknown variable"));
}

// Phase 3.1 — purpose: detect call argument type mismatch
#[test]
fn errors_on_call_arg_type_mismatch() {
    let src = r#"
        pure fn id(x: Int) -> Int { x }
        fn main() -> Int { id(true) }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail arg type mismatch");
    assert!(format!("{err:#}").contains("type mismatch"));
}

// Phase 3.1 — purpose: detect duplicate parameter names
#[test]
fn errors_on_duplicate_parameter_names() {
    let src = r#"
        pure fn f(a: Int, a: Int) -> Int { a }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail duplicate parameter");
    assert!(format!("{err:#}").contains("duplicate parameter"));
}

// Phase 3.1 — purpose: recursion is allowed by the typer (no totality yet)
#[test]
fn accepts_simple_recursion_typewise() {
    let src = r#"
        pure fn loop1(n: Int) -> Int { loop1(n) }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

