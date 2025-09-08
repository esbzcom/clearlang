use lumi_parser::parse;

#[test]
fn parses_option_and_result_types_in_signatures() {
    let src = r#"
        pure function f(x: Option<Int>) -> Result<Int, String> { x }
    "#;
    let _ = parse(src).expect("parse ok");
}

#[test]
fn parses_nested_parametric_types() {
    let src = r#"
        pure function g(x: Option<Result<Int, String>>) -> Option<Int> { 0 }
    "#;
    let _ = parse(src).expect("parse ok");
}

