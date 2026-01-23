use clg_ast::{BinOp, Effect, Expr, UnaryOp};
use clg_parser::parse;

#[test]
fn parses_contract_clauses() {
    let src = r#"
        pure function inc(x: Int) -> Int
            require { x < 10 && !(x == 0) }
            require { 0 <= x }
            ensure { result >= x && result != x }
        { x + 1 }
    "#;
    let ast = parse(src).expect("parse ok");
    assert_eq!(ast.funcs.len(), 1);
    let func = &ast.funcs[0];
    assert_eq!(func.name, "inc");
    assert_eq!(func.effect, Effect::Pure);
    assert!(func.effect_span.is_some());
    assert_eq!(func.requires.len(), 2);
    assert_eq!(func.ensures.len(), 1);
}

#[test]
fn parses_contract_logic_precedence() {
    let src = r#"
        pure function guard(x: Int, y: Int, z: Int) -> Int
            require { x < 5 || y > 0 && !(z == 3) }
        { x }
    "#;
    let ast = parse(src).expect("parse ok");
    let expr = &ast.funcs[0].requires.first().expect("require present").expr;
    match expr {
        Expr::Bin {
            op: BinOp::Or,
            lhs,
            rhs,
            ..
        } => {
            assert!(
                matches!(**lhs, Expr::Bin { op: BinOp::Lt, .. }),
                "left operand should be < comparison"
            );
            match &**rhs {
                Expr::Bin {
                    op: BinOp::And,
                    lhs: and_lhs,
                    rhs: and_rhs,
                    ..
                } => {
                    assert!(
                        matches!(**and_lhs, Expr::Bin { op: BinOp::Gt, .. }),
                        "AND lhs should be > comparison"
                    );
                    match &**and_rhs {
                        Expr::Unary {
                            op: UnaryOp::Not,
                            expr: inner,
                            ..
                        } => {
                            assert!(
                                matches!(**inner, Expr::Bin { op: BinOp::Eq, .. }),
                                "NOT should wrap equality"
                            );
                        }
                        other => panic!("expected unary NOT, found {other:?}"),
                    }
                }
                other => panic!("expected logical AND on right, found {other:?}"),
            }
        }
        other => panic!("expected logical OR at top level, found {other:?}"),
    }
}

#[test]
fn parses_bitwise_and_shift_precedence() {
    let src = r#"
        function main() -> Int { 1 + 2 << 3 & 4 }
    "#;
    let ast = parse(src).expect("parse ok");
    let expr = match &ast.funcs[0].body {
        Expr::Block { block } => block.tail.as_deref().expect("block tail"),
        other => other,
    };
    match expr {
        Expr::Bin {
            op: BinOp::BitAnd,
            lhs,
            rhs,
            ..
        } => {
            assert!(matches!(**rhs, Expr::Int(4, _)));
            match &**lhs {
                Expr::Bin {
                    op: BinOp::Shl,
                    lhs: shl_lhs,
                    rhs: shl_rhs,
                    ..
                } => {
                    assert!(matches!(**shl_rhs, Expr::Int(3, _)));
                    assert!(
                        matches!(**shl_lhs, Expr::Bin { op: BinOp::Add, .. }),
                        "shift lhs should be additive"
                    );
                }
                other => panic!("expected shift on left, found {other:?}"),
            }
        }
        other => panic!("expected bitwise AND at top, found {other:?}"),
    }
}

#[test]
fn parses_comparison_after_bitwise() {
    let src = r#"
        function main() -> Bool { 1 & 2 == 0 }
    "#;
    let ast = parse(src).expect("parse ok");
    let expr = match &ast.funcs[0].body {
        Expr::Block { block } => block.tail.as_deref().expect("block tail"),
        other => other,
    };
    match expr {
        Expr::Bin {
            op: BinOp::Eq,
            lhs,
            rhs,
            ..
        } => {
            assert!(matches!(**rhs, Expr::Int(0, _)));
            assert!(
                matches!(**lhs, Expr::Bin { op: BinOp::BitAnd, .. }),
                "lhs should be bitwise AND"
            );
        }
        other => panic!("expected equality at top, found {other:?}"),
    }
}

#[test]
fn parses_ensure_only_contract() {
    let src = r#"
        function health() -> Int
            ensure { true }
            ensure { true }
        { 42 }
    "#;
    let ast = parse(src).expect("parse ok");
    assert_eq!(ast.funcs.len(), 1);
    let func = &ast.funcs[0];
    assert_eq!(func.effect, Effect::None);
    assert!(func.effect_span.is_none());
    assert_eq!(func.requires.len(), 0);
    assert_eq!(func.ensures.len(), 2);
}

#[test]
fn parse_errors_on_missing_contract_braces() {
    let src = r#"
        pure function bad(x: Int) -> Int
            require x > 0
        { x }
    "#;
    let err = parse(src).expect_err("missing braces around contract");
    assert!(
        err.contains("keyword `require` must be followed by `{ ... }`") || err.contains("'{"),
        "error should hint at missing braces, got: {err}"
    );
}

