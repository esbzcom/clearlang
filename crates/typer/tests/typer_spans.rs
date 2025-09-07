use lumi_parser::parse;
use lumi_typer::check;

// Ensure typer errors include a span indicator `at start..end`
#[test]
fn errors_include_span_unknown_var() {
    let src = r#"
        pure function f(a: Int) -> Int { a + b }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail unknown variable");
    let s = format!("{err:#}");
    assert!(s.contains("unknown variable"));
    assert!(s.contains("at "));
    assert!(s.contains(".."));
}

#[test]
fn errors_include_span_call_arity() {
    let src = r#"
        pure function add(x: Int, y: Int) -> Int { x + y }
        function main() -> Int { add(1) }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail arity mismatch");
    let s = format!("{err:#}");
    assert!(s.contains("arity mismatch"));
    assert!(s.contains("at "));
    assert!(s.contains(".."));
}
