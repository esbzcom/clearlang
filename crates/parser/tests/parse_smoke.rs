use lumi_parser::parse;

#[test]
fn parses_add2_and_main() {
    let src = r#"
        pure fn add2(a: Int, b: Int) -> Int { a + b }
        fn main() -> Int { add2(20, 22) }
    "#;
    let ast = parse(src).expect("parse ok");
    assert_eq!(ast.funcs.len(), 2);
    assert_eq!(ast.funcs[0].name, "add2");
    assert_eq!(ast.funcs[1].name, "main");
}

#[test]
fn parses_add() {
    let src = "pure fn add(x: Int, y: Int) -> Int { x + y }";
    let prog = parse(src).expect("should parse");
    assert_eq!(prog.funcs.len(), 1);
    assert_eq!(prog.funcs[0].name, "add");
}

#[test]
fn parses_call_expr() {
    let src = r#"
        fn main() -> Int { add(1, (2 + 3) * 4) }
        pure fn add(x: Int, y: Int) -> Int { x + y }
    "#;
    let prog = parse(src).expect("should parse");
    assert_eq!(prog.funcs.len(), 2);
}

#[test]
fn parses_nested_calls_and_precedence() {
    let src = r#"
        fn main() -> Int {
            add(1, add(2, 3) * 4 + (5 * add(6, 7)))
        }
        pure fn add(x: Int, y: Int) -> Int { x + y }
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
        fn main() -> Int {
            add(
                10,
                (2
                    + 3)
                * 4
            )
        }
        pure fn add(x: Int, y: Int) -> Int { x + y }
    "#;

    let prog = parse(src).expect("should parse");
    assert_eq!(prog.funcs.len(), 2);
}

#[test]
fn parses_call_chain_as_argument() {
    // If your language allows using call results inside other calls
    // e.g., f(g(h(1,2), 3), 4)
    let src = r#"
        fn main() -> Int { f(g(h(1, 2), 3), 4) }
        pure fn f(a: Int, b: Int) -> Int { a + b }
        pure fn g(a: Int, b: Int) -> Int { a * b }
        pure fn h(a: Int, b: Int) -> Int { a - b }
    "#;

    let prog = parse(src).expect("should parse");
    assert_eq!(prog.funcs.len(), 4);
}

#[test]
fn parses_multiple_args_and_parentheses() {
    // Stress commas/parentheses and precedence: add(1, (2 + 3) * (4 + 5))
    let src = r#"
        fn main() -> Int { add(1, (2 + 3) * (4 + 5)) }
        pure fn add(x: Int, y: Int) -> Int { x + y }
    "#;

    parse(src).expect("should parse");
}

/* ---------- Parser error tests (syntax) ---------- */

#[test]
fn errors_on_missing_closing_paren_in_call() {
    let src = r#"
        fn main() -> Int { add(1, 2 }
        pure fn add(x: Int, y: Int) -> Int { x + y }
    "#;

    let err = parse(src).expect_err("should fail on missing ')'");
    // Optionally assert error span/message if your parser exposes it.
    // assert!(format!("{err:?}").contains("')'"));
}

#[test]
fn errors_on_missing_comma_between_args() {
    let src = r#"
        fn main() -> Int { add(1 2) }
        pure fn add(x: Int, y: Int) -> Int { x + y }
    "#;

    parse(src).expect_err("should fail on missing comma");
    // assert!(format!("{err:?}").contains("comma"));
}

#[test]
fn errors_on_trailing_comma_in_call_if_disallowed() {
    // Enable this only if your grammar forbids trailing commas.
    let src = r#"
        fn main() -> Int { add(1, 2,) }
        pure fn add(x: Int, y: Int) -> Int { x + y }
    "#;

    let _ = parse(src).expect_err("should fail on trailing comma (if not supported)");
}

/* ---------- Typer-bound tests (keep ignored until Phase 3) ---------- */

#[test]
#[ignore] // enable after typer is in place
fn typer_errors_on_arity_mismatch() {
    // Parser should accept; typer should reject wrong arity.
    let src = r#"
        fn main() -> Int { add(1) }
        pure fn add(x: Int, y: Int) -> Int { x + y }
    "#;

    // let prog = parse(src).expect("parsed");
    // let err = type_check(&prog).expect_err("should fail arity check");
    // assert!(format!("{err:?}").contains("arity"));
}

#[test]
#[ignore] // enable after typer is in place
fn typer_errors_on_unknown_function() {
    let src = r#"
        fn main() -> Int { missing(1, 2) }
    "#;

    // let prog = parse(src).expect("parsed");
    // let err = type_check(&prog).expect_err("unknown function");
    // assert!(format!("{err:?}").contains("unknown function"));
}
