use chumsky::prelude::*;
use crate::ErrTy;

// Low-level identifier pieces and keywords
pub(crate) fn ident_start<'a>() -> impl Parser<'a, &'a str, char, ErrTy<'a>> {
    any().filter(|c: &char| c.is_alphabetic() || *c == '_')
}

pub(crate) fn ident_continue<'a>() -> impl Parser<'a, &'a str, char, ErrTy<'a>> {
    any().filter(|c: &char| c.is_alphanumeric() || *c == '_')
}

pub(crate) fn kw<'a>(s: &'static str) -> impl Parser<'a, &'a str, &'static str, ErrTy<'a>> {
    just(s)
        .then(ident_continue().or_not())
        .try_map(move |(kwd, next), span| {
            if next.is_some() {
                Err(Rich::custom(span, format!("keyword `{kwd}` must be a word")))
            } else {
                Ok(s)
            }
        })
        .padded()
        .boxed()
}

// Reserved constructor names for ADTs used in expressions and patterns
pub(crate) fn ctor_name_p<'a>() -> impl Parser<'a, &'a str, String, ErrTy<'a>> {
    choice((
        just("Some").to("Some"),
        just("None").to("None"),
        just("Ok").to("Ok"),
        just("Err").to("Err"),
    ))
    .map(|s: &str| s.to_string())
    .padded()
}

pub(crate) fn ident_p<'a>() -> impl Parser<'a, &'a str, String, ErrTy<'a>> {
    ident_start()
        .then(ident_continue().repeated().collect::<String>())
        .map(|(h, rest)| {
            let mut s = String::with_capacity(1 + rest.len());
            s.push(h);
            s.push_str(&rest);
            s
        })
        .try_map(|s: String, span| match s.as_str() {
            "function" | "pure" | "mut" | "io" | "Int" | "Bool" | "String" | "Option" | "Result" | "match" | "Some" | "None" | "Ok" | "Err" | "true" | "false" => Err(Rich::custom(
                span,
                format!("`{s}` is a reserved keyword"),
            )),
            _ => Ok(s),
        })
        .padded()
        .labelled("identifier")
}

pub(crate) fn func_name_p<'a>() -> impl Parser<'a, &'a str, String, ErrTy<'a>> {
    // Use an explicit charset to produce a clearer expectation than a generic filter.
    let start = one_of("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_")
        .labelled("function name");
    start
        .then(ident_continue().repeated().collect::<String>())
        .map(|(h, rest)| {
            let mut s = String::with_capacity(1 + rest.len());
            s.push(h);
            s.push_str(&rest);
            s
        })
        .try_map(|s: String, span| match s.as_str() {
            "function" | "pure" | "mut" | "io" | "Int" | "Bool" | "true" | "false" | "match" => Err(Rich::custom(
                span,
                format!("`{s}` is a reserved keyword"),
            )),
            _ => Ok(s),
        })
        .padded()
}
