use lumi_parser::parse_errors;

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
        Ok(_) => panic!("expected parse_errors to return Err with multiple items"),
        Err(errs) => {
            assert!(errs.len() >= 2, "expected >= 2 parser errors, got {}", errs.len());
            let msg = errs.iter().map(|e| e.message.as_str()).collect::<Vec<_>>().join("\n");
            // Heuristic checks that typical labels appear in at least some messages
            assert!(msg.contains("comma"), "expected an error mentioning 'comma' in messages: {msg}");
            assert!(msg.contains("')'"), "expected an error mentioning ')': {msg}");
        }
    }
}

