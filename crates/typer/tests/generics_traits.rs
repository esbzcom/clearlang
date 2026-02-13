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
    assert!(names.iter().any(|name| *name == "id$Int"));
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
    assert!(names.iter().any(|name| *name == "eq_pair$Int"));
    assert!(names.iter().any(|name| *name == "impl$Eq$Int$eq"));
}

#[test]
fn monomorphizes_trait_default_method_when_impl_omits_it() {
    let src = r#"
        trait Eq {
            pure function eq(a: Self, b: Self) -> Bool { a == b }
        }

        impl Eq for Int { }

        function main() -> Bool {
            Eq::eq(1, 1)
        }
    "#;
    let ast = parse(src).expect("parse");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let names: Vec<&str> = output.ir.funcs.iter().map(|f| f.name.as_str()).collect();
    assert!(names.iter().any(|name| *name == "impl$Eq$Int$eq"));
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

#[test]
fn impl_with_inline_bounds_works() {
    let src = r#"
        trait Eq {
            pure function eq(a: Self, b: Self) -> Bool;
        }

        impl Eq for Int {
            pure function eq(a: Int, b: Int) -> Bool { a == b }
        }

        struct Box<T> { value: T; }

        impl<T: Eq> Eq for Box<T> {
            pure function eq(a: Box<T>, b: Box<T>) -> Bool {
                Eq::eq(a.value, b.value)
            }
        }

        function main() -> Bool {
            Eq::eq(Box { value: 1 }, Box { value: 2 })
        }
    "#;
    let ast = parse(src).expect("parse");
    let _output = check_with_vcs(&ast).expect("type-check ok");
}

#[test]
fn impl_with_where_bounds_works() {
    let src = r#"
        trait Eq {
            pure function eq(a: Self, b: Self) -> Bool;
        }

        impl Eq for Int {
            pure function eq(a: Int, b: Int) -> Bool { a == b }
        }

        struct Box<T> { value: T; }

        impl<T> Eq for Box<T> where T: Eq {
            pure function eq(a: Box<T>, b: Box<T>) -> Bool {
                Eq::eq(a.value, b.value)
            }
        }

        function main() -> Bool {
            Eq::eq(Box { value: 1 }, Box { value: 2 })
        }
    "#;
    let ast = parse(src).expect("parse");
    let _output = check_with_vcs(&ast).expect("type-check ok");
}

#[test]
fn monomorphizes_trait_calls_in_contracts_and_invariants() {
    let src = r#"
        trait Eq {
            pure function eq(a: Self, b: Self) -> Bool;
        }

        impl Eq for Int {
            pure function eq(a: Int, b: Int) -> Bool { a == b }
        }

        pure function spec_only<T: Eq>(a: T, b: T) -> T
            require { Eq::eq(a, b) }
            ensure { Eq::eq(result, a) }
        {
            a
        }

        function loop_inv(n: Int) -> Int {
            while n > 0 invariant { Eq::eq(n, n) } variant { n } { n }
            n
        }

        function main() -> Int {
            spec_only(1, 1);
            loop_inv(1)
        }
    "#;
    let ast = parse(src).expect("parse");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let names: Vec<&str> = output.ir.funcs.iter().map(|f| f.name.as_str()).collect();
    assert!(names.iter().any(|name| *name == "impl$Eq$Int$eq"));
}

#[test]
fn lowers_generic_enum_multi_field_match() {
    let src = r#"
        enum Pair<T> {
            Two(T, T),
            Empty
        }

        function sum(p: Pair<Int>) -> Int {
            match p {
                Pair::Two(a, b) => a + b,
                Pair::Empty => 0
            }
        }
    "#;
    let ast = parse(src).expect("parse");
    let _output = check_with_vcs(&ast).expect("type-check ok");
}

#[test]
fn monomorphizes_nested_generic_calls() {
    let src = r#"
        pure function wrap<T>(x: T) -> T { x }

        pure function make() -> Result<Option<Int>, Bool> {
            Ok(Some(1))
        }

        function main() -> Result<Option<Int>, Bool> {
            wrap(make())
        }
    "#;
    let ast = parse(src).expect("parse");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let names: Vec<&str> = output.ir.funcs.iter().map(|f| f.name.as_str()).collect();
    assert!(names
        .iter()
        .any(|name| *name == "wrap$Result$Option$Int$Bool"));
}

#[test]
fn mangling_preserves_refined_alias_names() {
    let src = r#"
        type Nat = Int where n >= 0;

        pure function id<T>(x: T) -> T { x }

        pure function use(n: Nat) -> Nat {
            id(n)
        }
    "#;
    let ast = parse(src).expect("parse");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let names: Vec<&str> = output.ir.funcs.iter().map(|f| f.name.as_str()).collect();
    assert!(names.iter().any(|name| *name == "id$Nat"));
}
