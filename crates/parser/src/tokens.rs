use crate::ErrTy;
use chumsky::prelude::*;

// Low-level identifier pieces and keywords
pub(crate) fn ident_start<'a>() -> impl Parser<'a, &'a str, char, ErrTy<'a>> {
    any().filter(|c: &char| c.is_alphabetic() || *c == '_')
}

pub(crate) fn ident_continue<'a>() -> impl Parser<'a, &'a str, char, ErrTy<'a>> {
    any().filter(|c: &char| c.is_alphanumeric() || *c == '_')
}

fn parse_decimal_i64(raw: &str) -> Result<i64, String> {
    parse_radix_i64(raw, 10, "decimal", |c| c.is_ascii_digit())
}

fn parse_prefixed_i64(raw: &str, prefix: &str, radix: u32, base_name: &str) -> Result<i64, String> {
    let digits = raw
        .strip_prefix(prefix)
        .expect("prefix validation should happen before parse_prefixed_i64");
    if digits.is_empty() {
        return Err(format!(
            "invalid integer literal: missing digits after `{prefix}`"
        ));
    }
    if radix == 16 {
        parse_radix_i64(digits, radix, base_name, |c| c.is_ascii_hexdigit())
    } else if radix == 2 {
        parse_radix_i64(digits, radix, base_name, |c| matches!(c, '0' | '1'))
    } else {
        Err(format!("internal error: unsupported radix {radix}"))
    }
}

pub(crate) fn parse_int_literal_raw(raw: &str) -> Result<i64, String> {
    if raw.starts_with("0x") || raw.starts_with("0X") {
        parse_prefixed_i64(raw, &raw[..2], 16, "hex")
    } else if raw.starts_with("0b") || raw.starts_with("0B") {
        parse_prefixed_i64(raw, &raw[..2], 2, "binary")
    } else {
        parse_decimal_i64(raw)
    }
}

fn parse_radix_i64(
    raw: &str,
    radix: u32,
    base_name: &str,
    is_valid_digit: impl Fn(char) -> bool,
) -> Result<i64, String> {
    if raw.starts_with('_') || raw.ends_with('_') || raw.contains("__") {
        return Err("invalid integer literal: misplaced `_` separator".to_string());
    }
    if let Some(c) = raw.chars().find(|c| *c != '_' && !is_valid_digit(*c)) {
        return Err(format!(
            "invalid integer literal: invalid {base_name} digit `{c}`"
        ));
    }
    let normalized = raw.replace('_', "");
    i64::from_str_radix(&normalized, radix)
        .map_err(|_| "integer literal out of range for `Int`".to_string())
}

pub(crate) fn int_literal_value_p<'a>() -> impl Parser<'a, &'a str, i64, ErrTy<'a>> {
    one_of("0123456789")
        .then(
            any()
                .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_')
                .repeated()
                .collect::<String>(),
        )
        .map(|(head, tail)| {
            let mut s = String::with_capacity(1 + tail.len());
            s.push(head);
            s.push_str(&tail);
            s
        })
        .try_map(|raw, span| match parse_int_literal_raw(&raw) {
            Ok(value) => Ok(value),
            Err(msg) => Err(Rich::custom(span, msg)),
        })
}

pub(crate) fn kw<'a>(s: &'static str) -> impl Parser<'a, &'a str, &'static str, ErrTy<'a>> {
    just(s)
        .then(ident_continue().or_not())
        .try_map(move |(kwd, next), span| {
            if next.is_some() {
                Err(Rich::custom(
                    span,
                    format!("keyword `{kwd}` must be a word"),
                ))
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
            "theorem" => Err(Rich::custom(
                span,
                "`theorem` keyword is reserved in milestone_3; theorem-grade is a release certification status, not syntax"
                    .to_string(),
            )),
            "function" | "pure" | "mut" | "io" | "return" | "let" | "while" | "invariant"
            | "variant" | "Int" | "U8" | "U64" | "U128" | "U256" | "Bool" | "String" | "Bytes"
            | "Option" | "Result" | "match" | "Some" | "None" | "Ok" | "Err" | "true" | "false"
            | "require" | "ensure" | "resource" | "struct" | "enum" | "interface"
            | "implementation" | "for" | "where" | "drop" | "consume" | "import" | "export"
            | "module" | "as" | "contract" | "state" | "version" | "init" | "old"
            | "migrate" => Err(Rich::custom(span, format!("`{s}` is a reserved keyword"))),
            _ => Ok(s),
        })
        .padded()
        .labelled("identifier")
}

pub(crate) fn func_name_p<'a>() -> impl Parser<'a, &'a str, String, ErrTy<'a>> {
    // Use an explicit charset to produce a clearer expectation than a generic filter.
    let start =
        one_of("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_").labelled("function name");
    start
        .then(ident_continue().repeated().collect::<String>())
        .map(|(h, rest)| {
            let mut s = String::with_capacity(1 + rest.len());
            s.push(h);
            s.push_str(&rest);
            s
        })
        .try_map(|s: String, span| match s.as_str() {
            "theorem" => Err(Rich::custom(
                span,
                "`theorem` keyword is reserved in milestone_3; theorem-grade is a release certification status, not syntax"
                    .to_string(),
            )),
            "function" | "pure" | "mut" | "io" | "return" | "let" | "while" | "invariant"
            | "variant" | "Int" | "U8" | "U64" | "U128" | "U256" | "Bool" | "true" | "false"
            | "match" | "require" | "ensure" | "resource" | "struct" | "enum" | "interface"
            | "implementation" | "for" | "where" | "drop" | "consume" | "import" | "export"
            | "module" | "as" | "contract" | "state" | "version" | "init" | "old"
            | "migrate" => Err(Rich::custom(span, format!("`{s}` is a reserved keyword"))),
            _ => Ok(s),
        })
        .padded()
}
