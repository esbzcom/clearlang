use clg_ast::{Expr, Stmt};
use clg_parser::parse;

#[test]
fn parse_resource_with_drop_body() {
    let src = r#"
resource FileHandle {
    path: String;
    drop {
        std::fs::close(path);
    }
}

function main() -> Int { 42 }
"#;

    let ast = parse(src).expect("parse succeeds");
    assert_eq!(ast.resources.len(), 1);
    let resource = &ast.resources[0];
    assert_eq!(resource.name, "FileHandle");
    assert_eq!(resource.fields.len(), 1);
    assert_eq!(resource.fields[0].name, "path");
    assert!(matches!(resource.fields[0].ty, clg_ast::Type::String));

    assert_eq!(resource.drop_block.statements.len(), 1);
    match &resource.drop_block.statements[0] {
        Stmt::Expr { expr, .. } => match expr {
            Expr::Call { callee, args, .. } => {
                assert_eq!(callee, "std::fs::close");
                assert_eq!(args.len(), 1);
            }
            other => panic!("unexpected expr: {other:?}"),
        },
        other => panic!("unexpected stmt: {other:?}"),
    }
    assert!(resource.drop_block.tail.is_none());
}

#[test]
fn parse_resource_with_empty_drop() {
    let src = r#"
resource Token {
    drop {}
}

function main() -> Int { 0 }
"#;

    let ast = parse(src).expect("parse succeeds");
    assert_eq!(ast.resources.len(), 1);
    let resource = &ast.resources[0];
    assert_eq!(resource.name, "Token");
    assert!(resource.fields.is_empty());
    assert!(resource.drop_block.statements.is_empty());
    assert!(resource.drop_block.tail.is_none());
}
