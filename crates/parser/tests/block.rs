use clg_ast::{Expr, Stmt};
use clg_parser::parse;

#[test]
fn parse_block_in_function_body() {
    let src = r#"
function add_one(x: Int) -> Int {
    let y = x + 1;
    y
}
"#;

    let ast = parse(src).expect("parse succeeds");
    let func = &ast.funcs[0];
    match &func.body {
        Expr::Block { block } => {
            assert_eq!(block.statements.len(), 1);
            match &block.statements[0] {
                Stmt::Let { name, .. } => assert_eq!(name, "y"),
                other => panic!("expected let stmt, found {other:?}"),
            }
            let tail = block.tail.as_ref().expect("tail expression");
            match tail.as_ref() {
                Expr::Var(name, _) => assert_eq!(name, "y"),
                other => panic!("unexpected tail expr: {other:?}"),
            }
        }
        other => panic!("expected block expression, found {other:?}"),
    }
}
