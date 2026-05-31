use clg_parser::parse;
use clg_typer::type_check_only;

#[test]
fn pure_function_cannot_call_mut_builtin() {
    let src = r#"
        pure function bad(l: List<Int>) -> List<Int> {
            std::list::push_mut(l, 1)
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = type_check_only(&ast).expect_err("pure call to mut intrinsic must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("requires `mut` effect"),
        "expected mut effect error, got {msg}"
    );
}

#[test]
fn mut_function_requires_guard_for_mut_builtin() {
    let src = r#"
        mut function ok(l: List<Int>) -> List<Int>
            require { std::list::can_mut(l) }
        {
            std::list::push_mut(l, 1)
        }
    "#;
    type_check_only(&parse(src).expect("parse ok")).expect("mut call should succeed");
}

#[test]
fn mut_function_without_guard_fails() {
    let src = r#"
        mut function bad(l: List<Int>) -> List<Int> {
            std::list::push_mut(l, 1)
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = type_check_only(&ast).expect_err("missing guard must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("requires `std::list::can_mut"),
        "expected guard error, got {msg}"
    );
}

#[test]
fn mut_map_function_requires_guard_for_mut_builtin() {
    let src = r#"
        mut function ok(m: Map<Int, Int>) -> Map<Int, Int>
            require { std::map::can_mut(m) }
        {
            std::map::insert_mut(m, 1, 10)
        }
    "#;
    type_check_only(&parse(src).expect("parse ok")).expect("mut map call should succeed");
}

#[test]
fn mut_map_function_without_guard_fails() {
    let src = r#"
        mut function bad(m: Map<Int, Int>) -> Map<Int, Int> {
            std::map::remove_mut(m, 1)
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = type_check_only(&ast).expect_err("missing map mut guard must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("requires `std::map::can_mut"),
        "expected map guard error, got {msg}"
    );
}

#[test]
fn mut_list_insert_remove_and_pop_aliases_require_list_guard() {
    let src = r#"
        mut function insert_ok(l: List<Int>) -> List<Int>
            require { std::list::can_mut(l) }
        {
            std::list::insert_mut(l, 1, 0)
        }
        mut function remove_ok(l: List<Int>) -> List<Int>
            require { std::list::can_mut(l) }
        {
            std::list::remove_mut(l, 0)
        }
        mut function pop_ok(l: List<Int>) -> Option<Int>
            require { std::list::can_mut(l) }
        {
            std::list::pop_mut(l)
        }
    "#;
    type_check_only(&parse(src).expect("parse ok"))
        .expect("list mut aliases should succeed when guard is present");
}

#[test]
fn mut_set_function_requires_guard_for_mut_builtins() {
    let src = r#"
        mut function insert_ok(s: Set<Int>) -> Set<Int>
            require { std::set::can_mut(s) }
        {
            std::set::insert_mut(s, 1)
        }
        mut function remove_ok(s: Set<Int>) -> Set<Int>
            require { std::set::can_mut(s) }
        {
            std::set::remove_mut(s, 1)
        }
    "#;
    type_check_only(&parse(src).expect("parse ok")).expect("mut set call should succeed");
}

#[test]
fn mut_set_function_without_guard_fails() {
    let src = r#"
        mut function bad(s: Set<Int>) -> Set<Int> {
            std::set::insert_mut(s, 1)
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = type_check_only(&ast).expect_err("missing set mut guard must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("requires `std::set::can_mut"),
        "expected set guard error, got {msg}"
    );
}

#[test]
fn mut_guard_requires_variable_target() {
    let src = r#"
        mut function bad(l: List<Int>) -> List<Int>
            require { std::list::can_mut(l) }
        {
            std::list::push_mut(std::list::push(l, 1), 1)
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = type_check_only(&ast).expect_err("non-variable guard target must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("requires its first argument to be a variable"),
        "expected T403 variable-target guard error, got {msg}"
    );
}

#[test]
fn pure_function_cannot_call_mut_user_function() {
    let src = r#"
        mut function helper(l: List<Int>) -> List<Int>
            require { std::list::can_mut(l) }
        {
            std::list::push_mut(l, 1)
        }
        pure function caller(l: List<Int>) -> List<Int> {
            helper(l)
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = type_check_only(&ast).expect_err("pure caller must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("requires `mut` effect"),
        "expected mut effect error, got {msg}"
    );
}

#[test]
fn pure_function_cannot_call_io_builtin() {
    let src = r#"
        pure function bad() -> Int {
            std::wasi::print(std::bytes::from_string("hi"))
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = type_check_only(&ast).expect_err("pure call to io intrinsic must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("requires `io` effect"),
        "expected io effect error, got {msg}"
    );
}

#[test]
fn mut_function_cannot_call_io_builtin() {
    let src = r#"
        mut function bad() -> Int {
            std::wasi::print(std::bytes::from_string("hi"))
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = type_check_only(&ast).expect_err("mut call to io intrinsic must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("requires `io` effect"),
        "expected io effect error, got {msg}"
    );
}

#[test]
fn pure_function_cannot_call_env_time() {
    let src = r#"
        pure function bad() -> Int { std::env::time() }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = type_check_only(&ast).expect_err("pure call to env time must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("requires `io` effect"),
        "expected io effect error, got {msg}"
    );
}

#[test]
fn mut_function_cannot_call_env_random() {
    let src = r#"
        mut function bad() -> Int {
            std::bytes::len(std::env::random(4))
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = type_check_only(&ast).expect_err("mut call to env random must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("requires `io` effect"),
        "expected io effect error, got {msg}"
    );
}

#[test]
fn pure_function_cannot_call_crypto_hash() {
    let src = r#"
        pure function bad() -> Bytes {
            std::crypto::hash("sha256", std::bytes::from_string("hi"))
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = type_check_only(&ast).expect_err("pure call to crypto hash must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("requires `io` effect"),
        "expected io effect error, got {msg}"
    );
}

#[test]
fn pure_function_can_call_linear_move_out_api() {
    let src = r#"
        resource File { drop {} }
        pure function step(consume files: List<File>) -> List<File> {
            std::list::remove_take(files, 0)[0]
        }
    "#;
    type_check_only(&parse(src).expect("parse ok"))
        .expect("linear move-out API should remain pure-by-construction");
}

#[test]
fn mut_function_can_call_linear_move_out_api_without_mut_guard() {
    let src = r#"
        resource File { drop {} }
        mut function step(consume files: List<File>) -> List<File> {
            std::list::remove_take(files, 0)[0]
        }
    "#;
    type_check_only(&parse(src).expect("parse ok"))
        .expect("pure linear API should not require can_mut guards");
}

#[test]
fn pure_function_cannot_call_mut_linear_collection_alias() {
    let src = r#"
        resource File { drop {} }
        pure function bad(consume files: List<File>, consume f: File) -> List<File>
            require { std::list::can_mut(files) }
        {
            std::list::push_mut(files, f)
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let err = type_check_only(&ast).expect_err("mut alias must still require mut effect");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("requires `mut` effect"),
        "expected mut effect error, got {msg}"
    );
}
