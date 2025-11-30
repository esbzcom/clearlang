use clg_parser::parse;
use clg_typer::type_check_only;

fn expect_typer_error(src: &str) -> String {
    let ast = parse(src).expect("parse succeeds");
    let err = type_check_only(&ast).expect_err("type checker should report an error");
    format!("{err:#}")
}

#[test]
fn loop_invariant_must_be_bool() {
    let src = r#"
function main(n: Int) -> Int {
    while true invariant { 1 } {
        n;
    }
    0
}
"#;

    let message = expect_typer_error(src);
    assert!(
        message.contains("T012"),
        "expected bool invariant error, got: {message}"
    );
}

#[test]
fn loop_variant_must_be_int() {
    let src = r#"
function main(n: Int) -> Int {
    while true invariant { true } variant { true } {
        n;
    }
    0
}
"#;

    let message = expect_typer_error(src);
    assert!(
        message.contains("T005"),
        "expected int variant error, got: {message}"
    );
}

#[test]
fn loop_condition_must_be_bool() {
    let src = r#"
function main(n: Int) -> Int {
    while n invariant { true } {
        n;
    }
    0
}
"#;

    let message = expect_typer_error(src);
    assert!(
        message.contains("T012"),
        "expected bool condition error, got: {message}"
    );
}

#[test]
fn loop_with_invariant_and_variant_typechecks() {
    let src = r#"
function main(n: Int) -> Int {
    while n > 0 invariant { n >= 0 } variant { n } {
        n;
    }
    0
}
"#;

    let ast = parse(src).expect("parse succeeds");
    type_check_only(&ast).expect("type check succeeds");
}
