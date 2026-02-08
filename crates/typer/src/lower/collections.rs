use anyhow::Result;
use clg_ast::{Expr, Type};
use clg_ir::Value;

use super::collection_types::normalize_collection_callee;
use super::collections_list::lower_list_call;
use super::collections_map::lower_map_call;
use super::collections_set::lower_set_call;
use super::collections_slice::lower_slice_call;
use super::LowerCtx;

pub(super) fn lower_collection_call<'a>(
    ctx: &mut LowerCtx<'a>,
    call_expr: &'a Expr,
    callee: &str,
    args: &'a [Expr],
    expected: Option<&Type>,
) -> Result<Option<Value>> {
    let callee = normalize_collection_callee(callee);
    if let Some(val) = lower_slice_call(ctx, callee, args)? {
        return Ok(Some(val));
    }
    if let Some(val) = lower_list_call(ctx, call_expr, callee, args, expected)? {
        return Ok(Some(val));
    }
    if let Some(val) = lower_set_call(ctx, call_expr, callee, args, expected)? {
        return Ok(Some(val));
    }
    if let Some(val) = lower_map_call(ctx, call_expr, callee, args, expected)? {
        return Ok(Some(val));
    }
    Ok(None)
}
