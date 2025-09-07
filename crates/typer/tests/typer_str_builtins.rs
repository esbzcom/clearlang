use lumi_parser::parse;
use lumi_typer::check;

#[test]
fn typer_accepts_std_str_len() {
    let src = r#"
        pure function size(s: Str) -> Int { std::str::len(s) }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

#[test]
fn typer_accepts_std_str_concat() {
    let src = r#"
        pure function greet(name: Str) -> Str { std::str::concat("Hello, ", name) }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

#[test]
fn typer_accepts_std_str_eq() {
    let src = r#"
        pure function same(a: Str, b: Str) -> Bool { std::str::eq(a, b) }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

#[test]
fn typer_rejects_std_str_len_wrong_arg_type() {
    let src = r#"
        pure function bad() -> Int { std::str::len(123) }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = check(&ast).expect_err("should fail type mismatch for len");
    let s = format!("{err:#}");
    assert!(s.contains("type mismatch"));
    assert!(s.contains("Str"));
}

#[test]
fn typer_rejects_std_str_concat_arity() {
    let src = r#"
        pure function bad() -> Str { std::str::concat("hi") }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = check(&ast).expect_err("should fail arity mismatch for concat");
    assert!(format!("{err:#}").contains("arity mismatch"));
}
