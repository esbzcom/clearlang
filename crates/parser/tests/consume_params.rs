use clg_ast::ParamKind;
use clg_parser::parse;

#[test]
fn parse_consume_param() {
    let src = r#"
function main(consume handle: Int, value: Int) -> Int { value }
"#;

    let ast = parse(src).expect("parse succeeds");
    let func = &ast.funcs[0];
    assert_eq!(func.params.len(), 2);
    assert_eq!(func.params[0].name, "handle");
    assert_eq!(func.params[0].kind, ParamKind::Consume);
    assert_eq!(func.params[1].name, "value");
    assert_eq!(func.params[1].kind, ParamKind::Borrow);
}
