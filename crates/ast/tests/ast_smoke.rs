use clg_ast::{Expr, MatchArm, MatchPat, Span, Type};

#[test]
fn type_equality_handles_nested_variants() {
    let t1 = Type::Map(
        Box::new(Type::String),
        Box::new(Type::Option(Box::new(Type::Int))),
    );
    let t2 = Type::Map(
        Box::new(Type::String),
        Box::new(Type::Option(Box::new(Type::Int))),
    );
    assert_eq!(t1, t2);
}

#[test]
fn expr_construction_preserves_span() {
    let span = Span { start: 4, end: 9 };
    if let Expr::Int(_, stored) = Expr::Int(42, span) {
        assert_eq!(stored, span);
    } else {
        panic!("expected int expression");
    }
}

#[test]
fn match_arm_clone_retains_pattern_and_expr() {
    let arm = MatchArm {
        pat: MatchPat::Some("value".into()),
        expr: Expr::Var("value".into(), Span { start: 10, end: 15 }),
    };
    let cloned = arm.clone();
    match cloned.pat {
        MatchPat::Some(ref name) => assert_eq!(name, "value"),
        _ => panic!("expected Some binder"),
    }
    if let Expr::Var(_, span) = cloned.expr {
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 15);
    } else {
        panic!("expected Var expression");
    }
}
