use std::collections::{HashMap, HashSet, VecDeque};

use clg_ast::{Func, ImplDecl, Span, Type};

use crate::check::ImplInfo;
use crate::errors::TyperError;

use super::mangle::{mangle_fn_name, mangle_impl_method_name, mangle_type};
use super::*;

#[test]
fn find_impl_uses_call_span_for_missing_impl() {
    let trait_env = TraitEnv {
        traits: HashMap::new(),
        impls: Vec::new(),
    };
    let aliases: AliasMap = HashMap::new();
    let type_defs = TypeDefs {
        resources: HashSet::new(),
        structs: HashMap::new(),
        enums: HashMap::new(),
    };
    let base_fns: HashMap<&str, FnSig> = HashMap::new();
    let base_funcs: HashMap<&str, &Func> = HashMap::new();
    let mono = Monomorphizer {
        base_fns: &base_fns,
        base_funcs,
        trait_env: &trait_env,
        aliases: &aliases,
        type_defs: &type_defs,
        mono_funcs: Vec::new(),
        mono_map: HashMap::new(),
        queue: VecDeque::new(),
    };
    let span = Span { start: 12, end: 34 };
    let err = match mono.find_impl("Eq", &Type::Int, span) {
        Ok(_) => panic!("expected missing impl"),
        Err(err) => err,
    };
    let te = err.downcast_ref::<TyperError>().expect("typer error");
    assert_eq!(te.code, "T237");
    assert_eq!(te.start, span.start);
    assert_eq!(te.end, span.end);
}

#[test]
fn find_impl_uses_call_span_for_ambiguous_impl() {
    let impl1 = ImplDecl {
        trait_name: "Eq".to_string(),
        trait_name_span: Span { start: 0, end: 0 },
        type_params: Vec::new(),
        for_type: Type::Int,
        where_bounds: Vec::new(),
        methods: Vec::new(),
        span: Span { start: 1, end: 2 },
    };
    let impl2 = ImplDecl {
        trait_name: "Eq".to_string(),
        trait_name_span: Span { start: 0, end: 0 },
        type_params: Vec::new(),
        for_type: Type::Int,
        where_bounds: Vec::new(),
        methods: Vec::new(),
        span: Span { start: 3, end: 4 },
    };
    let trait_env = TraitEnv {
        traits: HashMap::new(),
        impls: vec![
            ImplInfo {
                decl: &impl1,
                methods: HashMap::new(),
            },
            ImplInfo {
                decl: &impl2,
                methods: HashMap::new(),
            },
        ],
    };
    let aliases: AliasMap = HashMap::new();
    let type_defs = TypeDefs {
        resources: HashSet::new(),
        structs: HashMap::new(),
        enums: HashMap::new(),
    };
    let base_fns: HashMap<&str, FnSig> = HashMap::new();
    let base_funcs: HashMap<&str, &Func> = HashMap::new();
    let mono = Monomorphizer {
        base_fns: &base_fns,
        base_funcs,
        trait_env: &trait_env,
        aliases: &aliases,
        type_defs: &type_defs,
        mono_funcs: Vec::new(),
        mono_map: HashMap::new(),
        queue: VecDeque::new(),
    };
    let span = Span { start: 55, end: 89 };
    let err = match mono.find_impl("Eq", &Type::Int, span) {
        Ok(_) => panic!("expected ambiguous impl"),
        Err(err) => err,
    };
    let te = err.downcast_ref::<TyperError>().expect("typer error");
    assert_eq!(te.code, "T248");
    assert_eq!(te.start, span.start);
    assert_eq!(te.end, span.end);
    assert!(te.message.contains("candidates"));
    assert!(te.message.contains("implementation for `Int`"));
    assert!(te.message.contains("1..2"));
    assert!(te.message.contains("3..4"));
}

#[test]
fn mangled_names_use_identifier_safe_chars() {
    let aliases: AliasMap = HashMap::new();
    let types = vec![
        Type::Int,
        Type::Bool,
        Type::Named {
            name: "Box".to_string(),
            args: vec![Type::U8],
        },
        Type::Option(Box::new(Type::Named {
            name: "Pair".to_string(),
            args: vec![Type::U64, Type::String],
        })),
        Type::Result(Box::new(Type::Int), Box::new(Type::U256)),
        Type::List(Box::new(Type::Bytes)),
        Type::Set(Box::new(Type::U128)),
        Type::Map(
            Box::new(Type::U8),
            Box::new(Type::Named {
                name: "Thing".to_string(),
                args: vec![Type::Bool],
            }),
        ),
        Type::Array(Box::new(Type::U64), None),
        Type::Tuple(vec![Type::Int, Type::Bool, Type::U8]),
    ];
    for ty in types {
        let name = mangle_type(&ty, &aliases).expect("mangle type");
        assert!(
            name.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$'),
            "mangled name contained disallowed char: {name}"
        );
    }
    let fn_name = mangle_fn_name(
        "do_work",
        &[
            Type::Named {
                name: "Box".to_string(),
                args: vec![Type::U8],
            },
            Type::Tuple(vec![Type::Int, Type::U64]),
        ],
        &aliases,
    )
    .expect("mangle fn");
    assert!(
        fn_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$'),
        "mangled fn name contained disallowed char: {fn_name}"
    );
    let impl_name = mangle_impl_method_name("Eq", &Type::Int, "eq", &aliases).expect("mangle impl");
    assert!(
        impl_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$'),
        "mangled impl name contained disallowed char: {impl_name}"
    );
}
