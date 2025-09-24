use clg_parser::parse;
use clg_typer::{check, TyperError};

#[test]
fn collections_calls_produce_collection_kind_error() {
    let src = r#"
        pure function size() -> Int { std::list::len(0) }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = check(&ast).expect_err("should fail with wrong collection kind");
    if let Some(te) = err.downcast_ref::<TyperError>() {
        assert_eq!(te.code, "T207");
        assert!(te.message.contains("expected List"));
    } else {
        panic!("expected TyperError, got: {err:#}");
    }
}
