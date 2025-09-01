use lumi_parser::parse;

#[test]
fn errors_on_reserved_keyword_as_func_name() {
    // Using a reserved keyword `fn` as an identifier should fail to parse.
    let src = r#"
        pure fn fn(a: Int) -> Int { a }
    "#;
    parse(src).expect_err("should reject reserved keyword as function name");
}

#[test]
fn errors_on_reserved_keyword_as_param_name() {
    // Using a reserved keyword `true` as a parameter name should fail to parse.
    let src = r#"
        pure fn id(true: Int) -> Int { true }
    "#;
    parse(src).expect_err("should reject reserved keyword as parameter name");
}

#[test]
fn errors_on_unknown_type_name() {
    // Unknown type `Foo` should fail to parse in a parameter or return type.
    let src = r#"
        pure fn bad(a: Foo) -> Int { 0 }
    "#;
    parse(src).expect_err("should reject unknown type names");
}

