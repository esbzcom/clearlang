use lumi_parser::parse;

#[test]
fn parses_string_literal_simple() {
    let src = r#"
        pure fn id(s: Str) -> Str { "hello\nworld" }
    "#;
    let _ast = parse(src).expect("parse ok");
}

#[test]
fn parses_multiline_string_literal() {
    let src = r#"
        fn main() -> Str { "line1
line2" }
    "#;
    let _ = parse(src).expect("parse ok");
}

#[test]
fn errors_on_unclosed_string() {
    let src = r#"
        fn main() -> Str { "unterminated }
    "#;
    let _ = parse(src).expect_err("should fail on unclosed string");
}
