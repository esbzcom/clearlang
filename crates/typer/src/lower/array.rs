use clg_ir::{BinOpIR, GuardKind, Instr, IrType, TrapCode, Value};

use super::{emit_int_const, emit_load_i32, emit_ptr_add_const, fresh, LowerCtx};

pub(super) const ARRAY_HEADER_SIZE: u32 = 8;
pub(super) const ARRAY_HEADER_ALIGN: u32 = 4;
pub(super) const ARRAY_HEADER_LEN_OFFSET: u32 = 0;
pub(super) const ARRAY_HEADER_DATA_OFFSET: u32 = 4;
const BYTES_HEADER_LEN_OFFSET: u32 = 0;
const BYTES_HEADER_DATA_OFFSET: u32 = 4;

pub(super) fn emit_array_len(ctx: &mut LowerCtx<'_>, ptr: Value) -> Value {
    emit_load_i32(ctx, ptr, ARRAY_HEADER_LEN_OFFSET)
}

pub(super) fn emit_array_data_ptr(ctx: &mut LowerCtx<'_>, ptr: Value) -> Value {
    emit_load_i32(ctx, ptr, ARRAY_HEADER_DATA_OFFSET)
}

fn emit_bytes_len(ctx: &mut LowerCtx<'_>, ptr: Value) -> Value {
    emit_load_i32(ctx, ptr, BYTES_HEADER_LEN_OFFSET)
}

pub(super) fn emit_bytes_data_ptr(ctx: &mut LowerCtx<'_>, ptr: Value) -> Value {
    emit_ptr_add_const(ctx, ptr, BYTES_HEADER_DATA_OFFSET)
}

pub(super) fn emit_array_len_guard(ctx: &mut LowerCtx<'_>, ptr: Value, expected_len: u32) {
    let actual_len = emit_array_len(ctx, ptr);
    let expected = emit_int_const(ctx, expected_len as i64);
    let ok = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: ok,
        op: BinOpIR::Eq,
        lhs: actual_len,
        rhs: expected,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::Guard {
        cond: ok,
        trap: TrapCode::ContractViolation,
        span: None,
        detail: GuardKind::Require,
    });
}

pub(super) fn emit_bytes_len_guard(ctx: &mut LowerCtx<'_>, ptr: Value, expected_len: u32) {
    let actual_len = emit_bytes_len(ctx, ptr);
    let expected = emit_int_const(ctx, expected_len as i64);
    let ok = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: ok,
        op: BinOpIR::Eq,
        lhs: actual_len,
        rhs: expected,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::Guard {
        cond: ok,
        trap: TrapCode::ContractViolation,
        span: None,
        detail: GuardKind::Require,
    });
}
