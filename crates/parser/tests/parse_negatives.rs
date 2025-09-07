use lumi_parser::parse;

#[test]
fn errors_on_reserved_keyword_as_func_name() {
    // Using a reserved keyword `function` as an identifier should fail to parse.
    let src = r#"
        pure function function(a: Int) -> Int { a }
    "#;
    parse(src).expect_err("should reject reserved keyword as function name");
}

#[test]
fn errors_on_reserved_keyword_as_param_name() {
    // Using a reserved keyword `true` as a parameter name should fail to parse.
    let src = r#"
        pure function id(true: Int) -> Int { true }
    "#;
    parse(src).expect_err("should reject reserved keyword as parameter name");
}

#[test]
fn errors_on_unknown_type_name() {
    // Unknown type `Foo` should fail to parse in a parameter or return type.
    let src = r#"
        pure function bad(a: Foo) -> Int { 0 }
    "#;
    parse(src).expect_err("should reject unknown type names");
}

#[test]
fn missing_function_name_shows_identifier_label() {
    let src = r#"
        pure function (a: Int) -> Int { a }
    "#;
    let err = parse(src).expect_err("should fail without function name");
    assert!(
        err.contains("identifier") || err.contains("function name"),
        "error should mention identifier/function name label, got: {err}"
    );
}

#[test]
fn missing_type_in_param_shows_type_label() {
    let src = r#"
        pure function f(a: ) -> Int { 0 }
    "#;
    let err = parse(src).expect_err("should fail without a type in parameter");
    assert!(err.contains("type"), "error should mention type label, got: {err}");
}

#[test]
fn missing_comma_between_args_mentions_comma() {
    let src = r#"
        function main() -> Int { add(1 2) }
        pure function add(x: Int, y: Int) -> Int { x + y }
    "#;
    let err = parse(src).expect_err("should fail on missing comma");
    assert!(err.contains("comma"), "error should mention comma label, got: {err}");
}

#[test]
fn missing_closing_paren_mentions_paren() {
    let src = r#"
        function main() -> Int { add(1, 2 }
        pure function add(x: Int, y: Int) -> Int { x + y }
    "#;
    let err = parse(src).expect_err("should fail on missing ')'");
    assert!(err.contains("')'"), "error should mention closing paren label, got: {err}");
}

#[test]
fn function_missing_param_parens_mentions_open_paren() {
    let src = r#"
        pure function f -> Int { 0 }
    "#;
    let err = parse(src).expect_err("should fail without parameter parentheses");
    assert!(err.contains("'('"), "error should mention opening paren label, got: {err}");
}
