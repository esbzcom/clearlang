use clg_parser::parse;

#[test]
fn parses_while_with_invariant_and_variant() {
    let src = r#"
function main(n: Int) -> Int {
    while n > 0 invariant { n >= 0 } variant { n } {
        n;
    }
    0
}
"#;

    parse(src).expect("parse succeeds");
}

#[test]
fn errors_when_invariant_missing_braces() {
    let src = r#"
function main(n: Int) -> Int {
    while n > 0 invariant n >= 0 {
        n;
    }
    0
}
"#;

    let err = parse(src).expect_err("parse should fail without invariant braces");
    assert!(
        err.to_string().contains("expected '{'"),
        "unexpected parse message: {err}"
    );
}
