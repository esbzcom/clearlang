use std::collections::BTreeSet;
use std::sync::OnceLock;

use clg_ast::{Effect, Param, ParamKind, Type};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinRoute {
    Intrinsic,
    PackageImport,
}

pub fn all_builtin_sigs() -> Vec<(String, Vec<Param>, Type, Effect)> {
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
            "std::env::chain_id".to_string(),
            Vec::new(),
            Type::String,
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
            "std::host::storage::contains".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "key".to_string(),
                ty: Type::Bytes,
            }],
            Type::Result(
                Box::new(Type::Bool),
                Box::new(Type::Named {
                    name: "std::host::HostError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Io,
        ),
        (
            "std::host::storage::get".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "key".to_string(),
                ty: Type::Bytes,
            }],
            Type::Result(
                Box::new(Type::Option(Box::new(Type::Bytes))),
                Box::new(Type::Named {
                    name: "std::host::HostError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Io,
        ),
        (
            "std::host::storage::set".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "key".to_string(),
                    ty: Type::Bytes,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "value".to_string(),
                    ty: Type::Bytes,
                },
            ],
            Type::Result(
                Box::new(Type::Bool),
                Box::new(Type::Named {
                    name: "std::host::HostError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Io,
        ),
        (
            "std::host::storage::delete".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "key".to_string(),
                ty: Type::Bytes,
            }],
            Type::Result(
                Box::new(Type::Bool),
                Box::new(Type::Named {
                    name: "std::host::HostError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Io,
        ),
        (
            "std::host::log::info".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "code".to_string(),
                    ty: Type::Named {
                        name: "std::core::ErrorCode".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "message".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Result(
                Box::new(Type::Bool),
                Box::new(Type::Named {
                    name: "std::host::HostError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Io,
        ),
        (
            "std::host::log::warn".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "code".to_string(),
                    ty: Type::Named {
                        name: "std::core::ErrorCode".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "message".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Result(
                Box::new(Type::Bool),
                Box::new(Type::Named {
                    name: "std::host::HostError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Io,
        ),
        (
            "std::host::log::error".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "code".to_string(),
                    ty: Type::Named {
                        name: "std::core::ErrorCode".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "message".to_string(),
                    ty: Type::String,
                },
            ],
            Type::Result(
                Box::new(Type::Bool),
                Box::new(Type::Named {
                    name: "std::host::HostError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Io,
        ),
        (
            "std::host::env::chain_id".to_string(),
            Vec::new(),
            Type::Result(
                Box::new(Type::U64),
                Box::new(Type::Named {
                    name: "std::host::HostError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Io,
        ),
        (
            "std::host::env::caller".to_string(),
            Vec::new(),
            Type::Result(
                Box::new(Type::Bytes),
                Box::new(Type::Named {
                    name: "std::host::HostError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Io,
        ),
        (
            "std::host::env::block_height".to_string(),
            Vec::new(),
            Type::Result(
                Box::new(Type::U64),
                Box::new(Type::Named {
                    name: "std::host::HostError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Io,
        ),
        (
            "std::host::env::timestamp".to_string(),
            Vec::new(),
            Type::Result(
                Box::new(Type::U64),
                Box::new(Type::Named {
                    name: "std::host::HostError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Io,
        ),
        (
            "std::host::host_error::code".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "err".to_string(),
                ty: Type::Named {
                    name: "std::host::HostError".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Named {
                name: "std::core::ErrorCode".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::host::host_error::equals".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "lhs".to_string(),
                    ty: Type::Named {
                        name: "std::host::HostError".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "rhs".to_string(),
                    ty: Type::Named {
                        name: "std::host::HostError".to_string(),
                        args: Vec::new(),
                    },
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::contract::address::from_bytes".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "input".to_string(),
                ty: Type::Bytes,
            }],
            Type::Result(
                Box::new(Type::Named {
                    name: "std::contract::Address".to_string(),
                    args: Vec::new(),
                }),
                Box::new(Type::Named {
                    name: "std::contract::ContractError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Pure,
        ),
        (
            "std::contract::address::to_bytes".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "addr".to_string(),
                ty: Type::Named {
                    name: "std::contract::Address".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Bytes,
            Effect::Pure,
        ),
        (
            "std::contract::address::equals".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "lhs".to_string(),
                    ty: Type::Named {
                        name: "std::contract::Address".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "rhs".to_string(),
                    ty: Type::Named {
                        name: "std::contract::Address".to_string(),
                        args: Vec::new(),
                    },
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::contract::amount::from_u64".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "value".to_string(),
                ty: Type::U64,
            }],
            Type::Named {
                name: "std::contract::Amount".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::contract::amount::value".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "amount".to_string(),
                ty: Type::Named {
                    name: "std::contract::Amount".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::U64,
            Effect::Pure,
        ),
        (
            "std::contract::amount::add_checked".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "lhs".to_string(),
                    ty: Type::Named {
                        name: "std::contract::Amount".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "rhs".to_string(),
                    ty: Type::Named {
                        name: "std::contract::Amount".to_string(),
                        args: Vec::new(),
                    },
                },
            ],
            Type::Result(
                Box::new(Type::Named {
                    name: "std::contract::Amount".to_string(),
                    args: Vec::new(),
                }),
                Box::new(Type::Named {
                    name: "std::contract::ContractError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Pure,
        ),
        (
            "std::contract::amount::sub_checked".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "lhs".to_string(),
                    ty: Type::Named {
                        name: "std::contract::Amount".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "rhs".to_string(),
                    ty: Type::Named {
                        name: "std::contract::Amount".to_string(),
                        args: Vec::new(),
                    },
                },
            ],
            Type::Result(
                Box::new(Type::Named {
                    name: "std::contract::Amount".to_string(),
                    args: Vec::new(),
                }),
                Box::new(Type::Named {
                    name: "std::contract::ContractError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Pure,
        ),
        (
            "std::contract::amount::is_zero".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "amount".to_string(),
                ty: Type::Named {
                    name: "std::contract::Amount".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::contract::event::new".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "topic".to_string(),
                    ty: Type::Bytes,
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "payload".to_string(),
                    ty: Type::Bytes,
                },
            ],
            Type::Named {
                name: "std::contract::Event".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::contract::event::topic".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "event".to_string(),
                ty: Type::Named {
                    name: "std::contract::Event".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Bytes,
            Effect::Pure,
        ),
        (
            "std::contract::event::payload".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "event".to_string(),
                ty: Type::Named {
                    name: "std::contract::Event".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Bytes,
            Effect::Pure,
        ),
        (
            "std::contract::contract_error::code".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "err".to_string(),
                ty: Type::Named {
                    name: "std::contract::ContractError".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Named {
                name: "std::core::ErrorCode".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::contract::contract_error::equals".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "lhs".to_string(),
                    ty: Type::Named {
                        name: "std::contract::ContractError".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "rhs".to_string(),
                    ty: Type::Named {
                        name: "std::contract::ContractError".to_string(),
                        args: Vec::new(),
                    },
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::crypto::verify_result::is_valid".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "value".to_string(),
                ty: Type::Named {
                    name: "std::crypto::VerifyResult".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::crypto::verify_result::error_or_none".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "value".to_string(),
                ty: Type::Named {
                    name: "std::crypto::VerifyResult".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Option(Box::new(Type::Named {
                name: "std::crypto::CryptoError".to_string(),
                args: Vec::new(),
            })),
            Effect::Pure,
        ),
        (
            "std::crypto::verify_result::valid".to_string(),
            Vec::new(),
            Type::Named {
                name: "std::crypto::VerifyResult".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::crypto::verify_result::invalid".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "err".to_string(),
                ty: Type::Named {
                    name: "std::crypto::CryptoError".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Named {
                name: "std::crypto::VerifyResult".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::crypto::crypto_error::code".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "err".to_string(),
                ty: Type::Named {
                    name: "std::crypto::CryptoError".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Named {
                name: "std::core::ErrorCode".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::crypto::crypto_error::equals".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "lhs".to_string(),
                    ty: Type::Named {
                        name: "std::crypto::CryptoError".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "rhs".to_string(),
                    ty: Type::Named {
                        name: "std::crypto::CryptoError".to_string(),
                        args: Vec::new(),
                    },
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::encoder::new".to_string(),
            Vec::new(),
            Type::Named {
                name: "std::encoder::Encoder".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::encoder::write_u64".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "enc".to_string(),
                    ty: Type::Named {
                        name: "std::encoder::Encoder".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "value".to_string(),
                    ty: Type::U64,
                },
            ],
            Type::Result(
                Box::new(Type::Named {
                    name: "std::encoder::Encoder".to_string(),
                    args: Vec::new(),
                }),
                Box::new(Type::Named {
                    name: "std::encode_error::EncodeError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Pure,
        ),
        (
            "std::encoder::write_bool".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "enc".to_string(),
                    ty: Type::Named {
                        name: "std::encoder::Encoder".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "value".to_string(),
                    ty: Type::Bool,
                },
            ],
            Type::Result(
                Box::new(Type::Named {
                    name: "std::encoder::Encoder".to_string(),
                    args: Vec::new(),
                }),
                Box::new(Type::Named {
                    name: "std::encode_error::EncodeError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Pure,
        ),
        (
            "std::encoder::write_bytes".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "enc".to_string(),
                    ty: Type::Named {
                        name: "std::encoder::Encoder".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "value".to_string(),
                    ty: Type::Bytes,
                },
            ],
            Type::Result(
                Box::new(Type::Named {
                    name: "std::encoder::Encoder".to_string(),
                    args: Vec::new(),
                }),
                Box::new(Type::Named {
                    name: "std::encode_error::EncodeError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Pure,
        ),
        (
            "std::encoder::finish".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "enc".to_string(),
                ty: Type::Named {
                    name: "std::encoder::Encoder".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Result(
                Box::new(Type::Bytes),
                Box::new(Type::Named {
                    name: "std::encode_error::EncodeError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Pure,
        ),
        (
            "std::decoder::new".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "input".to_string(),
                ty: Type::Bytes,
            }],
            Type::Named {
                name: "std::decoder::Decoder".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::decoder::read_u64".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "dec".to_string(),
                ty: Type::Named {
                    name: "std::decoder::Decoder".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Result(
                Box::new(Type::Tuple(vec![
                    Type::Named {
                        name: "std::decoder::Decoder".to_string(),
                        args: Vec::new(),
                    },
                    Type::U64,
                ])),
                Box::new(Type::Named {
                    name: "std::decode_error::DecodeError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Pure,
        ),
        (
            "std::decoder::read_bool".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "dec".to_string(),
                ty: Type::Named {
                    name: "std::decoder::Decoder".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Result(
                Box::new(Type::Tuple(vec![
                    Type::Named {
                        name: "std::decoder::Decoder".to_string(),
                        args: Vec::new(),
                    },
                    Type::Bool,
                ])),
                Box::new(Type::Named {
                    name: "std::decode_error::DecodeError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Pure,
        ),
        (
            "std::decoder::read_bytes".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "dec".to_string(),
                ty: Type::Named {
                    name: "std::decoder::Decoder".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Result(
                Box::new(Type::Tuple(vec![
                    Type::Named {
                        name: "std::decoder::Decoder".to_string(),
                        args: Vec::new(),
                    },
                    Type::Bytes,
                ])),
                Box::new(Type::Named {
                    name: "std::decode_error::DecodeError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Pure,
        ),
        (
            "std::decoder::read_fixed".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "dec".to_string(),
                    ty: Type::Named {
                        name: "std::decoder::Decoder".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "len".to_string(),
                    ty: Type::Int,
                },
            ],
            Type::Result(
                Box::new(Type::Tuple(vec![
                    Type::Named {
                        name: "std::decoder::Decoder".to_string(),
                        args: Vec::new(),
                    },
                    Type::Bytes,
                ])),
                Box::new(Type::Named {
                    name: "std::decode_error::DecodeError".to_string(),
                    args: Vec::new(),
                }),
            ),
            Effect::Pure,
        ),
        (
            "std::decoder::position".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "dec".to_string(),
                ty: Type::Named {
                    name: "std::decoder::Decoder".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Int,
            Effect::Pure,
        ),
        (
            "std::decoder::remaining".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "dec".to_string(),
                ty: Type::Named {
                    name: "std::decoder::Decoder".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Int,
            Effect::Pure,
        ),
        (
            "std::decoder::is_eof".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "dec".to_string(),
                ty: Type::Named {
                    name: "std::decoder::Decoder".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::decode_error::code".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "err".to_string(),
                ty: Type::Named {
                    name: "std::decode_error::DecodeError".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Named {
                name: "std::core::ErrorCode".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::decode_error::offset".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "err".to_string(),
                ty: Type::Named {
                    name: "std::decode_error::DecodeError".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Option(Box::new(Type::Int)),
            Effect::Pure,
        ),
        (
            "std::decode_error::equals".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::Named {
                        name: "std::decode_error::DecodeError".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::Named {
                        name: "std::decode_error::DecodeError".to_string(),
                        args: Vec::new(),
                    },
                },
            ],
            Type::Bool,
            Effect::Pure,
        ),
        (
            "std::encode_error::code".to_string(),
            vec![Param {
                kind: ParamKind::Borrow,
                name: "err".to_string(),
                ty: Type::Named {
                    name: "std::encode_error::EncodeError".to_string(),
                    args: Vec::new(),
                },
            }],
            Type::Named {
                name: "std::core::ErrorCode".to_string(),
                args: Vec::new(),
            },
            Effect::Pure,
        ),
        (
            "std::encode_error::equals".to_string(),
            vec![
                Param {
                    kind: ParamKind::Borrow,
                    name: "a".to_string(),
                    ty: Type::Named {
                        name: "std::encode_error::EncodeError".to_string(),
                        args: Vec::new(),
                    },
                },
                Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::Named {
                        name: "std::encode_error::EncodeError".to_string(),
                        args: Vec::new(),
                    },
                },
            ],
            Type::Bool,
            Effect::Pure,
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

pub fn builtin_sigs() -> Vec<(String, Vec<Param>, Type, Effect)> {
    all_builtin_sigs()
        .into_iter()
        .filter(|(name, _, _, _)| verified_std_abi_value_symbols().contains(name))
        .collect()
}

pub fn non_abi_builtin_sigs() -> Vec<(String, Vec<Param>, Type, Effect)> {
    all_builtin_sigs()
        .into_iter()
        .filter(|(name, _, _, _)| !verified_std_abi_value_symbols().contains(name))
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StdTextCompatBuiltin {
    BytesEquals,
    BytesEqualsCt,
    BytesIsEmpty,
    StrEquals,
    StrIsEmpty,
    StrToBytes,
    StrPatternMatches,
}

pub fn std_text_compat_builtin(symbol: &str) -> Option<StdTextCompatBuiltin> {
    match symbol {
        "std::bytes::equals" => Some(StdTextCompatBuiltin::BytesEquals),
        "std::bytes::equals_ct" => Some(StdTextCompatBuiltin::BytesEqualsCt),
        "std::bytes::is_empty" => Some(StdTextCompatBuiltin::BytesIsEmpty),
        "std::str::equals" => Some(StdTextCompatBuiltin::StrEquals),
        "std::str::is_empty" => Some(StdTextCompatBuiltin::StrIsEmpty),
        "std::str::to_bytes" => Some(StdTextCompatBuiltin::StrToBytes),
        "std::str_pattern::matches" => Some(StdTextCompatBuiltin::StrPatternMatches),
        _ => None,
    }
}

pub fn builtin_compat_alias_target(symbol: &str) -> Option<&'static str> {
    match symbol {
        "std::bytes::equals" => Some("std::bytes::eq"),
        "std::bytes::equals_ct" => Some("std::bytes::eq_ct"),
        "std::str::equals" => Some("std::str::eq"),
        "std::u64::add_wrap" => Some("std::u64::add_wrapping"),
        "std::u64::sub_wrap" => Some("std::u64::sub_wrapping"),
        "std::u64::mul_wrap" => Some("std::u64::mul_wrapping"),
        "std::u64::add_sat" => Some("std::u64::add_saturating"),
        "std::u64::sub_sat" => Some("std::u64::sub_saturating"),
        "std::u64::mul_sat" => Some("std::u64::mul_saturating"),
        "std::crypto::sha256" => Some("std::crypto::hash"),
        "std::crypto::hmac_sha256" => Some("std::crypto::hmac"),
        _ => None,
    }
}

pub fn builtin_route(symbol: &str) -> BuiltinRoute {
    match symbol {
        "std::bytes::len"
        | "std::bytes::eq"
        | "std::bytes::eq_ct"
        | "std::bytes::concat"
        | "std::bytes::from_string"
        | "std::bytes::to_string"
        | "std::wasi::print"
        | "std::env::time"
        | "std::env::chain_id"
        | "std::env::random"
        | "std::crypto::hash"
        | "std::crypto::hmac"
        | "std::crypto::verify"
        | "std::host::__storage_contains_raw"
        | "std::host::__storage_get_raw"
        | "std::host::__storage_set_raw"
        | "std::host::__storage_delete_raw"
        | "std::host::__log_info_raw"
        | "std::host::__log_warn_raw"
        | "std::host::__log_error_raw"
        | "std::host::__env_chain_id_raw"
        | "std::host::__env_caller_raw"
        | "std::host::__env_block_height_raw"
        | "std::host::__env_timestamp_raw"
        | "std::str::len"
        | "std::str::eq"
        | "std::str::concat"
        | "std::str::starts_with"
        | "std::str::ends_with"
        | "std::str::contains"
        | "std::u64::rotl"
        | "std::u64::rotr"
        | "std::u64::to_bytes_le"
        | "std::u64::to_bytes_be"
        | "std::u64::from_bytes_le"
        | "std::u64::from_bytes_be" => BuiltinRoute::Intrinsic,
        _ => BuiltinRoute::PackageImport,
    }
}

pub fn verified_std_abi_value_symbols() -> &'static BTreeSet<String> {
    static VERIFIED_STD_ABI_VALUE_SYMBOLS: OnceLock<BTreeSet<String>> = OnceLock::new();
    VERIFIED_STD_ABI_VALUE_SYMBOLS.get_or_init(|| {
        let raw: VerifiedStdAbiManifest = serde_json::from_str(include_str!(
            "../../../docs/design/phase-27.1-verified-std-abi.manifest.v1.json"
        ))
        .expect("bundled verified std abi manifest must parse");
        assert_eq!(
            raw.schema_version, 1,
            "bundled verified std abi manifest schema must stay at v1"
        );
        let mut out = BTreeSet::new();
        for module in raw.modules {
            for export in module.exports {
                if export.kind == VerifiedStdAbiExportKind::Value {
                    out.insert(format!("{}::{}", module.path, export.name));
                }
            }
        }
        out
    })
}

#[derive(Deserialize)]
struct VerifiedStdAbiManifest {
    schema_version: u32,
    modules: Vec<VerifiedStdAbiModule>,
}

#[derive(Deserialize)]
struct VerifiedStdAbiModule {
    path: String,
    exports: Vec<VerifiedStdAbiExport>,
}

#[derive(Deserialize)]
struct VerifiedStdAbiExport {
    name: String,
    kind: VerifiedStdAbiExportKind,
}

#[derive(Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum VerifiedStdAbiExportKind {
    Type,
    Value,
}

#[cfg(test)]
mod tests {
    use super::{builtin_compat_alias_target, std_text_compat_builtin, StdTextCompatBuiltin};

    #[test]
    fn std_text_compat_builtins_stay_classified() {
        assert_eq!(
            std_text_compat_builtin("std::bytes::equals"),
            Some(StdTextCompatBuiltin::BytesEquals)
        );
        assert_eq!(
            std_text_compat_builtin("std::str_pattern::matches"),
            Some(StdTextCompatBuiltin::StrPatternMatches)
        );
        assert_eq!(std_text_compat_builtin("std::str::len"), None);
    }

    #[test]
    fn builtin_compat_alias_target_covers_text_and_legacy_aliases() {
        assert_eq!(
            builtin_compat_alias_target("std::bytes::equals_ct"),
            Some("std::bytes::eq_ct")
        );
        assert_eq!(
            builtin_compat_alias_target("std::u64::add_wrap"),
            Some("std::u64::add_wrapping")
        );
        assert_eq!(builtin_compat_alias_target("std::str::is_empty"), None);
    }
}
