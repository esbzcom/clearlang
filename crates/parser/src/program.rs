use chumsky::prelude::*;
use crate::ErrTy;
use crate::func::func_p;
use lumi_ast::Program;

fn program_p<'a>() -> impl Parser<'a, &'a str, Program, ErrTy<'a>> {
    func_p()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|funcs| Program { funcs })
        .then_ignore(end())
}

pub fn parse(src: &str) -> Result<Program, String> {
    program_p().parse(src).into_result().map_err(|errs| {
        errs
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
            .collect::<Vec<_>>()
            .join("\n")
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
                    ParserError { code: "P001", message: msg, start: span.start, end: span.end }
                })
                .collect();
            Err(items)
        }
    }
}
