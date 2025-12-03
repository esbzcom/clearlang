use clg_parser::parse;
use clg_typer::check;

#[test]
fn alias_resolves_in_params_and_returns() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function id(n: Nat) -> Nat { n }
        function main() -> Int { id(1) }
    "#;
    check(&parse(src).expect("parse ok")).expect("typecheck ok");
}

#[test]
fn predicate_must_be_bool() {
    let src = r#"
        type Bad = Int where 1;
        pure function id(x: Bad) -> Int { x }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("predicate not bool");
    let s = format!("{err:#}");
    assert!(s.contains("T704"), "missing code T704: {s}");
}

#[test]
fn cyclic_alias_is_rejected() {
    let src = r#"
        type A = B where a >= 0;
        type B = A where b >= 0;
        pure function f(x: A) -> A { x }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("cyclic alias");
    let s = format!("{err:#}");
    assert!(s.contains("T703"), "missing code T703: {s}");
}

#[test]
fn alias_cannot_shadow_resource() {
    let src = r#"
        resource File { fd: Int; drop {} }
        type File = Int where f >= 0;
        pure function f(fd: File) -> Int { fd }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("resource conflict");
    let s = format!("{err:#}");
    assert!(s.contains("T702"), "missing code T702: {s}");
}

#[test]
fn alias_to_resource_collection_is_rejected() {
    let src = r#"
        resource R { v: Int; drop {} }
        type Bad = List<R> where xs > 0;
        pure function f(xs: Bad) -> Int { 0 }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("resource in collection");
    let s = format!("{err:#}");
    assert!(s.contains("T806"), "missing code T806: {s}");
}
