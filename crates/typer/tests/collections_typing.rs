use lumi_parser::parse;
use lumi_typer::{type_check_only, TyperError};

fn type_ok(src: &str) {
    let ast = parse(src).expect("parse");
    type_check_only(&ast).expect("type ok");
}

fn type_err_code(src: &str, code: &str) {
    let ast = parse(src).expect("parse");
    let err = type_check_only(&ast).expect_err("expected type error");
    let te = err.downcast_ref::<TyperError>().expect("typer error");
    assert_eq!(te.code, code);
}

#[test]
fn list_len_types() {
    let src = r#"
        pure function len(l: List<Int>) -> Int { std::list::len(l) }
    "#;
    type_ok(src);
}

#[test]
fn list_push_types() {
    let src = r#"
        pure function push(l: List<Int>) -> List<Int> { std::list::push(l, 1) }
    "#;
    type_ok(src);
}

#[test]
fn list_pop_types() {
    let src = r#"
        pure function pop(l: List<Int>) -> Option<Int> { std::list::pop(l) }
    "#;
    type_ok(src);
}

#[test]
fn list_get_types() {
    let src = r#"
        pure function get0(l: List<Int>) -> Option<Int> { std::list::get(l, 0) }
    "#;
    type_ok(src);
}

#[test]
fn list_insert_types() {
    let src = r#"
        pure function ins(l: List<Int>) -> List<Int> { std::list::insert(l, 1, 0) }
    "#;
    type_ok(src);
}

#[test]
fn list_remove_types() {
    let src = r#"
        pure function rm(l: List<Int>) -> List<Int> { std::list::remove(l, 0) }
    "#;
    type_ok(src);
}

#[test]
fn set_contains_types() {
    let src = r#"
        function c(s: Set<String>) -> Bool { std::set::contains(s, "hi") }
    "#;
    type_ok(src);
}

#[test]
fn map_get_types() {
    let src = r#"
        function g(m: Map<Int, String>) -> Option<String> { std::map::get(m, 1) }
    "#;
    type_ok(src);
}

#[test]
fn list_push_element_mismatch() {
    let src = r#"
        function bad(l: List<Int>) -> List<Int> { std::list::push(l, true) }
    "#;
    type_err_code(src, "T208");
}

#[test]
fn list_index_type_mismatch() {
    let src = r#"
        function bad(l: List<Int>) -> Option<Int> { std::list::get(l, true) }
    "#;
    type_err_code(src, "T005");
}

#[test]
fn map_get_key_mismatch() {
    let src = r#"
        function bad(m: Map<Int, String>) -> Option<String> { std::map::get(m, "key") }
    "#;
    type_err_code(src, "T208");
}

#[test]
fn new_cannot_infer() {
    let src = r#"
        function bad() -> Int { std::list::len(std::list::new()) }
    "#;
    type_err_code(src, "T206");
}
