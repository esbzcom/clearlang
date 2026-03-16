use clg_parser::{parse, parse_errors};

#[test]
fn parses_line_and_block_comments() {
    let src = r#"
        // top-level comment
        function main() -> Int {
            /* block comment before expression */
            1_000 // trailing comment
        }
    "#;
    let _ = parse(src).expect("comments should be accepted");
}

#[test]
fn parses_nested_block_comments() {
    let src = r#"
        function main() -> Int {
            /* outer comment
                /* nested comment */
               back to outer */
            42
        }
    "#;
    let _ = parse(src).expect("nested block comments should be accepted");
}

#[test]
fn comments_inside_strings_are_not_stripped() {
    let src = r#"
        function main() -> String { "literal // not comment /* still string */" }
    "#;
    let _ = parse(src).expect("comment markers inside strings should parse as string content");
}

#[test]
fn parses_integer_literals_with_underscore_separators() {
    let src = r#"
        function main() -> Int { 1_024 + 32 }
    "#;
    let _ = parse(src).expect("underscore separators should parse");
}

#[test]
fn rejects_misplaced_underscore_separator() {
    let src = r#"
        function main() -> Int { 1__024 }
    "#;
    let _ = parse(src).expect_err("misplaced separators should fail");
}

#[test]
fn parse_errors_suggest_underscore_for_comma_grouping() {
    let src = r#"
        function main() -> Int { 1,000 }
    "#;
    let errs = parse_errors(src).expect_err("comma grouped number should fail");
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("comma separators are not allowed in numeric literals")),
        "expected comma-grouping diagnostic, got {:?}",
        errs.iter().map(|e| e.message.as_str()).collect::<Vec<_>>()
    );
    assert!(
        errs.iter().any(|e| e.message.contains("use `_`")),
        "expected underscore suggestion, got {:?}",
        errs.iter().map(|e| e.message.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn parse_error_code_stays_stable_with_comments_present() {
    let base = r#"
        function main() -> Int { add(1 2) }
    "#;
    let with_comments = r#"
        function main() -> Int { add(1 /* between args */ 2) }
    "#;
    let base_errs = parse_errors(base).expect_err("base source should fail");
    let comment_errs = parse_errors(with_comments).expect_err("comment source should fail");
    assert_eq!(base_errs[0].code, "P001");
    assert_eq!(comment_errs[0].code, "P001");
}
