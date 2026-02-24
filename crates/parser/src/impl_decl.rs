use crate::func::func_p;
use crate::generics::{type_params_with_bounds_p, where_bounds_p};
use crate::path::path_name_p;
use crate::tokens::kw;
use crate::types::ty_p;
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{ImplDecl, RefinedAlias, Span};

fn to_span(sp: chumsky::span::SimpleSpan<usize>) -> Span {
    Span {
        start: sp.start,
        end: sp.end,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedImplDecl {
    pub decl: ImplDecl,
    pub inline_aliases: Vec<RefinedAlias>,
}

pub(crate) fn impl_p<'a>() -> impl Parser<'a, &'a str, ParsedImplDecl, ErrTy<'a>> {
    let methods = func_p().repeated().collect::<Vec<_>>();
    kw("implementation")
        .ignore_then(type_params_with_bounds_p().or_not())
        .then(path_name_p().map_with(|name, e| (name, to_span(e.span()))))
        .then_ignore(kw("for"))
        .then(ty_p().padded())
        .then(where_bounds_p().or_not())
        .then(methods.delimited_by(just('{').padded(), just('}').padded()))
        .map_with(
            |(
                (((type_params, (trait_name, trait_name_span)), for_type), where_bounds),
                methods,
            ),
             e| {
                let (type_params, mut bounds) = type_params.unwrap_or_default();
                if let Some(mut extra_bounds) = where_bounds {
                    bounds.append(&mut extra_bounds);
                }
                let mut inline_aliases = Vec::new();
                let mut method_funcs = Vec::with_capacity(methods.len());
                for method in methods {
                    inline_aliases.extend(method.inline_aliases);
                    method_funcs.push(method.func);
                }
                ParsedImplDecl {
                    decl: ImplDecl {
                        trait_name,
                        trait_name_span,
                        type_params,
                        for_type,
                        where_bounds: bounds,
                        methods: method_funcs,
                        span: to_span(e.span()),
                    },
                    inline_aliases,
                }
            },
        )
}
