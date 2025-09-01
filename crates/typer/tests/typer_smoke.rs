use lumi_parser::parse;
use lumi_typer::check;

#[test]
fn accepts_simple_add_and_main() {
    let src = r#"
        pure fn add(a: Int, b: Int) -> Int { a + b }
        fn main() -> Int { add(20, 22) }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

#[test]
fn typer_errors_on_unknown_function() {
    let src = r#"
        fn main() -> Int { missing(1, 2) }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail unknown function");
    let s = format!("{err:#}");
    assert!(s.contains("unknown function"));
}

#[test]
fn typer_errors_on_arity_mismatch() {
    let src = r#"
        pure fn add(x: Int, y: Int) -> Int { x + y }
        fn main() -> Int { add(1) }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail arity check");
    assert!(format!("{err:#}").contains("arity mismatch"));
}

#[test]
fn typer_errors_on_type_mismatch_in_binop() {
    let src = r#"
        pure fn bad() -> Int { true + 1 }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail type mismatch");
    assert!(format!("{err:#}").contains("must be Int"));
}

