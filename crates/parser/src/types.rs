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
    choice((
        kw("Int").to(Type::Int),
        kw("Bool").to(Type::Bool),
        kw("Str").to(Type::Str),
    ))
    .padded()
    .labelled("type")
}

