use crate::alias::refined_alias_p;
use crate::enum_decl::enum_p;
use crate::func::{func_p, ParsedFunc};
use crate::impl_decl::{impl_p, ParsedImplDecl};
use crate::module_import::{import_decl_p, module_decl_p};
use crate::resource::resource_p;
use crate::struct_decl::struct_p;
use crate::tokens::kw;
use crate::trait_decl::trait_p;
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::{Program, Span};

mod heuristics;

use self::heuristics::{
    find_capture_list_lambda, find_comma_grouped_number, find_export_import, find_theorem_keyword,
    find_untyped_lambda, has_unclosed_paren, keyword_missing_brace, looks_like_missing_comma,
    strip_comments_preserve_layout,
};

#[derive(Debug)]
enum Item {
    Alias(clg_ast::RefinedAlias),
    Func(ParsedFunc),
    Resource(clg_ast::Resource),
    Struct(clg_ast::StructDecl),
    Enum(clg_ast::EnumDecl),
    Trait(clg_ast::TraitDecl),
    Impl(ParsedImplDecl),
}

#[derive(Debug)]
enum TopLevel {
    Import(clg_ast::ImportDecl),
    Item(Box<Item>),
}

fn to_span(sp: chumsky::span::SimpleSpan<usize>) -> Span {
    Span {
        start: sp.start,
        end: sp.end,
    }
}

fn apply_export<'a>(item: Item, export_span: Span) -> Result<Item, Rich<'a, char>> {
    match item {
        Item::Alias(mut alias) => {
            alias.is_exported = true;
            Ok(Item::Alias(alias))
        }
        Item::Func(mut func) => {
            func.func.is_exported = true;
            Ok(Item::Func(func))
        }
        Item::Resource(mut resource) => {
            resource.is_exported = true;
            Ok(Item::Resource(resource))
        }
        Item::Struct(mut strukt) => {
            strukt.is_exported = true;
            Ok(Item::Struct(strukt))
        }
        Item::Enum(mut enm) => {
            enm.is_exported = true;
            Ok(Item::Enum(enm))
        }
        Item::Trait(mut tr) => {
            tr.is_exported = true;
            Ok(Item::Trait(tr))
        }
        Item::Impl(_) => Err(Rich::custom(
            chumsky::span::SimpleSpan::new((), export_span.start..export_span.end),
            "implementation blocks cannot be exported",
        )),
    }
}

fn program_p<'a>() -> impl Parser<'a, &'a str, Program, ErrTy<'a>> {
    let item = choice((
        refined_alias_p().map(Item::Alias),
        resource_p().map(Item::Resource),
        struct_p().map(Item::Struct),
        enum_p().map(Item::Enum),
        trait_p().map(Item::Trait),
        impl_p().map(Item::Impl),
        func_p().map(Item::Func),
    ));

    let export_kw = kw("export").map_with(|_, e| to_span(e.span()));

    let exported_item = export_kw
        .then(choice((
            refined_alias_p().map(Item::Alias),
            resource_p().map(Item::Resource),
            struct_p().map(Item::Struct),
            enum_p().map(Item::Enum),
            trait_p().map(Item::Trait),
            impl_p().map(Item::Impl),
            func_p().map(Item::Func),
        )))
        .try_map(|(span, item), _| apply_export(item, span));

    let top_level = choice((
        import_decl_p().map(TopLevel::Import),
        exported_item.map(|item| TopLevel::Item(Box::new(item))),
        item.map(|item| TopLevel::Item(Box::new(item))),
    ));

    module_decl_p()
        .or_not()
        .then(top_level.repeated().collect::<Vec<_>>())
        .map(|(module_decl, items)| {
            let mut refined_aliases = Vec::with_capacity(items.len());
            let mut funcs = Vec::with_capacity(items.len());
            let mut resources = Vec::with_capacity(items.len());
            let mut structs = Vec::with_capacity(items.len());
            let mut enums = Vec::with_capacity(items.len());
            let mut traits = Vec::with_capacity(items.len());
            let mut impls = Vec::with_capacity(items.len());
            let mut imports = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    TopLevel::Import(i) => imports.push(i),
                    TopLevel::Item(item) => match *item {
                        Item::Alias(a) => refined_aliases.push(a),
                        Item::Func(f) => {
                            refined_aliases.extend(f.inline_aliases);
                            funcs.push(f.func);
                        }
                        Item::Resource(r) => resources.push(r),
                        Item::Struct(s) => structs.push(s),
                        Item::Enum(e) => enums.push(e),
                        Item::Trait(t) => traits.push(t),
                        Item::Impl(i) => {
                            refined_aliases.extend(i.inline_aliases);
                            impls.push(i.decl);
                        }
                    },
                }
            }
            Program {
                module: module_decl,
                imports,
                refined_aliases,
                resources,
                structs,
                enums,
                traits,
                impls,
                funcs,
            }
        })
        .then_ignore(end())
}

