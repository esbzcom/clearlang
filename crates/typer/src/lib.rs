mod builtins;
mod check;
mod errors;
mod lower;

pub use check::check;
pub use errors::TyperError;

// Type-check only (no lowering) — used by typer tests and tooling in Phase 4.5
pub fn type_check_only(ast: &clg_ast::Program) -> anyhow::Result<()> {
    check::type_check_only(ast)
}

// Re-export internal modules for tests if needed
