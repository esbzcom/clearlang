mod builtins;
mod check;
mod errors;
mod guards;
mod lower;
mod vc;

pub use builtins::{
    all_builtin_sigs, builtin_compat_alias_target, builtin_route, builtin_sigs,
    non_abi_builtin_sigs, std_int_compat_builtin, std_text_compat_builtin,
    verified_std_abi_value_symbols, BuiltinRoute, StdIntCompatBuiltin, StdTextCompatBuiltin,
};
pub use check::{
    check, check_with_vcs, check_with_vcs_with_std, check_with_vcs_with_std_and_external,
    ExternalBuiltinSig, StdTypeInfo, StdTypeMap, TypecheckOutput,
};
pub use errors::TyperError;
pub use vc::{
    expr_to_source, generate_vcs, generate_vcs_with_dependencies, AssumptionBoundary,
    AssumptionCategory, AssumptionDependencies, ContractExpr, ExprSnapshot, RefinementAttachment,
    RefinementAttachmentDetail, RefinementAttachmentKind, RefinementFlowDetail, RefinementFlowKind,
    RefinementPremise, VerificationCondition,
};

// Type-check only (no lowering) — used by typer tests and tooling in Phase 4.5
pub fn type_check_only(ast: &clg_ast::Program) -> anyhow::Result<()> {
    check::type_check_only(ast)
}

pub fn type_check_only_with_std(
    ast: &clg_ast::Program,
    std_types: &StdTypeMap,
) -> anyhow::Result<()> {
    check::type_check_only_with_std(ast, std_types)
}

// Re-export internal modules for tests if needed
