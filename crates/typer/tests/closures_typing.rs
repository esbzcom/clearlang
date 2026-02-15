use clg_parser::parse;
use clg_typer::type_check_only;

fn expect_typer_error(src: &str) -> String {
    let ast = parse(src).expect("parse succeeds");
    let err = type_check_only(&ast).expect_err("type checker should report an error");
    format!("{err:#}")
}

#[test]
fn lambda_return_inference_with_non_resource_capture_typechecks() {
    let src = r#"
function make_adder(base: Int) -> function(Int) -> Int {
    (x: Int) => x + base
}
"#;

    let ast = parse(src).expect("parse succeeds");
    type_check_only(&ast).expect("typed lambda with lexical capture should typecheck");
}

#[test]
fn closure_parameter_call_typechecks() {
    let src = r#"
io function apply(f: function(Int) -> Int, x: Int) -> Int { f(x) }
io function main() -> Int { apply((n: Int) => n + 1, 41) }
"#;

    let ast = parse(src).expect("parse succeeds");
    type_check_only(&ast).expect("calling function-typed params should typecheck");
}

#[test]
fn closure_call_arg_mismatch_reports_t003() {
    let src = r#"
function bad_apply(f: function(Int) -> Int) -> Int { f(true) }
"#;

    let msg = expect_typer_error(src);
    assert!(msg.contains("T003"), "unexpected error: {msg}");
    assert!(
        msg.contains("arg 0 type mismatch"),
        "unexpected error: {msg}"
    );
}

#[test]
fn capturing_resource_value_in_lambda_is_rejected() {
    let src = r#"
resource File { drop {} }

function touch(f: File) -> Int { 0 }

function bad(consume f: File) -> function(Int) -> Int {
    (x: Int) => x + touch(f)
}
"#;

    let msg = expect_typer_error(src);
    assert!(msg.contains("T017"), "unexpected error: {msg}");
    assert!(
        msg.contains("capturing resource value `f` in closures"),
        "unexpected error: {msg}"
    );
}

#[test]
fn capturing_resource_struct_value_in_lambda_is_rejected() {
    let src = r#"
resource File { drop {} }

resource struct Holder {
    file: File;
}

function score(h: Holder) -> Int { 1 }

function bad(consume h: Holder) -> function(Int) -> Int {
    (x: Int) => x + score(h)
}
"#;

    let msg = expect_typer_error(src);
    assert!(msg.contains("T017"), "unexpected error: {msg}");
    assert!(
        msg.contains("capturing resource value `h` in closures"),
        "unexpected error: {msg}"
    );
}

#[test]
fn capturing_resource_enum_value_in_lambda_is_rejected() {
    let src = r#"
resource File { drop {} }

resource enum Holder {
    One(File),
    Empty
}

function score(h: Holder) -> Int { 1 }

function bad(consume h: Holder) -> function(Int) -> Int {
    (x: Int) => x + score(h)
}
"#;

    let msg = expect_typer_error(src);
    assert!(msg.contains("T017"), "unexpected error: {msg}");
    assert!(
        msg.contains("capturing resource value `h` in closures"),
        "unexpected error: {msg}"
    );
}

#[test]
fn self_referential_closure_value_is_rejected_in_v1() {
    let src = r#"
function bad() -> function(Int) -> Int {
    let f = (x: Int) => f(x);
    f
}
"#;

    let msg = expect_typer_error(src);
    assert!(msg.contains("T017"), "unexpected error: {msg}");
    assert!(
        msg.contains("self-referential closure value `f`"),
        "unexpected error: {msg}"
    );
}

#[test]
fn mutually_recursive_closure_values_are_rejected_in_v1() {
    let src = r#"
function bad() -> Int {
    let f = (x: Int) => g(x);
    let g = (x: Int) => f(x);
    0
}
"#;

    let msg = expect_typer_error(src);
    assert!(msg.contains("T017"), "unexpected error: {msg}");
    assert!(
        msg.contains("mutually recursive closure values `f` and `g`"),
        "unexpected error: {msg}"
    );
}

#[test]
fn mutually_recursive_closure_values_use_canonical_name_order() {
    let src = r#"
function bad() -> Int {
    let g = (x: Int) => f(x);
    let f = (x: Int) => g(x);
    0
}
"#;

    let msg = expect_typer_error(src);
    assert!(msg.contains("T017"), "unexpected error: {msg}");
    assert!(
        msg.contains("mutually recursive closure values `f` and `g`"),
        "unexpected error: {msg}"
    );
}

#[test]
fn lambda_body_effect_must_fit_enclosing_function_effect() {
    let src = r#"
pure function bad() -> function() -> Int {
    () => std::wasi::print(std::bytes::from_string("hi"))
}
"#;

    let msg = expect_typer_error(src);
    assert!(msg.contains("T401"), "unexpected error: {msg}");
    assert!(
        msg.contains("requires `io` effect"),
        "unexpected error: {msg}"
    );
}

#[test]
fn function_typed_param_call_requires_conservative_effect() {
    let src = r#"
pure function apply(f: function(Int) -> Int, x: Int) -> Int { f(x) }
"#;

    let msg = expect_typer_error(src);
    assert!(msg.contains("T401"), "unexpected error: {msg}");
    assert!(
        msg.contains("requires `io` effect"),
        "unexpected error: {msg}"
    );
}

#[test]
fn local_pure_lambda_call_remains_pure() {
    let src = r#"
pure function use_local(x: Int) -> Int {
    let f = (n: Int) => n + 1;
    let g = f;
    g(x)
}
"#;

    let ast = parse(src).expect("parse succeeds");
    type_check_only(&ast).expect("known local pure closures should remain callable from pure code");
}
