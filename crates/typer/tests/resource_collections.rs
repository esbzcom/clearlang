use clg_parser::parse;
use clg_typer::check;

fn expect_typer_error(src: &str) -> String {
    let ast = parse(src).expect("parse succeeds");
    let err = check(&ast).expect_err("type checker should report an error");
    format!("{err:#}")
}

#[test]
fn list_of_resource_in_params_is_allowed() {
    let src = r#"
resource File {
    drop {}
}

function count(files: List<File>) -> Int { 0 }
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("List<Resource> params should type-check");
}

#[test]
fn map_with_nested_resource_value_is_allowed() {
    let src = r#"
resource File {
    drop {}
}

function uses_map(m: Map<Int, Option<File>>) -> Int { 0 }
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("Map<K, Resource> params should type-check");
}

#[test]
fn set_of_resource_is_still_rejected() {
    let src = r#"
resource File {
    drop {}
}

function bad_set(files: Set<File>) -> Int { 0 }
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T806"), "unexpected error: {message}");
    assert!(
        message.contains("Set<File>"),
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

#[test]
fn consuming_linear_list_twice_reports_t801() {
    let src = r#"
resource File {
    drop {}
}

function take_files(consume files: List<File>) -> List<File> { files }

function bad(consume files: List<File>) -> List<File> {
    let first = take_files(files);
    take_files(files)
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T801"), "unexpected error: {message}");
    assert!(message.contains("files"), "unexpected error: {message}");
}
