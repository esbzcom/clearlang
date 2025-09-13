use lumi_parser::parse;

#[test]
fn parses_return_in_function_body() {
    let src = r#"
        function main() -> Int { return 42 }
    "#;
    let _ = parse(src).expect("parse return body");
}

