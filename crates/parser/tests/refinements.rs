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
fn parses_inline_refinements_on_params_and_returns() {
    let src = r#"
        function f(x: Int where x >= 0) -> Int where result >= 0 { x }
    "#;
    let program = parse(src).expect("parse ok");
    assert_eq!(program.funcs.len(), 1);
    assert_eq!(program.refined_aliases.len(), 2);
    let func = &program.funcs[0];
    assert!(matches!(
        &func.params[0].ty,
        Type::Named { name, .. } if name.contains("__clg$inline_ref$")
    ));
    assert!(matches!(
        &func.ret,
        Type::Named { name, .. } if name.contains("__clg$inline_ref$")
    ));
}

#[test]
fn parses_where_bounds_when_type_param_is_named_result() {
    let src = r#"
        function id<result: Eq>(x: result) -> result where result: Eq { x }
    "#;
    let program = parse(src).expect("parse ok");
    assert_eq!(program.funcs.len(), 1);
    assert_eq!(program.refined_aliases.len(), 0);
    let func = &program.funcs[0];
    assert_eq!(func.where_bounds.len(), 2);
    assert!(matches!(&func.ret, Type::Named { name, .. } if name == "result"));
}
