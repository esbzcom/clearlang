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
fn list_of_resource_struct_in_params_is_allowed() {
    let src = r#"
resource File {
    drop {}
}

resource struct Holder {
    file: File;
}

function count(items: List<Holder>) -> Int { std::list::len(items) }
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("List<resource struct> params should type-check");
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
fn map_with_resource_enum_value_is_allowed() {
    let src = r#"
resource File {
    drop {}
}

resource enum Holder {
    One(File),
    Empty
}

function uses_map(m: Map<Int, Holder>) -> Int { std::map::len(m) }
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("Map<K, resource enum> params should type-check");
}

#[test]
fn tuple_wrapping_linear_collection_outputs_is_allowed() {
    let src = r#"
resource File {
    drop {}
}

function step(consume files: List<File>) -> (List<File>, Option<File>) {
    std::list::remove_take(files, 0)
}
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("tuple wrappers around linear outputs should type-check");
}

#[test]
fn consume_tuple_param_with_resources_must_be_consumed() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume pair: (List<File>, Option<File>)) -> Int {
    0
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T805"), "unexpected error: {message}");
    assert!(message.contains("pair"), "unexpected error: {message}");
}

#[test]
fn tuple_with_set_resource_is_still_rejected() {
    let src = r#"
resource File {
    drop {}
}

function bad(x: (Set<File>, Int)) -> Int { 0 }
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T806"), "unexpected error: {message}");
    assert!(message.contains("Set<File>"), "unexpected error: {message}");
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
    assert!(
        message.contains("std::list::len"),
        "unexpected error: {message}"
    );
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
    assert!(
        message.contains("std::list::push"),
        "unexpected error: {message}"
    );
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
    assert!(
        message.contains("std::list::push"),
        "unexpected error: {message}"
    );
}

#[test]
fn linear_list_get_reports_move_out_guidance() {
    let src = r#"
resource File {
    drop {}
}

function ok(consume files: List<File>) -> List<File> {
    let head = std::list::get(files, 0);
    files
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T806"), "unexpected error: {message}");
    assert!(
        message.contains("std::list::get"),
        "unexpected error: {message}"
    );
    assert!(
        message.contains("std::list::remove_take"),
        "unexpected error: {message}"
    );
}

#[test]
fn linear_list_remove_reports_move_out_guidance() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume files: List<File>) -> List<File> {
    std::list::remove(files, 0)
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T806"), "unexpected error: {message}");
    assert!(
        message.contains("std::list::remove"),
        "unexpected error: {message}"
    );
    assert!(
        message.contains("std::list::remove_take"),
        "unexpected error: {message}"
    );
}

#[test]
fn linear_list_pop_reports_move_out_guidance() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume files: List<File>) -> Option<File> {
    std::list::pop(files)
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T806"), "unexpected error: {message}");
    assert!(
        message.contains("std::list::pop"),
        "unexpected error: {message}"
    );
    assert!(
        message.contains("std::list::remove_take"),
        "unexpected error: {message}"
    );
}

#[test]
fn linear_map_get_reports_move_out_guidance() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume files: Map<Int, File>) -> Map<Int, File> {
    let has = std::map::contains(files, 1);
    let got = std::map::get(files, 1);
    files
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T806"), "unexpected error: {message}");
    assert!(
        message.contains("std::map::get"),
        "unexpected error: {message}"
    );
    assert!(
        message.contains("std::map::remove_take"),
        "unexpected error: {message}"
    );
}

#[test]
fn linear_map_get_with_nested_resource_reports_move_out_guidance() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume files: Map<Int, Option<File>>) -> Map<Int, Option<File>> {
    let got = std::map::get(files, 1);
    files
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T806"), "unexpected error: {message}");
    assert!(
        message.contains("std::map::get"),
        "unexpected error: {message}"
    );
    assert!(
        message.contains("Option<File>"),
        "unexpected error: {message}"
    );
    assert!(
        message.contains("std::map::remove_take"),
        "unexpected error: {message}"
    );
}

