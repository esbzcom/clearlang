use clg_parser::parse;
use clg_typer::type_check_only;

#[test]
fn typecheck_block_body_function() {
    let src = r#"
function add_one(x: Int) -> Int {
    let y = x + 1;
    y
}
"#;

    let ast = parse(src).expect("parse succeeds");
    type_check_only(&ast).expect("type check succeeds");
}
