use clg_ast::{Effect, Param, ParamKind, Type};

pub(crate) fn builtin_sigs() -> Vec<(String, Vec<Param>, Type, Effect)> {
    vec![
        (
            "std::bytes::len".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "b".to_string(),
                ty: Type::Bytes,
            }],
            Type::Int,
            Effect::Pure,
        ),
        (
            "std::bytes::is_empty".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "b".to_string(),
                ty: Type::Bytes,
            }],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::bytes::concat".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::Bytes,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::Bytes,
                },
            ],
            Type::Bytes,
            Effect::Pure,
        ),
        (
            "std::bytes::eq".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::Bytes,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::Bytes,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::bytes::equals".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::Bytes,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::Bytes,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::bytes::eq_ct".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::Bytes,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::Bytes,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::bytes::equals_ct".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::Bytes,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::Bytes,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::bytes::from_string".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "s".to_string(),
                ty: Type::String,
            }],
            Type::Bytes,
            Effect::Pure,
        ),
        (
            "std::bytes::to_string".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "b".to_string(),
                ty: Type::Bytes,
            }],
            Type::String,
            Effect::Pure,
        ),
        (
            "std::wasi::print".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "b".to_string(),
                ty: Type::Bytes,
            }],
            Type::Int,
            Effect::Io,
        ),
        (
            "std::env::time".to_string(),
            Vec::new(),
            Type::Int,
            Effect::Io,
        ),
        (
            "std::env::random".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "len".to_string(),
                ty: Type::Int,
            }],
            Type::Bytes,
            Effect::Io,
        ),
        (
            "std::crypto::hash".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "alg".to_string(),
                    ty: Type::String,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "data".to_string(),
                    ty: Type::Bytes,
                },
            ],
            Type::Bytes,
            Effect::Io,
        ),
        (
            "std::crypto::hmac".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "alg".to_string(),
                    ty: Type::String,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "key".to_string(),
                    ty: Type::Bytes,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "data".to_string(),
                    ty: Type::Bytes,
                },
            ],
            Type::Bytes,
            Effect::Io,
        ),
        (
            "std::crypto::sha256".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "data".to_string(),
                ty: Type::Bytes,
            }],
            Type::Bytes,
            Effect::Io,
        ),
        (
            "std::crypto::hmac_sha256".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "key".to_string(),
                    ty: Type::Bytes,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "data".to_string(),
                    ty: Type::Bytes,
                },
            ],
            Type::Bytes,
            Effect::Io,
        ),
        (
            "std::crypto::verify".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "alg".to_string(),
                    ty: Type::String,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "msg".to_string(),
                    ty: Type::Bytes,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "sig".to_string(),
                    ty: Type::Bytes,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "pk".to_string(),
                    ty: Type::Bytes,
                },
            ],
            Type::Bool,
            Effect::Io,
        ),
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
            "std::str::is_empty".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "s".to_string(),
                ty: Type::String,
            }],
            Type::Bool,
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
            "std::str::starts_with".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "value".to_string(),
                    ty: Type::String,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "prefix".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::str::ends_with".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "value".to_string(),
                    ty: Type::String,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "suffix".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::str::contains".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "value".to_string(),
                    ty: Type::String,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "needle".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::str_pattern::matches".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "pattern".to_string(),
                    ty: Type::String,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "input".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Bool,
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
        (
            "std::str::equals".to_string(),
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
        (
            "std::str::to_bytes".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "value".to_string(),
                ty: Type::String,
            }],
            Type::Bytes,
            Effect::Pure,
        ),
        (
            "std::list::new".to_string(),
            vec![],
            Type::List(Box::new(Type::Int)),
            Effect::Pure,
        ),
        (
            "std::list::len".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "list".to_string(),
                ty: Type::List(Box::new(Type::Int)),
            }],
            Type::Int,
            Effect::Pure,
        ),
        (
            "std::list::is_empty".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "list".to_string(),
                ty: Type::List(Box::new(Type::Int)),
            }],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::list::get".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "list".to_string(),
                    ty: Type::List(Box::new(Type::Int)),
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "index".to_string(),
                    ty: Type::Int,
                },
            ],
            Type::Option(Box::new(Type::Int)),
            Effect::Pure,
        ),
        (
            "std::list::push".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "list".to_string(),
                    ty: Type::List(Box::new(Type::Int)),
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "value".to_string(),
                    ty: Type::Int,
                },
            ],
            Type::List(Box::new(Type::Int)),
            Effect::Pure,
        ),
        (
            "std::list::pop".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "list".to_string(),
                ty: Type::List(Box::new(Type::Int)),
            }],
            Type::Option(Box::new(Type::Int)),
            Effect::Pure,
        ),
        (
            "std::list::insert".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "list".to_string(),
                    ty: Type::List(Box::new(Type::Int)),
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "value".to_string(),
                    ty: Type::Int,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "index".to_string(),
                    ty: Type::Int,
                },
            ],
            Type::List(Box::new(Type::Int)),
            Effect::Pure,
        ),
        (
            "std::list::remove".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "list".to_string(),
                    ty: Type::List(Box::new(Type::Int)),
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "index".to_string(),
                    ty: Type::Int,
                },
            ],
            Type::List(Box::new(Type::Int)),
            Effect::Pure,
        ),
        (
            "std::list::insert_checked".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "list".to_string(),
                    ty: Type::List(Box::new(Type::Int)),
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "value".to_string(),
                    ty: Type::Int,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "index".to_string(),
                    ty: Type::Int,
                },
            ],
            Type::Result(
                Box::new(Type::List(Box::new(Type::Int))),
                Box::new(Type::Named {
                    name: "std::collection_error::CollectionError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Pure,
        ),
        (
            "std::list::remove_checked".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "list".to_string(),
                    ty: Type::List(Box::new(Type::Int)),
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "index".to_string(),
                    ty: Type::Int,
                },
            ],
            Type::Result(
                Box::new(Type::List(Box::new(Type::Int))),
                Box::new(Type::Named {
                    name: "std::collection_error::CollectionError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Pure,
        ),
        (
            "std::list::remove_take".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "list".to_string(),
                    ty: Type::List(Box::new(Type::Int)),
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "index".to_string(),
                    ty: Type::Int,
                },
            ],
            Type::Tuple(vec![
                Type::List(Box::new(Type::Int)),
                Type::Option(Box::new(Type::Int)),
            ]),
            Effect::Pure,
        ),
        (
            "std::set::len".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "set".to_string(),
                ty: Type::Set(Box::new(Type::Int)),
            }],
            Type::Int,
            Effect::Pure,
        ),
        (
            "std::set::contains".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "set".to_string(),
                    ty: Type::Set(Box::new(Type::Int)),
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "value".to_string(),
                    ty: Type::Int,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::set::subset".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "lhs".to_string(),
                    ty: Type::Set(Box::new(Type::Int)),
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "rhs".to_string(),
                    ty: Type::Set(Box::new(Type::Int)),
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::set::union".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "lhs".to_string(),
                    ty: Type::Set(Box::new(Type::Int)),
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "rhs".to_string(),
                    ty: Type::Set(Box::new(Type::Int)),
                },
            ],
            Type::Set(Box::new(Type::Int)),
            Effect::Pure,
        ),
        (
            "std::set::intersect".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "lhs".to_string(),
                    ty: Type::Set(Box::new(Type::Int)),
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "rhs".to_string(),
                    ty: Type::Set(Box::new(Type::Int)),
                },
            ],
            Type::Set(Box::new(Type::Int)),
            Effect::Pure,
        ),
        (
            "std::set::diff".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "lhs".to_string(),
                    ty: Type::Set(Box::new(Type::Int)),
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "rhs".to_string(),
                    ty: Type::Set(Box::new(Type::Int)),
                },
            ],
            Type::Set(Box::new(Type::Int)),
            Effect::Pure,
        ),
        (
            "std::unit::assert_true".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "cond".to_string(),
                    ty: Type::Bool,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "msg".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::unit::assert_false".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "cond".to_string(),
                    ty: Type::Bool,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "msg".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::unit::assert_eq_int".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "actual".to_string(),
                    ty: Type::Int,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "expected".to_string(),
                    ty: Type::Int,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "msg".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::unit::assert_eq_u64".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "actual".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "expected".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "msg".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::unit::assert_eq_bool".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "actual".to_string(),
                    ty: Type::Bool,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "expected".to_string(),
                    ty: Type::Bool,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "msg".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::unit::fail".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "msg".to_string(),
                ty: Type::String,
            }],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::u64::add_wrap".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::add_wrapping".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::sub_wrap".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::sub_wrapping".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::mul_wrap".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::mul_wrapping".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::add_sat".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::add_saturating".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::sub_sat".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::sub_saturating".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::mul_sat".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::mul_saturating".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::rotl".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "value".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "shift".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::rotr".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "value".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "shift".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::to_bytes_le".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "value".to_string(),
                ty: Type::U64,
            }],
            Type::Bytes,
            Effect::Pure,
        ),
        (
            "std::u64::to_bytes_be".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "value".to_string(),
                ty: Type::U64,
            }],
            Type::Bytes,
            Effect::Pure,
        ),
        (
            "std::u64::from_bytes_le".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "bytes".to_string(),
                ty: Type::Bytes,
            }],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u64::from_bytes_be".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "bytes".to_string(),
                ty: Type::Bytes,
            }],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u128::from_limbs".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "lo".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "hi".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U128,
            Effect::Pure,
        ),
        (
            "std::u128::lo".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "value".to_string(),
                ty: Type::U128,
            }],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u128::hi".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "value".to_string(),
                ty: Type::U128,
            }],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u256::from_limbs".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "limb0".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "limb1".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "limb2".to_string(),
                    ty: Type::U64,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "limb3".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::U256,
            Effect::Pure,
        ),
        (
            "std::u256::limb0".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "value".to_string(),
                ty: Type::U256,
            }],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u256::limb1".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "value".to_string(),
                ty: Type::U256,
            }],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u256::limb2".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "value".to_string(),
                ty: Type::U256,
            }],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::u256::limb3".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "value".to_string(),
                ty: Type::U256,
            }],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::eth::from_bytes".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "bytes".to_string(),
                ty: Type::Bytes,
            }],
            Type::Named {
                name: "std::eth::Address".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::eth::from_array".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "bytes".to_string(),
                ty: Type::Array(Box::new(Type::U8), None),
            }],
            Type::Named {
                name: "std::eth::Address".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::solana::from_bytes".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "bytes".to_string(),
                ty: Type::Bytes,
            }],
            Type::Named {
                name: "std::solana::Pubkey".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::solana::from_array".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "bytes".to_string(),
                ty: Type::Array(Box::new(Type::U8), None),
            }],
            Type::Named {
                name: "std::solana::Pubkey".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::cosmos::from_bytes".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "bytes".to_string(),
                ty: Type::Bytes,
            }],
            Type::Named {
                name: "std::cosmos::Addr".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::cosmos::from_array".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "bytes".to_string(),
                ty: Type::Array(Box::new(Type::U8), None),
            }],
            Type::Named {
                name: "std::cosmos::Addr".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
    ]
}
