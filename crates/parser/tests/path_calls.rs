use clg_parser::parse;

#[test]
fn parses_namespaced_call() {
    let src = r#"
        function main() -> Int { std::math::add(1, 2 * 3) }
    "#;
    let prog = parse(src).expect("should parse namespaced call");
    assert_eq!(prog.funcs.len(), 1);
    assert_eq!(prog.funcs[0].name, "main");
}

#[test]
fn errors_on_bare_path_without_args() {
    // A path like `std::foo::bar` without `(...)` should not parse as an expression
    let src = r#"
        function main() -> Int { std::foo::bar }
    "#;
    let _ = parse(src).expect_err("bare namespaced path without args should fail");
}
