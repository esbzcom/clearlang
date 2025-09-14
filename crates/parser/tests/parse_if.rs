use lumi_parser::parse;

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
            if x { 1 } elif x { 2 } else { 3 }
        }
    "#;
    // Note: typer will enforce Bool conds; parser accepts the shape.
    let _ = parse(src).expect("parse else-if ok");
}

#[test]
fn errors_without_else_in_expression_form() {
    let src = r#"
        function main() -> Int { if true { 1 } }
    "#;
    let _ = parse(src).expect_err("missing else should be a parse error");
}

