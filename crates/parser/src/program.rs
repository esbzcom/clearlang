use crate::alias::refined_alias_p;
use crate::func::func_p;
use crate::resource::resource_p;
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::Program;

#[derive(Debug)]
enum Item {
    Alias(clg_ast::RefinedAlias),
    Func(clg_ast::Func),
    Resource(clg_ast::Resource),
}

fn program_p<'a>() -> impl Parser<'a, &'a str, Program, ErrTy<'a>> {
    let item = choice((
        refined_alias_p().map(Item::Alias),
        resource_p().map(Item::Resource),
        func_p().map(Item::Func),
    ));

    item.repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|items| {
            let mut refined_aliases = Vec::with_capacity(items.len());
            let mut funcs = Vec::with_capacity(items.len());
            let mut resources = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Item::Alias(a) => refined_aliases.push(a),
                    Item::Func(f) => funcs.push(f),
                    Item::Resource(r) => resources.push(r),
                }
            }
            Program {
                refined_aliases,
                resources,
                funcs,
            }
        })
        .then_ignore(end())
}

pub fn parse(src: &str) -> Result<Program, String> {
    program_p().parse(src).into_result().map_err(|errs| {
        let mut messages: Vec<String> = Vec::with_capacity(errs.len() + 1);
        for e in errs {
            let span = e.span();
            let expected: Vec<String> = e.expected().map(|p| p.to_string()).collect();
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
        for line in src.split_inclusive('\n') {
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
            offset += line.len();
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
    match program_p().parse(src).into_result() {
        Ok(ast) => Ok(ast),
        Err(errs) => {
            let mut items: Vec<ParserError> = Vec::with_capacity(errs.len() + 1);
            for e in errs {
                let span = e.span();
                let expected: Vec<String> = e.expected().map(|p| p.to_string()).collect();
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
                items.push(ParserError {
                    code: "P001",
                    message: msg,
                    start: span.start,
                    end: span.end,
                });
            }
            let mut offset = 0usize;
            for line in src.split_inclusive('\n') {
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
                offset += line.len();
            }
            Err(items)
        }
    }
}
