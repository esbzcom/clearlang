use clg_parser::parse;
use clg_typer::type_check_only;

#[test]
fn pure_recursion_without_measure_is_rejected() {
    let src = r#"
        pure function fact(n: Int) -> Int {
            if n == 0 { 1 } else { fact(n - 1) }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = type_check_only(&ast).expect_err("totality should reject pure recursion");
    let msg = err.to_string();
    assert!(
        msg.contains("T902") && msg.contains("recursive call"),
        "unexpected error: {msg}"
    );
}
