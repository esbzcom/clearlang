use chumsky::prelude::*;

// Shared error type alias across parser modules
pub(crate) type ErrTy<'a> = extra::Err<Rich<'a, char>>;

mod alias;
mod contract_decl;
mod enum_decl;
mod expr;
mod func;
mod generics;
mod impl_decl;
mod literals;
mod module_import;
mod path;
mod program;
mod resource;
mod struct_decl;
mod tokens;
mod trait_decl;
mod types;

pub use program::{parse, parse_errors, ParserError};
