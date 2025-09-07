use chumsky::prelude::*;

// Shared error type alias across parser modules
pub(crate) type ErrTy<'a> = extra::Err<Rich<'a, char>>;

mod tokens;
mod types;
mod literals;
mod path;
mod expr;
mod func;
mod program;

pub use program::parse;
