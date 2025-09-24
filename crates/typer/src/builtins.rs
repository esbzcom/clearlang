use clg_ast::{Param, Type};

pub(crate) fn builtin_sigs() -> Vec<(String, Vec<Param>, Type)> {
    vec![
        (
            "std::str::len".to_string(),
            vec![Param {
                name: "s".to_string(),
                ty: Type::String,
            }],
            Type::Int,
        ),
        (
            "std::str::concat".to_string(),
            vec![
                Param {
                    name: "a".to_string(),
                    ty: Type::String,
                },
                Param {
                    name: "b".to_string(),
                    ty: Type::String,
                },
            ],
            Type::String,
        ),
        (
            "std::str::eq".to_string(),
            vec![
                Param {
                    name: "a".to_string(),
                    ty: Type::String,
                },
                Param {
                    name: "b".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Bool,
        ),
    ]
}
