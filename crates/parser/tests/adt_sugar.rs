use clg_parser::parse;

#[test]
fn parses_if_let_option() {
    let src = r#"
        pure function unwrap(opt: Option<Int>) -> Int {
            if let Some(value) = opt { value } else { 0 }
        }
    "#;
    parse(src).expect("parse if let option");
}

#[test]
fn if_let_requires_else() {
    let src = r#"
        pure function bad(opt: Option<Int>) -> Int {
            if let Some(v) = opt { v }
        }
    "#;
    parse(src).expect_err("if let without else should fail");
}

#[test]
fn parses_option_coalesce() {
    let src = r#"
        pure function default_age(age: Option<Int>) -> Int { age ?? 18 }
    "#;
    parse(src).expect("parse coalesce");
}

#[test]
fn parses_try_operator() {
    let src = r#"
        pure function inc(opt: Option<Int>) -> Option<Int> { Some(opt? + 1) }
    "#;
    parse(src).expect("parse try operator");
}
#[test]
fn parses_if_let_result_ok() {
    let src = r#"
        pure function unwrap(res: Result<Int, String>) -> Int {
            if let Ok(v) = res { v } else { 0 }
        }
    "#;
    parse(src).expect("parse if let result ok");
}

#[test]
fn parses_if_let_result_err() {
    let src = r#"
        pure function handle(res: Result<Int, String>) -> Result<Int, String> {
            if let Err(e) = res { Err(e) } else { res }
        }
    "#;
    parse(src).expect("parse if let result err");
}

#[test]
fn parses_result_try_operator() {
    let src = r#"
        pure function inc(res: Result<Int, String>) -> Result<Int, String> {
            Ok(res? + 1)
        }
    "#;
    parse(src).expect("parse result try operator");
}
