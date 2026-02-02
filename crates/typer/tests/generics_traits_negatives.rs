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
        trait Eq {
            pure function eq(a: Self, b: Self) -> Bool;
        }

        impl Eq for Int {
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
        trait Eq {
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
        trait Eq {
            pure function eq(a: Self, b: Self) -> Bool;
        }

        impl Eq for Int {
            pure function eq(a: Int, b: Int) -> Bool { a == b }
        }

        impl Eq for Int {
            pure function eq(a: Int, b: Int) -> Bool { a == b }
        }
    "#;
    type_err_code(src, "T236");
}