pub fn parse(src: &str) -> Result<Program, String> {
    let normalized = strip_comments_preserve_layout(src);
    let parser = program_p();
    let parsed = parser.parse(normalized.as_str()).into_result();
    parsed.map_err(|errs| {
        let mut messages: Vec<String> = Vec::with_capacity(errs.len() + 1);
        for e in errs {
            let span = e.span();
            let mut expected: Vec<String> = e.expected().map(|p| p.to_string()).collect();
            for (ctx, _) in e.contexts() {
                let label = ctx.to_string();
                if !expected.contains(&label) {
                    expected.push(label);
                }
            }
            let msg = if expected.is_empty() {
                format!("at {}..{}: error: {}", span.start, span.end, e)
            } else {
                format!(
                    "at {}..{}: error: {}; expected: {}",
                    span.start,
                    span.end,
                    e,
                    expected.join(", ")
                )
            };
            messages.push(msg);
        }

        let mut offset = 0usize;
        for line in normalized.split_inclusive('\n') {
            let line_no_nl = line.strip_suffix('\n').unwrap_or(line);
            let line_text = line_no_nl.strip_suffix('\r').unwrap_or(line_no_nl);
            let trimmed = line_text.trim_start();
            if (trimmed.starts_with("require ") || trimmed.starts_with("ensure "))
                && !trimmed.contains('{')
            {
                let keyword = trimmed.split_whitespace().next().unwrap_or("contract");
                let leading = line_text.len().saturating_sub(trimmed.len());
                let start = offset + leading;
                let end = start + keyword.len();
                messages.push(format!(
                    "at {}..{}: error: keyword `{}` must be followed by `{{ ... }}`",
                    start, end, keyword
                ));
            }
            if let Some(rel) = keyword_missing_brace(line_text, "invariant") {
                let start = offset + rel;
                let end = start + "invariant".len();
                messages.push(format!(
                    "at {}..{}: error: expected '{{' after `invariant`",
                    start, end
                ));
            }
            if let Some(rel) = keyword_missing_brace(line_text, "variant") {
                let start = offset + rel;
                let end = start + "variant".len();
                messages.push(format!(
                    "at {}..{}: error: expected '{{' after `variant`",
                    start, end
                ));
            }
            offset += line.len();
        }

        let should_hint = messages
            .iter()
            .any(|msg| msg.contains("expected: something else"));
        if should_hint {
            if !messages.iter().any(|msg| msg.contains("comma"))
                && looks_like_missing_comma(normalized.as_str())
            {
                messages.push("hint: expected comma between arguments".to_string());
            }
            if !messages.iter().any(|msg| msg.contains("')'"))
                && has_unclosed_paren(normalized.as_str())
            {
                messages.push("hint: expected ')'".to_string());
            }
        }
        if let Some((start, end)) = find_comma_grouped_number(normalized.as_str()) {
            messages.push(format!(
                "at {}..{}: error: comma separators are not allowed in numeric literals; use `_` (for example `1_000`)",
                start, end
            ));
        }
        if let Some(start) = find_untyped_lambda(normalized.as_str()) {
            let end = start.saturating_add(1);
            messages.push(format!(
                "at {}..{}: error: lambda parameters require type annotations (`name: Type`)",
                start, end
            ));
        }
        if let Some(start) = find_capture_list_lambda(normalized.as_str()) {
            let end = start.saturating_add(1);
            messages.push(format!(
                "at {}..{}: error: capture-list syntax is not supported in v1",
                start, end
            ));
        }
        if let Some((start, end)) = find_export_import(normalized.as_str()) {
            messages.push(format!(
                "at {}..{}: error: `export import` is not supported in v1; import directly in each module",
                start, end
            ));
        }
        if let Some((start, end)) = find_theorem_keyword(normalized.as_str()) {
            messages.push(format!(
                "at {}..{}: error: `theorem` keyword is reserved in milestone_3; theorem-grade is a release certification status, not syntax",
                start, end
            ));
        }
        messages.join("\n")
    })
}

// Structured parser error for machine-readable diagnostics
#[derive(Debug, Clone)]
pub struct ParserError {
    pub code: &'static str,
    pub message: String,
    pub start: usize,
    pub end: usize,
}

