use anyhow::Result;
use clg_ir::{IrType, TrapCode, Value};
use wasm_encoder::{BlockType, InstructionSink, MemArg};

use crate::intrinsics::runtime::{emit_runtime_trap, TrapOperand};

use super::super::{StringPool, HEAP_PTR_GLOBAL};

pub(super) fn emit_iconst(insts: &mut InstructionSink<'_>, dst: Value, n: i64, ty: IrType) {
    match ty {
        IrType::U64 => insts.i64_const(n),
        IrType::U8 | IrType::U128 | IrType::U256 | IrType::Int | IrType::Bool => {
            insts.i32_const(n as i32)
        }
    };
    insts.local_set(dst.0);
}

pub(super) fn emit_istring_const(
    insts: &mut InstructionSink<'_>,
    dst: Value,
    s: &str,
    strs: &StringPool<'_>,
) -> Result<()> {
    let off = strs
        .get(s)
        .ok_or_else(|| anyhow::anyhow!("missing string offset for literal"))?;
    insts.i32_const(*off as i32);
    insts.local_set(dst.0);
    Ok(())
}

pub(super) fn emit_alloc(insts: &mut InstructionSink<'_>, dst: Value, size: u32, align: u32) {
    insts.global_get(HEAP_PTR_GLOBAL);
    insts.local_set(dst.0);
    emit_alignment(insts, dst, align);

    insts.local_get(dst.0);
    insts.i32_const(size as i32);
    insts.i32_add();
    emit_alloc_oom_check(insts, dst);

    insts.local_get(dst.0);
    insts.i32_const(size as i32);
    insts.i32_add();
    insts.global_set(HEAP_PTR_GLOBAL);
}

pub(super) fn emit_alloc_dyn(insts: &mut InstructionSink<'_>, dst: Value, size: Value, align: u32) {
    insts.global_get(HEAP_PTR_GLOBAL);
    insts.local_set(dst.0);
    emit_alignment(insts, dst, align);

    insts.local_get(dst.0);
    insts.local_get(size.0);
    insts.i32_add();
    emit_alloc_oom_check(insts, dst);

    insts.local_get(dst.0);
    insts.local_get(size.0);
    insts.i32_add();
    insts.global_set(HEAP_PTR_GLOBAL);
}

pub(super) fn emit_load(
    insts: &mut InstructionSink<'_>,
    dst: Value,
    ptr: Value,
    offset: u32,
    ty: IrType,
) {
    insts.local_get(ptr.0);
    match ty {
        IrType::U64 => insts.i64_load(MemArg {
            align: 3,
            offset: offset.into(),
            memory_index: 0,
        }),
        IrType::U8 => insts.i32_load8_u(MemArg {
            align: 0,
            offset: offset.into(),
            memory_index: 0,
        }),
        IrType::U128 | IrType::U256 | IrType::Int | IrType::Bool => insts.i32_load(MemArg {
            align: 2,
            offset: offset.into(),
            memory_index: 0,
        }),
    };
    insts.local_set(dst.0);
}

pub(super) fn emit_store(
    insts: &mut InstructionSink<'_>,
    ptr: Value,
    src: Value,
    offset: u32,
    ty: IrType,
) {
    insts.local_get(ptr.0);
    insts.local_get(src.0);
    match ty {
        IrType::U64 => insts.i64_store(MemArg {
            align: 3,
            offset: offset.into(),
            memory_index: 0,
        }),
        IrType::U8 => insts.i32_store8(MemArg {
            align: 0,
            offset: offset.into(),
            memory_index: 0,
        }),
        IrType::U128 | IrType::U256 | IrType::Int | IrType::Bool => insts.i32_store(MemArg {
            align: 2,
            offset: offset.into(),
            memory_index: 0,
        }),
    };
}

fn emit_alignment(insts: &mut InstructionSink<'_>, dst: Value, align: u32) {
    if align <= 1 {
        return;
    }
    insts.local_get(dst.0);
    insts.i32_const((align as i32) - 1);
    insts.i32_add();
    insts.i32_const(-(align as i32));
    insts.i32_and();
    insts.local_set(dst.0);
}

fn emit_alloc_oom_check(insts: &mut InstructionSink<'_>, dst: Value) {
    insts.memory_size(0);
    insts.i32_const(65536);
    insts.i32_mul();
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        insts,
        TrapCode::AllocatorOom,
        TrapOperand::local(dst.0),
        TrapOperand::zero(),
        0,
    );
    insts.end();
}
