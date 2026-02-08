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

#[test]
fn collection_call_use_after_move_reports_t801_with_call_context() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume files: List<File>, consume f: File) -> Int {
    let next = std::list::push(files, f);
    std::list::len(files)
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T801"), "unexpected error: {message}");
    assert!(message.contains("std::list::len"), "unexpected error: {message}");
}

#[test]
fn collection_call_double_consume_reports_t802_with_call_context() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume files: List<File>, consume a: File, consume b: File) -> List<File> {
    let next = std::list::push(files, a);
    std::list::push(files, b)
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T802"), "unexpected error: {message}");
    assert!(message.contains("std::list::push"), "unexpected error: {message}");
}

#[test]
fn collection_call_consume_borrow_reports_t803_with_call_context() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume files: List<File>, file: File) -> List<File> {
    std::list::push(files, file)
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T803"), "unexpected error: {message}");
    assert!(message.contains("std::list::push"), "unexpected error: {message}");
}

#[test]
fn linear_list_get_then_remove_is_allowed() {
    let src = r#"
resource File {
    drop {}
}

function ok(consume files: List<File>) -> List<File> {
    let head = std::list::get(files, 0);
    std::list::remove(files, 0)
}
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("get should be borrow-read; remove should consume");
}

#[test]
fn linear_list_remove_then_get_old_binding_reports_t801() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume files: List<File>) -> Option<File> {
    let next = std::list::remove(files, 0);
    std::list::get(files, 0)
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T801"), "unexpected error: {message}");
    assert!(message.contains("std::list::get"), "unexpected error: {message}");
}

#[test]
fn linear_map_contains_and_get_are_borrow_reads() {
    let src = r#"
resource File {
    drop {}
}

function ok(consume files: Map<Int, File>) -> Map<Int, File> {
    let has = std::map::contains(files, 1);
    let got = std::map::get(files, 1);
    files
}
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("contains/get should not consume linear map");
}

#[test]
fn linear_map_insert_then_reuse_old_binding_reports_t801() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume files: Map<Int, File>, consume f: File) -> Bool {
    let updated = std::map::insert(files, 1, f);
    std::map::contains(files, 1)
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T801"), "unexpected error: {message}");
    assert!(message.contains("std::map::contains"), "unexpected error: {message}");
}

#[test]
fn linear_collection_if_join_mismatch_reports_t804() {
    let src = r#"
resource File {
    drop {}
}

function bad(flag: Bool, consume files: List<File>) -> List<File> {
    if flag {
        std::list::remove(files, 0)
    } else {
        std::list::new()
    }
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T804"), "unexpected error: {message}");
}

#[test]
fn linear_collection_if_join_when_both_branches_consume_is_allowed() {
    let src = r#"
resource File {
    drop {}
}

function ok(flag: Bool, consume files: List<File>) -> List<File> {
    if flag {
        std::list::remove(files, 0)
    } else {
        std::list::remove(files, 0)
    }
}
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("both branches consume list consistently");
}

#[test]
fn linear_collection_match_join_mismatch_reports_t804() {
    let src = r#"
resource File {
    drop {}
}

function bad(opt: Option<Int>, consume files: List<File>) -> List<File> {
    match opt {
        Some(v) => std::list::remove(files, 0),
        None => files
    }
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T804"), "unexpected error: {message}");
}
