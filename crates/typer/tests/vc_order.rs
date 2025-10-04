use clg_ast::{BinOp, Contract, Effect, Expr, Func, Param, ParamKind, Program, Span, Type};
use clg_typer::generate_vcs;

fn span() -> Span {
    Span { start: 0, end: 0 }
}

#[test]
fn vcs_are_sorted_by_function_and_id() {
    let ensure_alpha = Contract {
        span: span(),
        expr: Expr::Bin {
            op: BinOp::Eq,
            lhs: Box::new(Expr::Var("result".into(), span())),
            rhs: Box::new(Expr::Int(0, span())),
            span: span(),
        },
    };
    let alpha = Func {
        effect: Effect::Pure,
        effect_span: None,
        name: "alpha".into(),
        params: vec![],
        ret: Type::Int,
        requires: vec![],
        ensures: vec![ensure_alpha],
        body: Expr::Int(0, span()),
    };

    let mut_guard = Contract {
        span: span(),
        expr: Expr::Call {
            callee: "std::list::can_mut".into(),
            args: vec![Expr::Var("l".into(), span())],
            span: span(),
        },
    };
    let beta = Func {
        effect: Effect::Mut,
        effect_span: None,
        name: "beta".into(),
        params: vec![Param {
            kind: ParamKind::Borrow,
            name: "l".into(),
            ty: Type::List(Box::new(Type::Int)),
        }],
        ret: Type::List(Box::new(Type::Int)),
        requires: vec![mut_guard],
        ensures: vec![],
        body: Expr::Call {
            callee: "std::list::push_mut".into(),
            args: vec![Expr::Var("l".into(), span()), Expr::Int(1, span())],
            span: span(),
        },
    };

    let program = Program {
        resources: vec![],
        funcs: vec![beta, alpha],
    };
    let vcs = generate_vcs(&program);
    let observed: Vec<String> = vcs
        .iter()
        .map(|vc| format!("{}:{}", vc.function, vc.vc_id))
        .collect();
    let mut expected = observed.clone();
    expected.sort();
    assert_eq!(
        observed, expected,
        "VCs should be sorted by function and id"
    );
}
