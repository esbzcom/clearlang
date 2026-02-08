use std::path::Path;

use anyhow::anyhow;
use clg_ast::Span;

use crate::commands::helpers::{make_single_json_error, CommandError};

pub(super) fn module_error(
    code: &'static str,
    message: impl Into<String>,
    file: &Path,
    span: Span,
    json_errors: bool,
) -> anyhow::Error {
    if json_errors {
        CommandError::json(make_single_json_error(
            code, "build", message, file, span.start, span.end, None,
        ))
        .into()
    } else {
        anyhow!(message.into())
    }
}
