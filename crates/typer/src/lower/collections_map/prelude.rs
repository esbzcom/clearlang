use crate::check::{base_type, infer_expr_type};
use anyhow::Result;
use clg_ast::{Expr, Type};
use clg_ir::{BinOpIR, Instr, IrType, Value};

use super::collection_types::map_key_val_type;
use super::collections_helpers::{
    emit_cap_from_len, emit_collection_cap, emit_collection_data_ptr, emit_collection_header,
    emit_collection_len, emit_collection_payload_guard, emit_find_index,
};
use super::layout::{map_entry_layout, tuple_layout};
use super::{
    emit_alloc, emit_alloc_dyn, emit_int_const, emit_memcpy_bytes, emit_ptr_add, fresh,
    load_value_copy, lower_expr, store_value, zero_value_for_type, LowerCtx,
};

fn emit_tuple_pair(
    ctx: &mut LowerCtx<'_>,
    first_ty: Type,
    first: Value,
    second_ty: Type,
    second: Value,
) -> Result<Value> {
    let layout = tuple_layout(
        &[first_ty.clone(), second_ty.clone()],
        ctx.aliases,
        ctx.std_types,
    )?;
    let ptr = emit_alloc(ctx, layout.size, layout.align);
    let first_off = *layout
        .offsets
        .first()
        .ok_or_else(|| anyhow::anyhow!("tuple offset missing"))?;
    let second_off = *layout
        .offsets
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("tuple offset missing"))?;
    store_value(ctx, &first_ty, ptr, first_off, first)?;
    store_value(ctx, &second_ty, ptr, second_off, second)?;
    Ok(ptr)
}

