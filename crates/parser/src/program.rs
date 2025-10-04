use crate::func::func_p;
use crate::resource::resource_p;
use crate::ErrTy;
use chumsky::prelude::*;
use clg_ast::Program;

#[derive(Debug)]
enum Item {
    Func(clg_ast::Func),
    Resource(clg_ast::Resource),
}

fn program_p<'a>() -> impl Parser<'a, &'a str, Program, ErrTy<'a>> {
    let item = choice((resource_p().map(Item::Resource), func_p().map(Item::Func)));

    item.repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|items| {
            let mut funcs = Vec::new();
            let mut resources = Vec::new();
            for item in items {
                match item {
                    Item::Func(f) => funcs.push(f),
                    Item::Resource(r) => resources.push(r),
                }
            }
            Program { resources, funcs }
        })
        .then_ignore(end())
}

pub fn parse(src: &str) -> Result<Program, String> {
    program_p().parse(src).into_result().map_err(|errs| {
        let mut messages: Vec<String> = errs
            .into_iter()
            .map(|e| {
                let span = e.span();
                let expected: Vec<String> = e.expected().map(|p| p.to_string()).collect();
                if expected.is_empty() {
                    format!("error at {}..{}: {}", span.start, span.end, e)
                } else {
                    format!(
                        "error at {}..{}: {}; expected: {}",
                        span.start,
                        span.end,
                        e,
                        expected.join(", ")
                    )
                }
            })
            .collect();

        for (offset, line) in src.lines().enumerate() {
            let trimmed = line.trim_start();
            if (trimmed.starts_with("require ") || trimmed.starts_with("ensure ")) && !trimmed.contains('{') {
                messages.push(format!(
                    "line {}: keyword `{}` must be followed by `{{ ... }}`",
                    offset + 1,
                    trimmed.split_whitespace().next().unwrap_or("contract"),
                ));
            }
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
            let items: Vec<ParserError> = errs
                .into_iter()
                .map(|e| {
                    let span = e.span();
                    let expected: Vec<String> = e.expected().map(|p| p.to_string()).collect();
                    let msg = if expected.is_empty() {
                        format!("error at {}..{}: {}", span.start, span.end, e)
                    } else {
                        format!(
                            "error at {}..{}: {}; expected: {}",
                            span.start,
                            span.end,
                            e,
                            expected.join(", ")
                        )
                    };
                    ParserError {
                        code: "P001",
                        message: msg,
                        start: span.start,
                        end: span.end,
                    }
                })
                .collect();
            Err(items)
        }
    }
}
