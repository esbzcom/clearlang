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

fn has_unclosed_paren(src: &str) -> bool {
    let mut depth = 0u32;
    let mut in_string = false;
    for b in src.bytes() {
        if b == b'"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match b {
            b'(' => depth = depth.saturating_add(1),
            b')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth > 0
}

fn looks_like_missing_comma(src: &str) -> bool {
    let bytes = src.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            let mut j = i + 1;
            let mut saw_gap = false;
            let mut saw_comma = false;
            let mut saw_operator = false;
            let mut prev_token = false;
            let mut pending_gap = false;
            while j < bytes.len() && bytes[j] != b')' {
                let b = bytes[j];
                if b == b'"' {
                    if pending_gap {
                        saw_gap = true;
                    }
                    prev_token = true;
                    pending_gap = false;
                    j += 1;
                    while j < bytes.len() && bytes[j] != b'"' {
                        j += 1;
                    }
                } else if b == b',' {
                    saw_comma = true;
                    prev_token = false;
                    pending_gap = false;
                } else if matches!(
                    b,
                    b'+' | b'-'
                        | b'*'
                        | b'/'
                        | b'%'
                        | b'<'
                        | b'>'
                        | b'='
                        | b'!'
                        | b'&'
                        | b'|'
                        | b'?'
                        | b':'
                        | b'.'
                ) {
                    saw_operator = true;
                    prev_token = false;
                    pending_gap = false;
                } else if b.is_ascii_whitespace() {
                    if prev_token {
                        pending_gap = true;
                    }
                } else if b.is_ascii_alphanumeric() || b == b'_' {
                    if pending_gap {
                        saw_gap = true;
                    }
                    prev_token = true;
                    pending_gap = false;
                } else {
                    prev_token = false;
                    pending_gap = false;
                }
                j += 1;
            }
            if j < bytes.len() && saw_gap && !saw_comma && !saw_operator {
                return true;
            }
            i = j;
        }
        i += 1;
    }
    false
}

fn keyword_missing_brace(line: &str, keyword: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let key = keyword.as_bytes();
    let mut idx = 0usize;
    while idx + key.len() <= bytes.len() {
        let Some(rel) = line[idx..].find(keyword) else {
            break;
        };
        let start = idx + rel;
        let end = start + key.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric() && bytes[start - 1] != b'_';
        let after_ok = end == bytes.len() || !bytes[end].is_ascii_alphanumeric() && bytes[end] != b'_';
        if before_ok && after_ok {
            let mut j = end;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j >= bytes.len() || bytes[j] != b'{' {
                return Some(start);
            }
        }
        idx = end;
    }
    None
}

pub fn parse(src: &str) -> Result<Program, String> {
    program_p().parse(src).into_result().map_err(|errs| {
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
            if !messages.iter().any(|msg| msg.contains("comma")) && looks_like_missing_comma(src) {
                messages.push("hint: expected comma between arguments".to_string());
            }
            if !messages.iter().any(|msg| msg.contains("')'")) && has_unclosed_paren(src) {
                messages.push("hint: expected ')'".to_string());
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
                let (code, message) = if missing_else {
                    (
                        "P010",
                        format!(
                            "at {}..{}: error: missing `else` in expression-form `if`",
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
            Err(items)
        }
    }
}
