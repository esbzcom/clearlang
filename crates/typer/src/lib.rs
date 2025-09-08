mod errors;
mod builtins;
mod check;
mod lower;

pub use errors::TyperError;
pub use check::check;
use anyhow::{Result};
use lumi_ast::Program;
use lumi_ir::Module;

/// Type-check the Lumi AST and return a lowered IR module on success.
/// Phase 3.1–3.4: checks Int/Bool, variables, calls, binops, arity, and returns,
/// then lowers AST → IR using a simple SSA-like scheme.
pub fn check(ast: &Program) -> Result<Module> { check::check(ast) }

// Re-export internal modules for tests if needed
