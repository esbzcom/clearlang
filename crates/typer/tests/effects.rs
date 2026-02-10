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
