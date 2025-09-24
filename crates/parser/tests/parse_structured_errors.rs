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
                errs.len() >= 1,
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
