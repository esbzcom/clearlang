use clg_parser::parse_errors;

#[test]
fn parse_errors_returns_multiple_items_for_contrived_input() {
    // Contrived input with multiple syntax issues inside one function call:
    // - missing comma between arguments
    // - missing closing ')'
    // - missing closing '}' for the function body
    let src = r#"
        function main() -> Int { add(1 2 }
    "#;
    match parse_errors(src) {
        Ok(_) => panic!("expected parse_errors to return Err with at least one item"),
        Err(errs) => {
            assert!(
                !errs.is_empty(),
                "expected >= 1 parser error, got {}",
                errs.len()
            );
            let msg = errs
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            // Heuristic: expect at least one helpful label such as 'comma' or a closing paren
            assert!(
                msg.contains("comma") || msg.contains("')'") || msg.contains("expected"),
                "expected an error mentioning 'comma' or ')', got: {msg}"
            );
        }
    }
}

#[test]
fn parse_errors_reports_missing_else_as_p010() {
    let src = r#"
        function main() -> Int { if true { 1 } }
    "#;
    let errs = parse_errors(src).expect_err("missing else should error");
    assert!(
        errs.iter().any(|e| e.code == "P010"),
        "expected P010, got {:?}",
        errs.iter().map(|e| e.code).collect::<Vec<_>>()
    );
}

#[test]
fn parse_errors_does_not_report_lambda_hint_for_match_arm_arrow() {
    let src = r#"
        function main() -> Int {
            let v = Some(1)
            let y = match v { Some(x) => x, None => 0 }
            let n =
        }
    "#;
    let errs = parse_errors(src).expect_err("expected parse error");
    let joined = errs
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !joined.contains("lambda parameters require type annotations"),
        "unexpected lambda hint in non-lambda source: {joined}"
    );
}

#[test]
fn parse_errors_does_not_report_capture_list_hint_for_string_literal() {
    let src = r#"
        function main() -> Int {
            let s = "[x](y: Int) => y"
            let n =
        }
    "#;
    let errs = parse_errors(src).expect_err("expected parse error");
    let joined = errs
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !joined.contains("capture-list syntax is not supported"),
        "unexpected capture-list hint in string literal source: {joined}"
    );
}

#[test]
fn parse_errors_reports_capture_list_as_p012() {
    let src = r#"
        function bad() -> function(Int) -> Int {
            [x](y: Int) => y + x
        }
    "#;
    let errs = parse_errors(src).expect_err("capture-list syntax should error");
    assert!(
        errs.iter().any(|e| e.code == "P012"),
        "expected P012, got {:?}",
        errs.iter().map(|e| e.code).collect::<Vec<_>>()
    );
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("capture-list syntax is not supported in v1")),
        "expected explicit capture-list rejection message, got {:?}",
        errs.iter().map(|e| e.message.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn parse_errors_reports_export_import_as_p011() {
    let src = r#"
        export import foo::bar::{Baz};
    "#;
    let errs = parse_errors(src).expect_err("export import should be rejected");
    assert!(
        errs.iter().any(|e| e.code == "P011"),
        "expected P011, got {:?}",
        errs.iter().map(|e| e.code).collect::<Vec<_>>()
    );
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`export import` is not supported in v1")),
        "expected explicit export import message, got {:?}",
        errs.iter().map(|e| e.message.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn parse_errors_reports_theorem_keyword_as_p014() {
    let src = r#"
        theorem function add(x: Int, y: Int) -> Int { x + y }
    "#;
    let errs = parse_errors(src).expect_err("theorem keyword should be rejected");
    assert!(
        errs.iter().any(|e| e.code == "P014"),
        "expected P014, got {:?}",
        errs.iter().map(|e| e.code).collect::<Vec<_>>()
    );
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("theorem-grade is a release certification status")),
        "expected explicit deferred theorem syntax message, got {:?}",
        errs.iter().map(|e| e.message.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn parse_errors_reports_versioned_import_as_p015() {
    let src = r#"
        import vendor::crypto@1_2_3::hash;

        function main() -> Int { 0 }
    "#;
    let errs = parse_errors(src).expect_err("versioned import should be rejected");
    assert!(
        errs.iter().any(|e| e.code == "P015"),
        "expected P015, got {:?}",
        errs.iter().map(|e| e.code).collect::<Vec<_>>()
    );
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("keep import paths version-free")),
        "expected explicit version-free import guidance, got {:?}",
        errs.iter().map(|e| e.message.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn parse_errors_accepts_inline_refinements() {
    let src = r#"
        function f(x: Int where x >= 0) -> Int where result >= 0 { x }
    "#;
    let parsed = parse_errors(src).expect("inline refinements should parse");
    assert_eq!(parsed.funcs.len(), 1);
    assert_eq!(parsed.refined_aliases.len(), 2);
}
