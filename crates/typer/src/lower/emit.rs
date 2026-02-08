use anyhow::Result;
use clg_ir::{BinOpIR, Instr, IrType, Value};

use super::{emit_u64_const, LowerCtx};

pub(crate) fn emit_int_const(ctx: &mut LowerCtx<'_>, n: i64) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst,
        ty: IrType::Int,
        n,
    });
    dst
}

pub(crate) fn emit_alloc(ctx: &mut LowerCtx<'_>, size: u32, align: u32) -> Value {
    let dst = fresh(ctx);
    let align = align.max(1);
    ctx.body.push(Instr::Alloc { dst, size, align });
    dst
}

pub(crate) fn emit_alloc_dyn(ctx: &mut LowerCtx<'_>, size: Value, align: u32) -> Value {
    let dst = fresh(ctx);
    let align = align.max(1);
    ctx.body.push(Instr::AllocDyn { dst, size, align });
    dst
}

pub(crate) fn emit_ptr_add(ctx: &mut LowerCtx<'_>, ptr: Value, offset: Value) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst,
        op: BinOpIR::Add,
        lhs: ptr,
        rhs: offset,
        ty: IrType::Int,
    });
    dst
}

pub(crate) fn emit_load_i32(ctx: &mut LowerCtx<'_>, ptr: Value, offset: u32) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::Load {
        dst,
        ptr,
        offset,
        ty: IrType::Int,
    });
    dst
}

pub(crate) fn emit_store_i32(ctx: &mut LowerCtx<'_>, ptr: Value, offset: u32, src: Value) {
    ctx.body.push(Instr::Store {
        ptr,
        src,
        offset,
        ty: IrType::Int,
    });
}

pub(crate) fn emit_memcpy_bytes(
    ctx: &mut LowerCtx<'_>,
    src_ptr: Value,
    dst_ptr: Value,
    byte_len: Value,
) -> Result<()> {
    let idx = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst: idx,
        ty: IrType::Int,
        n: 0,
    });
    let one = emit_int_const(ctx, 1);

    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::LoopBegin);
    let cond = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: cond,
        op: BinOpIR::Lt,
        lhs: idx,
        rhs: byte_len,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::BrIfEqz { cond, depth: 1 });

    let src_addr = emit_ptr_add(ctx, src_ptr, idx);
    let dst_addr = emit_ptr_add(ctx, dst_ptr, idx);
    let byte = fresh(ctx);
    ctx.body.push(Instr::Load {
        dst: byte,
        ptr: src_addr,
        offset: 0,
        ty: IrType::U8,
    });
    ctx.body.push(Instr::Store {
        ptr: dst_addr,
        src: byte,
        offset: 0,
        ty: IrType::U8,
    });

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
    Ok(())
}

pub(crate) fn emit_zero_for_mem_ty(ctx: &mut LowerCtx<'_>, mem_ty: IrType) -> Value {
    match mem_ty {
        IrType::U64 => emit_u64_const(ctx, 0),
        IrType::U8 => {
            let dst = fresh(ctx);
            ctx.body.push(Instr::IConst {
                dst,
                ty: IrType::U8,
                n: 0,
            });
            dst
        }
        IrType::Bool => emit_bool_const(ctx, false),
        _ => emit_int_const(ctx, 0),
    }
}

pub(crate) fn emit_bool_const(ctx: &mut LowerCtx<'_>, value: bool) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst,
        ty: IrType::Bool,
        n: if value { 1 } else { 0 },
    });
    dst
}

pub(crate) fn mem_layout_for_ir(ty: IrType) -> (u32, u32) {
    match ty {
        IrType::U8 => (1, 1),
        IrType::U64 => (8, 8),
        _ => (4, 4),
    }
}

pub(crate) fn fresh(ctx: &mut LowerCtx<'_>) -> Value {
    let v = Value(ctx.next);
    ctx.next += 1;
    v
}
