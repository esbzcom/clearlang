use lumi_parser::parse;

#[test]
fn parses_option_constructors_in_exprs() {
    // Some with argument
    let src1 = r#"
        function main() -> Int { Some(1) }
    "#;
    let _ = parse(src1).expect("parse Some(1)");

    // None without parentheses
    let src2 = r#"
        function main() -> Int { None }
    "#;
    let _ = parse(src2).expect("parse None");

    // None with parentheses (tolerated by parser)
    let src3 = r#"
        function main() -> Int { None() }
    "#;
    let _ = parse(src3).expect("parse None()");
}

#[test]
fn parses_result_constructors_in_exprs() {
    let src = r#"
        function main() -> Int { Ok(1) }
        pure function f() -> Int { 0 }
        pure function g() -> Int { 1 }
        function other() -> Int { Err(2) }
    "#;
    let _ = parse(src).expect("parse Ok/Err constructors");
}

