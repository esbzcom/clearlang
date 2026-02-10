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

#[test]
fn resource_consume_and_return_succeeds() {
    let src = r#"
resource File {
    drop {}
}

function take(consume f: File) -> File { f }

function main(consume f: File) -> File {
    let out = take(f);
    out
}
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("type check succeeds");
}

#[test]
fn option_constructor_consumes_resource_param() {
    let src = r#"
resource File {
    drop {}
}

function main(consume f: File) -> Option<File> {
    Some(f)
}
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("Some(...) should consume owned resource args");
}

#[test]
fn tuple_literal_consumes_resource_param() {
    let src = r#"
resource File {
    drop {}
}

function main(consume f: File) -> (File, Int) {
    (f, 0)
}
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("tuple literals should consume owned resource elems");
}

#[test]
fn tuple_literal_duplicate_resource_reports_t801() {
    let src = r#"
resource File {
    drop {}
}

function main(consume f: File) -> (File, File) {
    (f, f)
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T801"), "unexpected error: {message}");
}

#[test]
fn result_ok_constructor_consumes_resource_param() {
    let src = r#"
resource File {
    drop {}
}

function main(consume f: File) -> Result<File, Int> {
    Ok(f)
}
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("Ok(...) should consume owned resource args");
}

#[test]
fn borrowed_resource_can_be_used_without_consuming() {
    let src = r#"
resource File {
    drop {}
}

function size(file: File) -> Int { 0 }
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("type check succeeds");
}
#[test]
fn resource_branch_mismatch_reports_t804_in_if() {
    let src = r#"
resource File {
    drop {}
}

function take(consume f: File) -> File { f }

function choose(consume f: File, flag: Bool) -> Int {
    if flag {
        take(f);
        0
    } else {
        0
    }
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T804"), "unexpected error: {message}");
}

#[test]
fn resource_branch_mismatch_reports_t804_in_match() {
    let src = r#"
resource File {
    drop {}
}

function take(consume f: File) -> File { f }

function choose(opt: Option<File>, consume fallback: File) -> Int {
    match opt {
        Some(f) => {
            take(f);
            0
        },
        None => {
            take(fallback);
            0
        }
    }
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T804"), "unexpected error: {message}");
}
