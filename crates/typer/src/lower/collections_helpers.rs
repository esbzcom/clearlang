use crate::check::AliasMap;
use anyhow::Result;
use clg_ast::{Span, Type};
use clg_ir::{BinOpIR, GuardKind, Instr, IrType, TrapCode, Value};

use super::eq::emit_eq_for_type;
use super::{
    emit_alloc, emit_int_const, emit_load_i32, emit_ptr_add, emit_store_i32, fresh,
    load_value_borrow, LowerCtx, COLLECTION_CAP_OFFSET, COLLECTION_DATA_OFFSET,
    COLLECTION_FLAGS_OFFSET, COLLECTION_HEADER_ALIGN, COLLECTION_HEADER_SIZE, COLLECTION_LEN_OFFSET,
};

pub(super) fn emit_collection_guard(ctx: &mut LowerCtx<'_>, cond: Value, span: Span) {
    ctx.body.push(Instr::Guard {
        cond,
        trap: TrapCode::CollectionBounds,
        span: Some((span.start as u32, span.end as u32)),
        detail: GuardKind::Require,
    });
}

fn emit_memory_bytes(ctx: &mut LowerCtx<'_>) -> Value {
    let pages = fresh(ctx);
    ctx.body.push(Instr::MemorySize { dst: pages });
    let shift = emit_int_const(ctx, 16);
    let bytes = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: bytes,
        op: BinOpIR::Shl,
        lhs: pages,
        rhs: shift,
        ty: IrType::Int,
    });
    let max_pages = emit_int_const(ctx, 65536);
    let is_max = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: is_max,
        op: BinOpIR::Eq,
        lhs: pages,
        rhs: max_pages,
        ty: IrType::Int,
    });
    let max_bytes = emit_int_const(ctx, -1);
    let capped = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst: capped,
        cond: is_max,
        then_v: max_bytes,
        else_v: bytes,
    });
    capped
}

fn emit_collection_ptr_guard(ctx: &mut LowerCtx<'_>, ptr: Value) {
    let zero = emit_int_const(ctx, 0);
    let not_zero = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: not_zero,
        op: BinOpIR::Neq,
        lhs: ptr,
        rhs: zero,
        ty: IrType::Int,
    });
    let align_mask = emit_int_const(ctx, (COLLECTION_HEADER_ALIGN - 1) as i64);
    let masked = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: masked,
        op: BinOpIR::And,
        lhs: ptr,
        rhs: align_mask,
        ty: IrType::Int,
    });
    let aligned = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: aligned,
        op: BinOpIR::Eq,
        lhs: masked,
        rhs: zero,
        ty: IrType::Int,
    });

    let header_size = emit_int_const(ctx, COLLECTION_HEADER_SIZE as i64);
    let end = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: end,
        op: BinOpIR::Add,
        lhs: ptr,
        rhs: header_size,
        ty: IrType::Int,
    });
    let no_wrap = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: no_wrap,
        op: BinOpIR::LeU,
        lhs: ptr,
        rhs: end,
        ty: IrType::Int,
    });
    let mem_bytes = emit_memory_bytes(ctx);
    let within = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: within,
        op: BinOpIR::LeU,
        lhs: end,
        rhs: mem_bytes,
        ty: IrType::Int,
    });

    let tmp = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp,
        op: BinOpIR::And,
        lhs: not_zero,
        rhs: aligned,
        ty: IrType::Int,
    });
    let ok = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: ok,
        op: BinOpIR::And,
        lhs: tmp,
        rhs: no_wrap,
        ty: IrType::Int,
    });
    let ok2 = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: ok2,
        op: BinOpIR::And,
        lhs: ok,
        rhs: within,
        ty: IrType::Int,
    });

    ctx.body.push(Instr::Guard {
        cond: ok2,
        trap: TrapCode::InvalidBuffer,
        span: None,
        detail: GuardKind::Require,
    });
}

