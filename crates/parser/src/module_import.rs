use crate::path::path_segments_p;
use crate::tokens::{ident_p, kw};
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{ImportDecl, ImportItem, ImportKind, ModuleDecl, Span};

fn to_span(sp: chumsky::span::SimpleSpan<usize>) -> Span {
    Span {
        start: sp.start,
        end: sp.end,
    }
}

pub(crate) fn module_decl_p<'a>() -> impl Parser<'a, &'a str, ModuleDecl, ErrTy<'a>> {
    kw("module")
        .ignore_then(path_segments_p().map_with(|path, e| (path, to_span(e.span()))))
        .then_ignore(just(';').padded().or_not())
        .map(|(path, span)| ModuleDecl { path, span })
}

pub(crate) fn import_decl_p<'a>() -> impl Parser<'a, &'a str, ImportDecl, ErrTy<'a>> {
    let item = ident_p().map_with(|name, e| ImportItem {
        name,
        span: to_span(e.span()),
    });

    let items = item
        .separated_by(just(',').padded().labelled("comma"))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(
            just('{').padded().labelled("'{'"),
            just('}').padded().labelled("'}'"),
        );

    let item_list = just("::")
        .padded()
        .ignore_then(items)
        .map(|items| ImportKind::Items { items });

    let alias = kw("as")
        .ignore_then(ident_p())
        .map(|alias| ImportKind::Module {
            alias: Some(alias),
        });

    kw("import")
        .ignore_then(path_segments_p().map_with(|path, e| (path, to_span(e.span()))))
        .then(item_list.or(alias).or_not())
        .then_ignore(just(';').padded().or_not())
        .map_with(|((path, path_span), kind), e| ImportDecl {
            path,
            path_span,
            kind: kind.unwrap_or(ImportKind::Module { alias: None }),
            span: to_span(e.span()),
        })
}
