use lumi_parser::parse;
use lumi_typer::{check, TyperError};

#[test]
fn collections_calls_produce_t101_error() {
    let src = r#"
        pure function size() -> Int { std::list::len(0) }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = check(&ast).expect_err("should fail with collections unavailable");
    if let Some(te) = err.downcast_ref::<TyperError>() {
        assert_eq!(te.code, "T101");
        assert!(te.message.contains("collections"));
        assert!(te.message.contains("std::list::len"));
    } else {
        panic!("expected TyperError, got: {err:#}");
    }
}