pub(super) fn emit_collection_len(ctx: &mut LowerCtx<'_>, ptr: Value) -> Value {
    emit_collection_ptr_guard(ctx, ptr);
    emit_load_i32(ctx, ptr, COLLECTION_LEN_OFFSET)
}

pub(super) fn emit_collection_data_ptr(ctx: &mut LowerCtx<'_>, ptr: Value) -> Value {
    emit_collection_ptr_guard(ctx, ptr);
    emit_load_i32(ctx, ptr, COLLECTION_DATA_OFFSET)
}

pub(super) fn emit_collection_cap(ctx: &mut LowerCtx<'_>, ptr: Value) -> Value {
    emit_collection_ptr_guard(ctx, ptr);
    emit_load_i32(ctx, ptr, COLLECTION_CAP_OFFSET)
}

pub(super) fn emit_collection_payload_guard(
    ctx: &mut LowerCtx<'_>,
    data_ptr: Value,
    len: Value,
    cap: Value,
    stride: u32,
    align: u32,
) {
    let zero = emit_int_const(ctx, 0);
    let not_zero = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: not_zero,
        op: BinOpIR::Neq,
        lhs: data_ptr,
        rhs: zero,
        ty: IrType::Int,
    });

    let align = align.max(1);
    let align_mask = emit_int_const(ctx, (align - 1) as i64);
    let masked = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: masked,
        op: BinOpIR::And,
        lhs: data_ptr,
        rhs: align_mask,
        ty: IrType::Int,
    });
    let aligned = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: aligned,
        op: BinOpIR::Eq,
        lhs: masked,
        rhs: zero,
        ty: IrType::Int,
    });

    let len_nonneg = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: len_nonneg,
        op: BinOpIR::Ge,
        lhs: len,
        rhs: zero,
        ty: IrType::Int,
    });
    let cap_gt_zero = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: cap_gt_zero,
        op: BinOpIR::Gt,
        lhs: cap,
        rhs: zero,
        ty: IrType::Int,
    });
    let len_le_cap = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: len_le_cap,
        op: BinOpIR::LeU,
        lhs: len,
        rhs: cap,
        ty: IrType::Int,
    });
    let max_len = emit_int_const(ctx, (i32::MAX as i64) / stride as i64);
    let len_fits = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: len_fits,
        op: BinOpIR::Le,
        lhs: len,
        rhs: max_len,
        ty: IrType::Int,
    });

    let stride_val = emit_int_const(ctx, stride as i64);
    let bytes = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: bytes,
        op: BinOpIR::Mul,
        lhs: len,
        rhs: stride_val,
        ty: IrType::Int,
    });
    let end = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: end,
        op: BinOpIR::Add,
        lhs: data_ptr,
        rhs: bytes,
        ty: IrType::Int,
    });
    let no_wrap = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: no_wrap,
        op: BinOpIR::LeU,
        lhs: data_ptr,
        rhs: end,
        ty: IrType::Int,
    });
    let mem_bytes = emit_memory_bytes(ctx);
    let within = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: within,
        op: BinOpIR::LeU,
        lhs: end,
        rhs: mem_bytes,
        ty: IrType::Int,
    });

    let tmp = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp,
        op: BinOpIR::And,
        lhs: not_zero,
        rhs: aligned,
        ty: IrType::Int,
    });
    let tmp2 = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp2,
        op: BinOpIR::And,
        lhs: tmp,
        rhs: len_nonneg,
        ty: IrType::Int,
    });
    let tmp3 = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp3,
        op: BinOpIR::And,
        lhs: tmp2,
        rhs: cap_gt_zero,
        ty: IrType::Int,
    });
    let tmp4 = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp4,
        op: BinOpIR::And,
        lhs: tmp3,
        rhs: len_le_cap,
        ty: IrType::Int,
    });
    let tmp5 = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp5,
        op: BinOpIR::And,
        lhs: tmp4,
        rhs: len_fits,
        ty: IrType::Int,
    });
    let tmp6 = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp6,
        op: BinOpIR::And,
        lhs: tmp5,
        rhs: no_wrap,
        ty: IrType::Int,
    });
    let ok = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: ok,
        op: BinOpIR::And,
        lhs: tmp6,
        rhs: within,
        ty: IrType::Int,
    });

    ctx.body.push(Instr::Guard {
        cond: ok,
        trap: TrapCode::InvalidBuffer,
        span: None,
        detail: GuardKind::Require,
    });
}

