use lumi_parser::parse;

#[test]
fn parses_match_on_option() {
    let src = r#"
        function main() -> Int {
            match maybe {
                Some(x) => 1,
                None => 0
            }
        }
    "#;
    let _ = parse(src).expect("parse ok");
}

#[test]
fn parses_match_on_result() {
    let src = r#"
        function main() -> Int {
            match r {
                Ok(v) => v,
                Err(e) => 0
            }
        }
    "#;
    let _ = parse(src).expect("parse ok");
}

