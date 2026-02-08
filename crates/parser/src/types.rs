use crate::path::path_name_p;
use crate::tokens::kw;
use crate::ErrTy;
use chumsky::prelude::*;
use chumsky::text;
use clg_ast::{Effect, Type};

pub(crate) fn effect_p<'a>() -> impl Parser<'a, &'a str, Effect, ErrTy<'a>> {
    choice((
        kw("pure").to(Effect::Pure),
        kw("mut").to(Effect::Mut),
        kw("io").to(Effect::Io),
    ))
    .padded()
}

pub(crate) fn ty_p<'a>() -> impl Parser<'a, &'a str, Type, ErrTy<'a>> {
    recursive(|ty| {
        let base = choice((
            kw("Int").to(Type::Int),
            kw("U8").to(Type::U8),
            kw("U64").to(Type::U64),
            kw("U128").to(Type::U128),
            kw("U256").to(Type::U256),
            kw("Bool").to(Type::Bool),
            kw("String").to(Type::String),
            kw("Bytes").to(Type::Bytes),
        ));
        let list_t = kw("List")
            .ignore_then(just('<').padded())
            .ignore_then(ty.clone())
            .then_ignore(just('>').padded())
            .map(|t| Type::List(Box::new(t)));
        let set_t = kw("Set")
            .ignore_then(just('<').padded())
            .ignore_then(ty.clone())
            .then_ignore(just('>').padded())
            .map(|t| Type::Set(Box::new(t)));
        let map_t = kw("Map")
            .ignore_then(just('<').padded())
            .ignore_then(ty.clone())
            .then_ignore(just(',').padded())
            .then(ty.clone())
            .then_ignore(just('>').padded())
            .map(|(k, v)| Type::Map(Box::new(k), Box::new(v)));
        let array_dyn = kw("Array")
            .ignore_then(just('<').padded())
            .ignore_then(ty.clone())
            .then_ignore(just('>').padded())
            .map(|t| Type::Array(Box::new(t), None));
        let slice_t = kw("Slice")
            .ignore_then(just('<').padded())
            .ignore_then(ty.clone())
            .then_ignore(just('>').padded())
            .map(|t| Type::Slice(Box::new(t)));
        let option = kw("Option")
            .ignore_then(just('<').padded())
            .ignore_then(ty.clone())
            .then_ignore(just('>').padded())
            .map(|t| Type::Option(Box::new(t)));
        let result = kw("Result")
            .ignore_then(just('<').padded())
            .ignore_then(ty.clone())
            .then_ignore(just(',').padded())
            .then(ty.clone())
            .then_ignore(just('>').padded())
            .map(|(ok, err)| Type::Result(Box::new(ok), Box::new(err)));
        let named = path_name_p().map(|name| Type::Named {
            name,
            args: Vec::new(),
        });
        let named_args = path_name_p()
            .then(
                ty.clone()
                    .separated_by(just(',').padded())
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just('<').padded(), just('>').padded()),
            )
            .map(|(name, args)| Type::Named { name, args });
        let tuple_t = ty
            .clone()
            .separated_by(just(',').padded())
            .at_least(2)
            .collect::<Vec<_>>()
            .map(Type::Tuple)
            .delimited_by(just('(').padded(), just(')').padded());
        let array_size = text::int(10)
            .from_str::<i64>()
            .unwrapped()
            .try_map(|n, span| {
                if n < 0 {
                    Err(Rich::custom(
                        span,
                        "array size must not be negative".to_string(),
                    ))
                } else if n > u32::MAX as i64 {
                    Err(Rich::custom(
                        span,
                        "array size exceeds maximum u32 value".to_string(),
                    ))
                } else {
                    Ok(n as u32)
                }
            });
        let array_t = just('[')
            .padded()
            .ignore_then(ty.clone())
            .then_ignore(just(';').padded())
            .then(array_size.padded())
            .then_ignore(just(']').padded())
            .map(|(inner, len)| Type::Array(Box::new(inner), Some(len)));
        choice((
            option, result, list_t, set_t, map_t, array_dyn, slice_t, array_t, tuple_t, base,
            named_args, named,
        ))
        .boxed()
    })
}
