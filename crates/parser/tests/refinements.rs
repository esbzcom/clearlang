use clg_ast::{BinOp, Expr, Type};
use clg_parser::parse;

#[test]
fn parses_refined_alias_and_binder() {
    let src = r#"
        type Nat = Int where n >= 0;
        function id(n: Nat) -> Nat { n }
    "#;
    let program = parse(src).expect("parse ok");
    assert_eq!(program.refined_aliases.len(), 1);
    let alias = &program.refined_aliases[0];
    assert_eq!(alias.name, "Nat");
    assert_eq!(alias.base, Type::Int);
    assert_eq!(alias.binder.as_deref(), Some("n"));
    assert!(
        matches!(alias.predicate, Expr::Bin { op: BinOp::Ge, .. }),
        "expected >= predicate, found {:?}",
        alias.predicate
    );
}

#[test]
fn parses_refined_alias_with_type_params() {
    let src = r#"
        type NonEmpty<T> = List<T> where xs > 0;
        pure function len(xs: NonEmpty) -> Int { 1 }
    "#;
    let program = parse(src).expect("parse ok");
    assert_eq!(program.refined_aliases.len(), 1);
    let alias = &program.refined_aliases[0];
    assert_eq!(alias.name, "NonEmpty");
    assert_eq!(alias.type_params, vec!["T".to_string()]);
    assert_eq!(alias.binder.as_deref(), Some("xs"));
    assert_eq!(program.funcs.len(), 1, "functions should still parse");
}

#[test]
fn rejects_inline_refinements_on_params() {
    let src = r#"
        function f(x: Int where x >= 0) -> Int { x }
    "#;
    assert!(parse(src).is_err(), "inline refinements should be rejected");
}
