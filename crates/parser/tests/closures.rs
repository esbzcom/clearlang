use clg_ast::{BinOp, Expr, Type};
use clg_parser::parse;

#[test]
fn parses_function_type_and_typed_lambda() {
    let src = r#"
        function make_adder(base: Int) -> function(Int) -> Int { (x: Int) => x + base }
    "#;

    let program = parse(src).expect("parse ok");
    let func = &program.funcs[0];
    match &func.ret {
        Type::Fn { params, ret } => {
            assert_eq!(params.len(), 1);
            assert!(matches!(params[0], Type::Int));
            assert!(matches!(**ret, Type::Int));
        }
        other => panic!("expected function type return, found {other:?}"),
    }

    let body_expr = match &func.body {
        Expr::Block { block } => block.tail.as_deref().expect("block tail"),
        other => other,
    };
    match body_expr {
        Expr::Lambda { params, body, .. } => {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");
            assert!(matches!(params[0].ty, Type::Int));
            assert!(
                matches!(**body, Expr::Bin { op: BinOp::Add, .. }),
                "lambda body should parse with expression precedence"
            );
        }
        other => panic!("expected lambda expression, found {other:?}"),
    }
}

#[test]
fn parses_zero_arg_lambda_and_function_type() {
    let src = r#"
        function forty_two() -> function() -> Int { () => 42 }
    "#;

    let program = parse(src).expect("parse ok");
    let func = &program.funcs[0];
    match &func.ret {
        Type::Fn { params, ret } => {
            assert!(params.is_empty());
            assert!(matches!(**ret, Type::Int));
        }
        other => panic!("expected zero-arg function type return, found {other:?}"),
    }
    let body_expr = match &func.body {
        Expr::Block { block } => block.tail.as_deref().expect("block tail"),
        other => other,
    };
    match body_expr {
        Expr::Lambda { params, body, .. } => {
            assert!(params.is_empty());
            assert!(matches!(**body, Expr::Int(42, _)));
        }
        other => panic!("expected zero-arg lambda expression, found {other:?}"),
    }
}

#[test]
fn rejects_untyped_lambda_params() {
    let src = r#"
        function bad(base: Int) -> function(Int) -> Int { (x) => x + base }
    "#;
    let err = parse(src).expect_err("missing lambda param type should fail");
    assert!(
        err.contains("lambda parameters require type annotations"),
        "expected typed-lambda diagnostic, got: {err}"
    );
}

#[test]
fn rejects_capture_list_lambda_syntax() {
    let src = r#"
        function bad(base: Int) -> function(Int) -> Int { [base](x: Int) => x + base }
    "#;
    let err = parse(src).expect_err("capture-list syntax should fail");
    assert!(
        err.contains("capture-list syntax is not supported"),
        "expected capture-list rejection diagnostic, got: {err}"
    );
}

#[test]
fn parses_function_keyword_in_decl_and_type_without_ambiguity() {
    let src = r#"
        function apply_twice(f: function(Int) -> Int, x: Int) -> Int { f(f(x)) }
    "#;

    let program = parse(src).expect("parse ok");
    let func = &program.funcs[0];
    match &func.params[0].ty {
        Type::Fn { params, ret } => {
            assert_eq!(params.len(), 1);
            assert!(matches!(params[0], Type::Int));
            assert!(matches!(**ret, Type::Int));
        }
        other => panic!("expected function-typed parameter, found {other:?}"),
    }
}
