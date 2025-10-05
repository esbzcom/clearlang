use clg_parser::parse;
use clg_typer::check;

fn expect_typer_error(src: &str) -> String {
    let ast = parse(src).expect("parse succeeds");
    let err = check(&ast).expect_err("type checker should report an error");
    format!("{err:#}")
}

#[test]
fn resource_use_after_consume_reports_t801() {
    let src = r#"
resource File {
    drop {}
}

function take(consume f: File) -> File { f }

function main(consume f: File) -> File {
    let tmp = take(f);
    f;
    tmp
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T801"), "unexpected error: {message}");
    assert!(message.contains("was consumed"));
}

#[test]
fn resource_consumed_twice_reports_use_after_consume() {
    let src = r#"
resource File {
    drop {}
}

function take(consume f: File) -> File { f }

function main(consume f: File) -> File {
    let first = take(f);
    let second = take(f);
    first
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T801"), "unexpected error: {message}");
    assert!(message.contains("was consumed"));
}

#[test]
fn resource_consume_while_borrowed_reports_t803() {
    let src = r#"
resource File {
    drop {}
}

function dual(file: File, consume taken: File) -> File { taken }

function main(consume f: File) -> File {
    dual(f, f);
    f
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T803"), "unexpected error: {message}");
    assert!(message.contains("cannot consume borrowed resource"));
}
