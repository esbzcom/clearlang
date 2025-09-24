use clg_parser::parse;

#[test]
fn match_some_without_parens_fails() {
    let src = r#"
        function main() -> Int {
            match m { Some => 1, None => 0 }
        }
    "#;
    parse(src).expect_err("pattern Some requires a binder in parentheses");
}

#[test]
fn match_none_with_parens_fails() {
    let src = r#"
        function main() -> Int {
            match m { None(x) => 0 }
        }
    "#;
    parse(src).expect_err("None pattern must not have parentheses");
}

#[test]
fn match_unknown_pattern_identifier_fails() {
    let src = r#"
        function main() -> Int {
            match m { Somee(x) => 1 }
        }
    "#;
    parse(src).expect_err("unknown match arm pattern should fail");
}

#[test]
fn match_ok_without_parens_fails() {
    let src = r#"
        function main() -> Int {
            match r { Ok => 1, Err(e) => 0 }
        }
    "#;
    parse(src).expect_err("Ok pattern requires a binder in parentheses");
}

#[test]
fn match_missing_scrutinee_fails() {
    let src = r#"
        function main() -> Int { match { None => 0 } }
    "#;
    parse(src).expect_err("match requires a scrutinee expression");
}

#[test]
fn match_missing_arrow_fails() {
    let src = r#"
        function main() -> Int {
            match m { Some(x) 1, None => 0 }
        }
    "#;
    parse(src).expect_err("missing '=>' in match arm should fail");
}

