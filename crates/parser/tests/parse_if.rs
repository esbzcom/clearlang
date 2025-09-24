use clg_parser::parse;

#[test]
fn parses_simple_if_else() {
    let src = r#"
        function main() -> Int {
            if true { 1 } else { 0 }
        }
    "#;
    parse(src).expect("parse if/else ok");
}

#[test]
fn parses_else_if_chain() {
    let src = r#"
        function f(x: Int) -> Int {
            if x { 1 } else if x { 2 } else { 3 }
        }
    "#;
    // Note: typer will enforce Bool conds; parser accepts the shape.
    let _ = parse(src).expect("parse else-if ok");
}

#[test]
fn parses_else_if_alias_chain() {
    let src = r#"
        function f(b: Bool) -> Int {
            if b { 1 } else if b { 2 } else { 3 }
        }
    "#;
    parse(src).expect("parse else if alias ok");
}

#[test]
fn errors_without_else_in_expression_form() {
    let src = r#"
        function main() -> Int { if true { 1 } }
    "#;
    let _ = parse(src).expect_err("missing else should be a parse error");
}

#[test]
fn errors_without_else_with_else_if_chain() {
    let src = r#"
        function main() -> Int { if true { 1 } else if false { 2 } }
    "#;
    let _ = parse(src).expect_err("missing final else with else-if chain should error");
}

#[test]
fn parses_multiline_blocks() {
    let src = r#"
        pure function g(b: Bool) -> Int {
            if b {
                1
            } else {
                2
            }
        }
    "#;
    parse(src).expect("parse multiline if/else ok");
}
