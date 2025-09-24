use chumsky::prelude::*;

// Shared error type alias across parser modules
pub(crate) type ErrTy<'a> = extra::Err<Rich<'a, char>>;

mod expr;
mod func;
mod literals;
mod path;
mod program;
mod tokens;
mod types;

pub use program::{parse, parse_errors, ParserError};
