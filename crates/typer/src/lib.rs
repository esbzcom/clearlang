use anyhow::Result;
use lumi_ast::Program;
use lumi_ir::PlaceHolder;

/// Temporary stub: accept the real AST Program and return a placeholder IR.
/// (We’ll replace this in Phase 3 when we add a real IR & lowering.)
pub fn check(_ast: &Program) -> Result<PlaceHolder> {
    Ok(PlaceHolder)
}