// Parse returning structured errors (preferred for --json-errors)
pub fn parse_errors(src: &str) -> Result<Program, Vec<ParserError>> {
    let normalized = strip_comments_preserve_layout(src);
    let parser = program_p();
    let parsed = parser.parse(normalized.as_str()).into_result();
    match parsed {
        Ok(ast) => Ok(ast),
        Err(errs) => {
            let mut items: Vec<ParserError> = Vec::with_capacity(errs.len() + 1);
            for e in errs {
                let span = e.span();
                let mut expected: Vec<String> = e.expected().map(|p| p.to_string()).collect();
                for (ctx, _) in e.contexts() {
                    let label = ctx.to_string();
                    if !expected.contains(&label) {
                        expected.push(label);
                    }
                }
                let msg = if expected.is_empty() {
                    format!("at {}..{}: error: {}", span.start, span.end, e)
                } else {
                    format!(
                        "at {}..{}: error: {}; expected: {}",
                        span.start,
                        span.end,
                        e,
                        expected.join(", ")
                    )
                };
                let missing_else = msg.contains("missing `else` in expression-form `if`");
                let theorem_deferred = msg.contains("`theorem` keyword is reserved in milestone_3");
                let (code, message) = if missing_else {
                    (
                        "P010",
                        format!(
                            "at {}..{}: error: missing `else` in expression-form `if`",
                            span.start, span.end
                        ),
                    )
                } else if theorem_deferred {
                    (
                        "P014",
                        format!(
                            "at {}..{}: error: `theorem` keyword is reserved in milestone_3; theorem-grade is a release certification status, not syntax",
                            span.start, span.end
                        ),
                    )
                } else {
                    ("P001", msg)
                };
                items.push(ParserError {
                    code,
                    message,
                    start: span.start,
                    end: span.end,
                });
            }
            let mut offset = 0usize;
            for line in normalized.split_inclusive('\n') {
                let line_no_nl = line.strip_suffix('\n').unwrap_or(line);
                let line_text = line_no_nl.strip_suffix('\r').unwrap_or(line_no_nl);
                let trimmed = line_text.trim_start();
                if (trimmed.starts_with("require ") || trimmed.starts_with("ensure "))
                    && !trimmed.contains('{')
                {
                    let keyword = trimmed.split_whitespace().next().unwrap_or("contract");
                    let leading = line_text.len().saturating_sub(trimmed.len());
                    let start = offset + leading;
                    let end = start + keyword.len();
                    items.push(ParserError {
                        code: "P001",
                        message: format!(
                            "at {}..{}: error: keyword `{}` must be followed by `{{ ... }}`",
                            start, end, keyword
                        ),
                        start,
                        end,
                    });
                }
                if let Some(rel) = keyword_missing_brace(line_text, "invariant") {
                    let start = offset + rel;
                    let end = start + "invariant".len();
                    items.push(ParserError {
                        code: "P001",
                        message: format!(
                            "at {}..{}: error: expected '{{' after `invariant`",
                            start, end
                        ),
                        start,
                        end,
                    });
                }
                if let Some(rel) = keyword_missing_brace(line_text, "variant") {
                    let start = offset + rel;
                    let end = start + "variant".len();
                    items.push(ParserError {
                        code: "P001",
                        message: format!(
                            "at {}..{}: error: expected '{{' after `variant`",
                            start, end
                        ),
                        start,
                        end,
                    });
                }
                offset += line.len();
            }
            if let Some((start, end)) = find_comma_grouped_number(normalized.as_str()) {
                items.push(ParserError {
                    code: "P001",
                    message: format!(
                        "at {}..{}: error: comma separators are not allowed in numeric literals; use `_` (for example `1_000`)",
                        start, end
                    ),
                    start,
                    end,
                });
            }
            if let Some(start) = find_untyped_lambda(normalized.as_str()) {
                let end = start.saturating_add(1);
                items.push(ParserError {
                    code: "P001",
                    message: format!(
                        "at {}..{}: error: lambda parameters require type annotations (`name: Type`)",
                        start, end
                    ),
                    start,
                    end,
                });
            }
            if let Some(start) = find_capture_list_lambda(normalized.as_str()) {
                let end = start.saturating_add(1);
                items.push(ParserError {
                    code: "P012",
                    message: format!(
                        "at {}..{}: error: capture-list syntax is not supported in v1",
                        start, end
                    ),
                    start,
                    end,
                });
            }
            if let Some((start, end)) = find_export_import(normalized.as_str()) {
                items.push(ParserError {
                    code: "P011",
                    message: format!(
                        "at {}..{}: error: `export import` is not supported in v1; import directly in each module",
                        start, end
                    ),
                    start,
                    end,
                });
            }
            if let Some((start, end)) = find_theorem_keyword(normalized.as_str()) {
                items.push(ParserError {
                    code: "P014",
                    message: format!(
                        "at {}..{}: error: `theorem` keyword is reserved in milestone_3; theorem-grade is a release certification status, not syntax",
                        start, end
                    ),
                    start,
                    end,
                });
            }
            let specialized_spans: Vec<(usize, usize)> = items
                .iter()
                .filter(|e| e.code == "P011" || e.code == "P012" || e.code == "P014")
                .map(|e| (e.start, e.end))
                .collect();
            if !specialized_spans.is_empty() {
                items.retain(|e| {
                    if e.code != "P001" {
                        return true;
                    }
                    !specialized_spans
                        .iter()
                        .any(|(s, t)| e.start < *t && *s < e.end)
                });
            }
            Err(items)
        }
    }
}
