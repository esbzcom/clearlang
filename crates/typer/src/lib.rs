mod builtins;
mod check;
mod errors;
mod guards;
mod lower;
mod vc;

pub use check::{check, check_with_vcs, TypecheckOutput};
pub use errors::TyperError;
pub use vc::{generate_vcs, ContractExpr, VerificationCondition};

// Type-check only (no lowering) — used by typer tests and tooling in Phase 4.5
pub fn type_check_only(ast: &clg_ast::Program) -> anyhow::Result<()> {
    check::type_check_only(ast)
}

// Re-export internal modules for tests if needed
