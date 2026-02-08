use clg_ast::Type;
use clg_parser::parse;

#[test]
fn parses_struct_trait_impl_and_generic_types() {
    let src = r#"
        struct Box<T> {
            value: T;
        }

        trait Eq {
            pure function eq(a: Self, b: Self) -> Bool;
        }

        impl Eq for Int {
            pure function eq(a: Int, b: Int) -> Bool { a == b }
        }

        function wrap(x: Int) -> Box<Int> {
            Box { value: x }
        }
    "#;

    let program = parse(src).expect("parse ok");
    assert_eq!(program.structs.len(), 1);
    assert_eq!(program.structs[0].type_params.len(), 1);
    assert_eq!(program.traits.len(), 1);
    assert_eq!(program.impls.len(), 1);
    assert_eq!(program.funcs.len(), 1);

    let ret_ty = &program.funcs[0].ret;
    assert!(matches!(
        ret_ty,
        Type::Named { name, args } if name == "Box" && matches!(args.as_slice(), [Type::Int])
    ));
}

#[test]
fn parses_where_bounds_on_functions() {
    let src = r#"
        trait Eq {
            pure function eq(a: Self, b: Self) -> Bool;
        }

        function eq_pair<T: Eq>(a: T, b: T) -> Bool
            where T: Eq
        {
            Eq::eq(a, b)
        }
    "#;

    let program = parse(src).expect("parse ok");
    assert_eq!(program.funcs.len(), 1);
    let func = &program.funcs[0];
    assert_eq!(func.type_params.len(), 1);
    assert_eq!(func.where_bounds.len(), 2);
    assert!(func
        .where_bounds
        .iter()
        .any(|b| b.param == "T" && b.trait_name == "Eq"));
}
