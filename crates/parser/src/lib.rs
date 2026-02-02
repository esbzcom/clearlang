use chumsky::prelude::*;

// Shared error type alias across parser modules
pub(crate) type ErrTy<'a> = extra::Err<Rich<'a, char>>;

mod alias;
mod generics;
mod expr;
mod func;
mod literals;
mod path;
mod program;
mod resource;
mod struct_decl;
mod enum_decl;
mod trait_decl;
mod impl_decl;
mod tokens;
mod types;

pub use program::{parse, parse_errors, ParserError};
