use lumi_parser::parse;
use lumi_typer::check;

// Arity mismatch: too many args
#[test]
fn errors_on_arity_mismatch_too_many_args() {
    let src = r#"
        pure function add(a: Int, b: Int) -> Int { a + b }
        function main() -> Int { add(1, 2, 3) }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail arity check (too many)");
    assert!(format!("{err:#}").contains("arity mismatch"));
}

// Arity mismatch: zero-arg function called with one arg
#[test]
fn errors_on_arity_mismatch_zero_arg_fn_called_with_arg() {
    let src = r#"
        pure function f() -> Int { 1 }
        function main() -> Int { f(1) }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail arity check (zero-arg)");
    assert!(format!("{err:#}").contains("arity mismatch"));
}

// Return type mismatch: declared Int, body Bool
#[test]
fn errors_on_return_type_mismatch_int_decl_bool_body() {
    let src = r#"
        pure function bad() -> Int { true }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail return type mismatch");
    assert!(format!("{err:#}").contains("return type mismatch"));
}

// Binop type check: right operand must be Int
#[test]
fn errors_on_binop_right_operand_non_int() {
    let src = r#"
        pure function bad(a: Int) -> Int { a + true }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail right operand int check");
    let s = format!("{err:#}");
    assert!(s.contains("right operand must be Int"), "unexpected error: {}", s);
}

// Duplicate function names are rejected
#[test]
fn errors_on_duplicate_function_names() {
    let src = r#"
        pure function f(a: Int) -> Int { a }
        function f(b: Int) -> Int { b }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail duplicate function name");
    assert!(format!("{err:#}").contains("duplicate function"));
}

// Arg type mismatch on the second parameter (index 1)
#[test]
fn errors_on_second_arg_type_mismatch() {
    let src = r#"
        pure function add(x: Int, y: Int) -> Int { x + y }
        function main() -> Int { add(1, true) }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail arg type mismatch");
    assert!(format!("{err:#}").contains("arg 1 type mismatch"));
}

// Unknown function with namespaced callee should report a clear error with span
#[test]
fn errors_on_unknown_namespaced_function() {
    let src = r#"
        function main() -> Int { std::foo::bar(42) }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail unknown namespaced function");
    let s = format!("{err:#}");
    assert!(s.contains("unknown function"), "unexpected: {s}");
    assert!(s.contains("std::foo::bar"), "unexpected: {s}");
    assert!(s.contains("at ") && s.contains(".."), "missing span: {s}");
}

// Return type mismatch: declared String, body Int
#[test]
fn errors_on_return_type_mismatch_str_decl_int_body() {
    let src = r#"
        pure function bad() -> String { 123 }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail return type mismatch (String vs Int)");
    let s = format!("{err:#}");
    assert!(s.contains("return type mismatch"));
    assert!(s.contains("String"));
    assert!(s.contains("Int"));
    assert!(s.contains("at ") && s.contains(".."));
}

// Arg type mismatch using String
#[test]
fn errors_on_arg_type_mismatch_with_str() {
    let src = r#"
        pure function add(x: Int, y: Int) -> Int { x + y }
        function main() -> Int { add("hi", 2) }
    "#;
    let ast = parse(src).expect("parsed");
    let err = check(&ast).expect_err("should fail arg type mismatch with String");
    let s = format!("{err:#}");
    assert!(s.contains("type mismatch"));
    assert!(s.contains("String"));
    assert!(s.contains("Int"));
    assert!(s.contains("at ") && s.contains(".."));
}
