use clg_parser::parse;
use clg_typer::check;

#[test]
fn accepts_simple_add_and_main() {
    let src = r#"
        pure function add(a: Int, b: Int) -> Int { a + b }
        function main() -> Int { add(20, 22) }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

#[test]
fn typer_errors_on_unknown_function() {
    let src = r#"
        function main() -> Int { missing(1, 2) }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail unknown function");
    let s = format!("{err:#}");
    assert!(s.contains("unknown function"));
}

#[test]
fn typer_errors_on_arity_mismatch() {
    let src = r#"
        pure function add(x: Int, y: Int) -> Int { x + y }
        function main() -> Int { add(1) }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail arity check");
    assert!(format!("{err:#}").contains("arity mismatch"));
}

#[test]
fn typer_errors_on_type_mismatch_in_binop() {
    let src = r#"
        pure function bad() -> Int { true + 1 }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail type mismatch");
    assert!(format!("{err:#}").contains("must be Int"));
}

#[test]
fn accepts_str_echo() {
    let src = r#"
        pure function echo(s: String) -> String { s }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

#[test]
fn accepts_prefixed_integer_literals() {
    let src = r#"
        function main() -> Int { 0x2A + 0b1_0101 }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

#[test]
fn accepts_contextual_unsigned_prefixed_literals() {
    let src = r#"
        pure function id(x: U64) -> U64 { x }
        function main() -> U64 { id(0xFF) + 0b1 }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}