#[test]
fn parses_add2_and_main() {
    let src = r#"
        pure function add2(a: Int, b: Int) -> Int { a + b }
        function main() -> Int { add2(20, 22) }
    "#;
    let ast = parse(src).expect("parse ok");
    assert_eq!(ast.funcs.len(), 2);
    assert_eq!(ast.funcs[0].name, "add2");
    assert_eq!(ast.funcs[1].name, "main");
}

#[test]
fn parses_add() {
    let src = "pure function add(x: Int, y: Int) -> Int { x + y }";
    let prog = parse(src).expect("should parse");
    assert_eq!(prog.funcs.len(), 1);
    assert_eq!(prog.funcs[0].name, "add");
}

#[test]
fn parses_call_expr() {
    let src = r#"
        function main() -> Int { add(1, (2 + 3) * 4) }
        pure function add(x: Int, y: Int) -> Int { x + y }
    "#;
    let prog = parse(src).expect("should parse");
    assert_eq!(prog.funcs.len(), 2);
}

#[test]
fn parses_nested_calls_and_precedence() {
    let src = r#"
        function main() -> Int {
            add(1, add(2, 3) * 4 + (5 * add(6, 7)))
        }
        pure function add(x: Int, y: Int) -> Int { x + y }
    "#;

    let prog = parse(src).expect("should parse");
    assert_eq!(prog.funcs.len(), 2, "expect two functions (main, add)");

    // Optional: assert the AST shape if your types expose it.
    // Example (adapt names to your AST):
    // let main = &prog.funcs[0];
    // match &main.body {
    //     Expr::Call { name, args } => {
    //         assert_eq!(name.as_str(), "add");
    //         assert_eq!(args.len(), 2);
    //         // right arg should be a BinOp with correct precedence
    //     }
    //     _ => panic!("main body should be a call expression"),
    // }
}

#[test]
fn parses_multi_line_and_spaces_in_calls() {
    let src = r#"
        function main() -> Int {
            add(
                10,
                (2
                    + 3)
                * 4
            )
        }
        pure function add(x: Int, y: Int) -> Int { x + y }
    "#;

    let prog = parse(src).expect("should parse");
    assert_eq!(prog.funcs.len(), 2);
}

#[test]
fn parses_call_chain_as_argument() {
    // If your language allows using call results inside other calls
    // e.g., f(g(h(1,2), 3), 4)
    let src = r#"
        function main() -> Int { f(g(h(1, 2), 3), 4) }
        pure function f(a: Int, b: Int) -> Int { a + b }
        pure function g(a: Int, b: Int) -> Int { a * b }
        pure function h(a: Int, b: Int) -> Int { a - b }
    "#;

    let prog = parse(src).expect("should parse");
    assert_eq!(prog.funcs.len(), 4);
}

#[test]
fn parses_multiple_args_and_parentheses() {
    // Stress commas/parentheses and precedence: add(1, (2 + 3) * (4 + 5))
    let src = r#"
        function main() -> Int { add(1, (2 + 3) * (4 + 5)) }
        pure function add(x: Int, y: Int) -> Int { x + y }
    "#;

    parse(src).expect("should parse");
}

/* ---------- Parser error tests (syntax) ---------- */

#[test]
fn errors_on_missing_closing_paren_in_call() {
    let src = r#"
        function main() -> Int { add(1, 2 }
        pure function add(x: Int, y: Int) -> Int { x + y }
    "#;

    let err = parse(src).expect_err("should fail on missing ')'");
    // Optionally assert error span/message if your parser exposes it.
    assert!(format!("{err:?}").contains("')'"));
}

#[test]
fn errors_on_missing_comma_between_args() {
    let src = r#"
        function main() -> Int { add(1 2) }
        pure function add(x: Int, y: Int) -> Int { x + y }
    "#;

    parse(src).expect_err("should fail on missing comma");
    // assert!(format!("{err:?}").contains("comma"));
}

#[test]
fn errors_on_trailing_comma_in_call_if_disallowed() {
    // Enable this only if your grammar forbids trailing commas.
    let src = r#"
        function main() -> Int { add(1, 2,) }
        pure function add(x: Int, y: Int) -> Int { x + y }
    "#;

    let _ = parse(src).expect_err("should fail on trailing comma (if not supported)");
}

/* ---------- Typer-bound tests (keep ignored until Phase 3) ---------- */

#[test]
#[ignore] // enable after typer is in place
fn typer_errors_on_arity_mismatch() {
    // Parser should accept; typer should reject wrong arity.
    let _src = r#"
        function main() -> Int { add(1) }
        pure function add(x: Int, y: Int) -> Int { x + y }
    "#;

    // let prog = parse(src).expect("parsed");
    // let err = type_check(&prog).expect_err("should fail arity check");
    // assert!(format!("{err:?}").contains("arity"));
}

#[test]
#[ignore] // enable after typer is in place
fn typer_errors_on_unknown_function() {
    let _src = r#"
        function main() -> Int { missing(1, 2) }
    "#;

    // let prog = parse(src).expect("parsed");
    // let err = type_check(&prog).expect_err("unknown function");
    // assert!(format!("{err:?}").contains("unknown function"));
}