pub(super) fn emit_collection_header(
    ctx: &mut LowerCtx<'_>,
    len: Value,
    cap: Value,
    data_ptr: Value,
) -> Value {
    let header = emit_alloc(ctx, COLLECTION_HEADER_SIZE, COLLECTION_HEADER_ALIGN);
    let zero = emit_int_const(ctx, 0);
    emit_store_i32(ctx, header, COLLECTION_LEN_OFFSET, len);
    emit_store_i32(ctx, header, COLLECTION_CAP_OFFSET, cap);
    emit_store_i32(ctx, header, COLLECTION_FLAGS_OFFSET, zero);
    emit_store_i32(ctx, header, COLLECTION_DATA_OFFSET, data_ptr);
    header
}

pub(super) fn emit_cap_from_len(ctx: &mut LowerCtx<'_>, len: Value) -> Value {
    let zero = emit_int_const(ctx, 0);
    let one = emit_int_const(ctx, 1);
    let gt_zero = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: gt_zero,
        op: BinOpIR::Gt,
        lhs: len,
        rhs: zero,
        ty: IrType::Int,
    });
    let cap = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst: cap,
        cond: gt_zero,
        then_v: len,
        else_v: one,
    });
    cap
}

pub(super) fn emit_find_index(
    ctx: &mut LowerCtx<'_>,
    data_ptr: Value,
    len: Value,
    stride: u32,
    key_val: Value,
    key_ty: &Type,
    key_offset: u32,
    aliases: &AliasMap,
) -> Result<(Value, Value)> {
    let found = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst: found,
        ty: IrType::Bool,
        n: 0,
    });
    let found_idx = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst: found_idx,
        ty: IrType::Int,
        n: 0,
    });
    let idx = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst: idx,
        ty: IrType::Int,
        n: 0,
    });
    let stride_val = emit_int_const(ctx, stride as i64);

    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::LoopBegin);
    let cond = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: cond,
        op: BinOpIR::Lt,
        lhs: idx,
        rhs: len,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::BrIfEqz { cond, depth: 1 });

    let offset = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: offset,
        op: BinOpIR::Mul,
        lhs: idx,
        rhs: stride_val,
        ty: IrType::Int,
    });
    let base_ptr = emit_ptr_add(ctx, data_ptr, offset);
    let key_ptr = if key_offset == 0 {
        base_ptr
    } else {
        let key_off_val = emit_int_const(ctx, key_offset as i64);
        emit_ptr_add(ctx, base_ptr, key_off_val)
    };

    let key_loaded = load_value_borrow(ctx, key_ty, key_ptr, 0)?;
    let eq = emit_eq_for_type(ctx, key_ty, key_loaded, key_val, aliases)?;

    ctx.body.push(Instr::IBin {
        dst: found,
        op: BinOpIR::Or,
        lhs: found,
        rhs: eq,
        ty: IrType::Bool,
    });
    ctx.body.push(Instr::ISelect {
        dst: found_idx,
        cond: eq,
        then_v: idx,
        else_v: found_idx,
    });

    let one = emit_int_const(ctx, 1);
    ctx.body.push(Instr::IBin {
        dst: idx,
        op: BinOpIR::Add,
        lhs: idx,
        rhs: one,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::Br { depth: 0 });
    ctx.body.push(Instr::LoopEnd);
    ctx.body.push(Instr::BlockEnd);

    Ok((found, found_idx))
}
