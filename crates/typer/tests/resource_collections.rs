use clg_parser::parse;
use clg_typer::check;

fn expect_typer_error(src: &str) -> String {
    let ast = parse(src).expect("parse succeeds");
    let err = check(&ast).expect_err("type checker should report an error");
    format!("{err:#}")
}

#[test]
fn list_of_resource_in_params_is_rejected() {
    let src = r#"
resource File {
    drop {}
}

function count(files: List<File>) -> Int { 0 }
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T806"), "unexpected error: {message}");
    assert!(
        message.contains("List<File>"),
        "expected container type in message: {message}"
    );
}

#[test]
fn map_with_nested_resource_is_rejected() {
    let src = r#"
resource File {
    drop {}
}

function uses_map(m: Map<Int, Option<File>>) -> Int { 0 }
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T806"), "unexpected error: {message}");
    assert!(
        message.contains("Map<Int, Option<File>>"),
        "expected container type in message: {message}"
    );
}

#[test]
fn resource_field_cannot_store_resource_collection() {
    let src = r#"
resource File {
    drop {}
}

resource Bucket {
    items: List<File>;
    drop {}
}

function main(consume f: File) -> File { f }
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T806"), "unexpected error: {message}");
    assert!(
        message.contains("List<File>"),
        "expected container type in message: {message}"
    );
}
