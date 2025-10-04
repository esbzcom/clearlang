use clg_ast::{Effect, Param, ParamKind, Type};

pub(crate) fn builtin_sigs() -> Vec<(String, Vec<Param>, Type, Effect)> {
    vec![
        (
            "std::str::len".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "s".to_string(),
                ty: Type::String,
            }],
            Type::Int,
            Effect::Pure,
        ),
        (
            "std::str::concat".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::String,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::String,
                },
            ],
            Type::String,
            Effect::Pure,
        ),
        (
            "std::str::eq".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::String,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
    ]
}
