use clg_parser::parse;
use clg_typer::check_with_vcs;

#[test]
fn monomorphizes_generic_function_calls() {
    let src = r#"
        pure function id<T>(x: T) -> T { x }

        function main() -> Int {
            id(1)
        }
    "#;
    let ast = parse(src).expect("parse");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let names: Vec<&str> = output.ir.funcs.iter().map(|f| f.name.as_str()).collect();
    assert!(names.iter().any(|name| *name == "id<Int>"));
    assert!(names.iter().any(|name| *name == "main"));
}

#[test]
fn monomorphizes_trait_impl_calls() {
    let src = r#"
        trait Eq {
            pure function eq(a: Self, b: Self) -> Bool;
        }

        impl Eq for Int {
            pure function eq(a: Int, b: Int) -> Bool { a == b }
        }

        pure function eq_pair<T: Eq>(a: T, b: T) -> Bool {
            Eq::eq(a, b)
        }

        function main() -> Bool {
            eq_pair(1, 2)
        }
    "#;
    let ast = parse(src).expect("parse");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let names: Vec<&str> = output.ir.funcs.iter().map(|f| f.name.as_str()).collect();
    assert!(names.iter().any(|name| *name == "eq_pair<Int>"));
    assert!(names.iter().any(|name| *name == "impl::Eq::<Int>::eq"));
}

#[test]
fn lowers_generic_structs_and_enums() {
    let src = r#"
        struct Box<T> {
            value: T;
        }

        enum Maybe<T> {
            Just(T),
            Empty
        }

        function unwrap(boxed: Box<Int>, opt: Maybe<Int>) -> Int {
            let v = boxed.value;
            match opt {
                Maybe::Just(x) => x,
                Maybe::Empty => v
            }
        }
    "#;
    let ast = parse(src).expect("parse");
    let _output = check_with_vcs(&ast).expect("type-check ok");
}
