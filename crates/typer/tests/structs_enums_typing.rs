use clg_parser::parse;
use clg_typer::{type_check_only, TyperError};

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
fn struct_literal_and_field_access_typecheck() {
    let src = r#"
        struct Point {
            x: Int;
            y: Int;
        }

        function main() -> Int {
            let p = Point { x: 1, y: 2 };
            p.x
        }
    "#;
    type_ok(src);
}

#[test]
fn struct_literal_missing_field_errors() {
    let src = r#"
        struct Point {
            x: Int;
            y: Int;
        }

        function main() -> Int {
            let _p = Point { x: 1 };
            0
        }
    "#;
    type_err_code(src, "T212");
}

#[test]
fn struct_literal_unknown_field_errors() {
    let src = r#"
        struct Point {
            x: Int;
            y: Int;
        }

        function main() -> Int {
            let _p = Point { x: 1, z: 2 };
            0
        }
    "#;
    type_err_code(src, "T211");
}

#[test]
fn enum_variant_constructor_typecheck() {
    let src = r#"
        enum Shape {
            Circle(Int),
            Empty
        }

        function make() -> Shape {
            Shape::Circle(1)
        }
    "#;
    type_ok(src);
}

#[test]
fn enum_variant_arity_mismatch_errors() {
    let src = r#"
        enum Shape {
            Circle(Int)
        }

        function make() -> Shape {
            Shape::Circle(1, 2)
        }
    "#;
    type_err_code(src, "T216");
}

#[test]
fn enum_match_exhaustive_typecheck() {
    let src = r#"
        enum Shape {
            Circle(Int),
            Empty
        }

        function area(s: Shape) -> Int {
            match s {
                Shape::Circle(r) => r,
                Shape::Empty => 0
            }
        }
    "#;
    type_ok(src);
}

#[test]
fn enum_match_non_exhaustive_errors() {
    let src = r#"
        enum Shape {
            Circle(Int),
            Empty
        }

        function area(s: Shape) -> Int {
            match s { Shape::Circle(r) => r }
        }
    "#;
    type_err_code(src, "T201");
}

#[test]
fn option_match_with_wildcard_typecheck() {
    let src = r#"
        function f(m: Option<Int>) -> Int {
            match m { Some(x) => x, _ => 0 }
        }
    "#;
    type_ok(src);
}

#[test]
fn match_unreachable_arm_reports_t209() {
    let src = r#"
        enum Shape {
            Circle(Int),
            Empty
        }

        function area(s: Shape) -> Int {
            match s {
                _ => 0,
                Shape::Empty => 1
            }
    }
    "#;
    type_err_code(src, "T209");
}

#[test]
fn non_resource_struct_with_resource_field_errors_t218() {
    let src = r#"
        resource File { drop {} }

        struct Holder {
            file: File;
        }

        function main(consume f: File) -> File { f }
    "#;
    type_err_code(src, "T218");
}

#[test]
fn resource_struct_allows_resource_fields() {
    let src = r#"
        resource File { drop {} }

        resource struct Holder {
            file: File;
        }

        function take(consume h: Holder) -> Holder { h }
    "#;
    type_ok(src);
}

#[test]
fn non_resource_enum_with_resource_variant_field_errors_t218() {
    let src = r#"
        resource File { drop {} }

        enum Holder {
            Has(File),
            Empty
        }

        function main(consume f: File) -> File { f }
    "#;
    type_err_code(src, "T218");
}

#[test]
fn resource_enum_allows_resource_variant_fields() {
    let src = r#"
        resource File { drop {} }

        resource enum Holder {
            Has(File),
            Empty
        }

        function take(consume h: Holder) -> Holder { h }
    "#;
    type_ok(src);
}
