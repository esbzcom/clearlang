use clg_parser::parse;

#[test]
fn parses_struct_decl_and_field_access() {
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
    let program = parse(src).expect("parse ok");
    assert_eq!(program.structs.len(), 1);
    assert_eq!(program.structs[0].name, "Point");
}

#[test]
fn parses_enum_decl_and_variant_call() {
    let src = r#"
        enum Shape {
            Circle(Int),
            Empty
        }

        function main() -> Int {
            let _s = Shape::Circle(1);
            0
        }
    "#;
    let program = parse(src).expect("parse ok");
    assert_eq!(program.enums.len(), 1);
    assert_eq!(program.enums[0].name, "Shape");
}

#[test]
fn parses_enum_match_patterns() {
    let src = r#"
        enum Shape {
            Circle(Int),
            Empty
        }

        function main(s: Shape) -> Int {
            match s { Shape::Circle(r) => r, Shape::Empty => 0 }
        }
    "#;
    let program = parse(src).expect("parse ok");
    assert_eq!(program.enums.len(), 1);
}
