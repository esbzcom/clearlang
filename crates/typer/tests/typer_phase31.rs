use clg_parser::parse;
use clg_typer::check;

// Phase 3.1 — purpose: successful type-check on Bool-returning function
#[test]
fn accepts_bool_return_function() {
    let src = r#"
        pure function t() -> Bool { true }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

// Phase 3.1 — purpose: detect return type mismatch (declared Bool, body Int)
#[test]
fn errors_on_return_type_mismatch() {
    let src = r#"
        pure function bad() -> Bool { 1 }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail return type mismatch");
    assert!(format!("{err:#}").contains("return type mismatch"));
}

// Phase 3.1 — purpose: detect unknown variable usage in expressions
#[test]
fn errors_on_unknown_variable() {
    let src = r#"
        pure function f(a: Int) -> Int { a + b }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail unknown variable");
    assert!(format!("{err:#}").contains("unknown variable"));
}

// Phase 3.1 — purpose: detect call argument type mismatch
#[test]
fn errors_on_call_arg_type_mismatch() {
    let src = r#"
        pure function id(x: Int) -> Int { x }
        function main() -> Int { id(true) }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail arg type mismatch");
    assert!(format!("{err:#}").contains("type mismatch"));
}

// Phase 3.1 — purpose: detect duplicate parameter names
#[test]
fn errors_on_duplicate_parameter_names() {
    let src = r#"
        pure function f(a: Int, a: Int) -> Int { a }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail duplicate parameter");
    assert!(format!("{err:#}").contains("duplicate parameter"));
}

// Phase 3.1 — previously allowed recursion; now totality enforcement rejects unmeasured recursion
#[test]
fn accepts_simple_recursion_typewise() {
    let src = r#"
        pure function loop1(n: Int) -> Int { loop1(n) }
    "#;
    let err = check(&parse(src).expect("parse ok"))
        .expect_err("pure recursion without a measure should be rejected");
    assert!(format!("{err:#}").contains("T902"));
}

// Phase 3.2 — purpose: accept None/Pure effects; reject Mut/Io for now
#[test]
fn accepts_none_and_pure_effects() {
    let src = r#"
        function id(x: Int) -> Int { x }
        pure function pid(x: Int) -> Int { x }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

// Phase 3.2 — purpose: reject unsupported effects Mut/Io
#[test]
fn rejects_io_effect_only() {
    let src_mut = r#"
        mut function f(x: Int) -> Int { x }
    "#;
    check(&parse(src_mut).expect("parsed")).expect("mut effect now allowed");

    let src_io = r#"
        io function g(x: Int) -> Int { x }
    "#;
    let err = check(&parse(src_io).expect("parsed")).expect_err("io not allowed");
    assert!(format!("{err:#}").contains("effect `io`"));
}
