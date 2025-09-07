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

