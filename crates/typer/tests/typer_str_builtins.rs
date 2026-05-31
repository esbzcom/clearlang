use clg_parser::parse;
use clg_typer::check;

#[test]
fn typer_accepts_std_str_len() {
    let src = r#"
        pure function size(s: String) -> Int { std::str::len(s) }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

#[test]
fn typer_accepts_std_str_concat() {
    let src = r#"
        pure function greet(name: String) -> String { std::str::concat("Hello, ", name) }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

#[test]
fn typer_accepts_std_str_eq() {
    let src = r#"
        pure function same(a: String, b: String) -> Bool { std::str::eq(a, b) }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

#[test]
fn typer_accepts_std_str_search_helpers() {
    let src = r#"
        pure function ok(a: String, b: String) -> Bool {
            std::str::starts_with(a, b)
                || std::str::ends_with(a, b)
                || std::str::contains(a, b)
                || std::str_pattern::matches(b, a)
        }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

#[test]
fn typer_accepts_std_str_to_bytes() {
    let src = r#"
        pure function ok(a: String) -> Int { std::bytes::len(std::str::to_bytes(a)) }
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
    assert!(s.contains("String"));
}

#[test]
fn typer_rejects_std_str_concat_arity() {
    let src = r#"
        pure function bad() -> String { std::str::concat("hi") }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = check(&ast).expect_err("should fail arity mismatch for concat");
    assert!(format!("{err:#}").contains("arity mismatch"));
}

#[test]
fn typer_rejects_std_str_pattern_matches_wrong_arg_type() {
    let src = r#"
        pure function bad() -> Bool { std::str_pattern::matches(123, "abc") }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = check(&ast).expect_err("should fail type mismatch for str_pattern::matches");
    let s = format!("{err:#}");
    assert!(s.contains("type mismatch"));
    assert!(s.contains("String"));
}

#[test]
fn typer_rejects_std_str_pattern_matches_arity() {
    let src = r#"
        pure function bad() -> Bool { std::str_pattern::matches("abc") }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = check(&ast).expect_err("should fail arity mismatch for str_pattern::matches");
    assert!(format!("{err:#}").contains("arity mismatch"));
}