#[test]
fn linear_map_contains_is_still_allowed() {
    let src = r#"
resource File {
    drop {}
}

function ok(files: Map<Int, File>) -> Bool {
    std::map::contains(files, 1)
}
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("contains should remain a borrow-read on linear maps");
}

#[test]
fn linear_map_insert_reports_move_out_guidance() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume files: Map<Int, File>, consume f: File) -> Map<Int, File> {
    std::map::insert(files, 1, f)
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T806"), "unexpected error: {message}");
    assert!(
        message.contains("std::map::insert"),
        "unexpected error: {message}"
    );
    assert!(
        message.contains("std::map::insert_take"),
        "unexpected error: {message}"
    );
}

#[test]
fn linear_map_remove_reports_move_out_guidance() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume files: Map<Int, File>) -> Map<Int, File> {
    std::map::remove(files, 1)
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T806"), "unexpected error: {message}");
    assert!(
        message.contains("std::map::remove"),
        "unexpected error: {message}"
    );
    assert!(
        message.contains("std::map::remove_take"),
        "unexpected error: {message}"
    );
}

#[test]
fn linear_list_remove_take_then_use_old_binding_reports_t801() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume files: List<File>) -> Int {
    let out = std::list::remove_take(files, 0);
    std::list::len(files)
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T801"), "unexpected error: {message}");
    assert!(
        message.contains("std::list::len"),
        "unexpected error: {message}"
    );
}

#[test]
fn linear_list_remove_take_extracts_container() {
    let src = r#"
resource File {
    drop {}
}

function ok(consume files: List<File>) -> List<File> {
    std::list::remove_take(files, 0)[0]
}
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("remove_take should return updated list");
}

#[test]
fn linear_list_remove_take_bound_tuple_then_partial_use_reports_t805() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume files: List<File>) -> Option<File> {
    let out = std::list::remove_take(files, 0);
    out[1]
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T805"), "unexpected error: {message}");
    assert!(message.contains("out"), "unexpected error: {message}");
}

#[test]
fn linear_map_insert_take_extracts_container() {
    let src = r#"
resource File {
    drop {}
}

function ok(consume files: Map<Int, File>, consume f: File) -> Map<Int, File> {
    std::map::insert_take(files, 1, f)[0]
}
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("insert_take should return updated map");
}

#[test]
fn linear_map_remove_take_extracts_removed_value() {
    let src = r#"
resource File {
    drop {}
}

function ok(consume files: Map<Int, File>) -> Option<File> {
    std::map::remove_take(files, 1)[1]
}
"#;

    let ast = parse(src).expect("parse succeeds");
    check(&ast).expect("remove_take should return removed value option");
}

#[test]
fn option_resource_param_must_be_consumed() {
    let src = r#"
resource File {
    drop {}
}

function bad(consume maybe_file: Option<File>) -> Int {
    0
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T805"), "unexpected error: {message}");
    assert!(
        message.contains("maybe_file"),
        "unexpected error: {message}"
    );
}

#[test]
fn linear_collection_if_join_mismatch_reports_t804() {
    let src = r#"
resource File {
    drop {}
}

function take_files(consume files: List<File>) -> List<File> { files }

function bad(flag: Bool, consume files: List<File>) -> List<File> {
    if flag {
        take_files(files)
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

function take_files(consume files: List<File>) -> List<File> { files }

function ok(flag: Bool, consume files: List<File>) -> List<File> {
    if flag {
        take_files(files)
    } else {
        take_files(files)
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

function take_files(consume files: List<File>) -> List<File> { files }

function bad(opt: Option<Int>, consume files: List<File>) -> List<File> {
    match opt {
        Some(v) => take_files(files),
        None => std::list::new()
    }
}
"#;

    let message = expect_typer_error(src);
    assert!(message.contains("T804"), "unexpected error: {message}");
}
