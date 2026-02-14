use clg_parser::parse;
use clg_typer::{type_check_only, TyperError};

fn type_err_code(src: &str, code: &str) {
    let ast = parse(src).expect("parse");
    let err = type_check_only(&ast).expect_err("expected type error");
    let te = err.downcast_ref::<TyperError>().expect("typer error");
    assert_eq!(te.code, code);
}

#[test]
fn trait_call_without_bound_errors() {
    let src = r#"
        interface Eq {
            pure function eq(a: Self, b: Self) -> Bool;
        }

        implementation Eq for Int {
            pure function eq(a: Int, b: Int) -> Bool { a == b }
        }

        pure function bad<T>(a: T, b: T) -> Bool {
            Eq::eq(a, b)
        }
    "#;
    type_err_code(src, "T237");
}

#[test]
fn trait_call_missing_impl_for_concrete_type_errors() {
    let src = r#"
        interface Eq {
            pure function eq(a: Self, b: Self) -> Bool;
        }

        struct Foo { value: Int; }

        function main() -> Bool {
            Eq::eq(Foo { value: 1 }, Foo { value: 2 })
        }
    "#;
    type_err_code(src, "T237");
}

#[test]
fn overlapping_impls_are_rejected() {
    let src = r#"
        interface Eq {
            pure function eq(a: Self, b: Self) -> Bool;
        }

        implementation Eq for Int {
            pure function eq(a: Int, b: Int) -> Bool { a == b }
        }

        implementation Eq for Int {
            pure function eq(a: Int, b: Int) -> Bool { a == b }
        }
    "#;
    type_err_code(src, "T236");
}

#[test]
fn impl_missing_non_default_trait_method_errors() {
    let src = r#"
        interface Eq {
            pure function eq(a: Self, b: Self) -> Bool;
        }

        implementation Eq for Int { }
    "#;
    type_err_code(src, "T233");
}

#[test]
fn trait_default_body_effect_mismatch_stronger_than_declared_errors() {
    let src = r#"
        interface Probe {
            pure function ping(a: Self, b: Self) -> Bool { std::env::time() == 0 }
        }

        implementation Probe for Int { }
    "#;
    type_err_code(src, "T249");
}

#[test]
fn trait_default_body_effect_mismatch_weaker_than_declared_errors() {
    let src = r#"
        interface Probe {
            io function ping(a: Self, b: Self) -> Bool { true }
        }

        implementation Probe for Int { }
    "#;
    type_err_code(src, "T249");
}

#[test]
fn impl_override_default_method_effect_mismatch_errors() {
    let src = r#"
        interface Probe {
            pure function ping(a: Self, b: Self) -> Bool { true }
        }

        implementation Probe for Int {
            io function ping(a: Int, b: Int) -> Bool { std::env::time() == 0 }
        }
    "#;
    type_err_code(src, "T235");
}

#[test]
fn explicit_type_arg_count_mismatch_errors() {
    let src = r#"
        pure function id<T>(x: T) -> T { x }

        function main() -> Int {
            id<Int, Bool>(1)
        }
    "#;
    type_err_code(src, "T242");
}

#[test]
fn explicit_type_args_on_nongeneric_call_error() {
    let src = r#"
        pure function inc(x: Int) -> Int { x + 1 }

        function main() -> Int {
            inc<Int>(1)
        }
    "#;
    type_err_code(src, "T242");
}
