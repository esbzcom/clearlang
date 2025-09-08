use chumsky::prelude::*;
use crate::ErrTy;
use crate::tokens::kw;
use lumi_ast::{Effect, Type};

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
            kw("Bool").to(Type::Bool),
            kw("String").to(Type::String),
        ));
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
        choice((option, result, base))
            .padded()
            .labelled("type")
    })
}
