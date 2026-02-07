use anyhow::Result;
use clg_ast::{Expr, Type};
use clg_ir::Value;

use super::{lower_expr, LowerCtx};

pub(super) fn lower_return_expr<'a>(
    ctx: &mut LowerCtx<'a>,
    expr: &'a Expr,
    expected: Option<Type>,
) -> Result<Value> {
    // For expression-bodied functions, `return e` is equivalent to `e`.
    // Lower inner expression; the enclosing function appends the Ret.
    lower_expr(ctx, expr, expected)
}

pub(super) fn lower_var_expr(ctx: &LowerCtx<'_>, name: &str) -> Result<Value> {
    ctx.env
        .get(name)
        .copied()
        .ok_or_else(|| anyhow::anyhow!(format!("unknown variable `{}`", name)))
}

pub(super) fn lower_unary_expr() -> Result<Value> {
    anyhow::bail!("unary operators are not supported in codegen yet")
}
