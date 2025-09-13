mod errors;
mod builtins;
mod check;
mod lower;

pub use errors::TyperError;
pub use check::check;

// Type-check only (no lowering) — used by typer tests and tooling in Phase 4.5
pub fn type_check_only(ast: &lumi_ast::Program) -> anyhow::Result<()> {
    check::type_check_only(ast)
}

// Re-export internal modules for tests if needed
