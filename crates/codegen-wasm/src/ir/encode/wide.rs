use anyhow::Result;
use clg_ir::{TrapCode, Value, VariantKind};
use wasm_encoder::{BlockType, InstructionSink, MemArg};

use crate::intrinsics::runtime::{emit_runtime_trap, TrapOperand};

use super::super::HEAP_PTR_GLOBAL;

pub(super) fn emit_variant_init(
    insts: &mut InstructionSink<'_>,
    dst: Value,
    tag: Value,
    payload_lo: Value,
    payload_hi: Value,
) {
    emit_aligned_bump_alloc(insts, dst, 16, 16);

    insts.local_get(dst.0);
    insts.local_get(tag.0);
    insts.i32_store(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });

    insts.local_get(dst.0);
    insts.local_get(payload_lo.0);
    insts.i32_store(MemArg {
        align: 2,
        offset: 4,
        memory_index: 0,
    });

    insts.local_get(dst.0);
    insts.local_get(payload_hi.0);
    insts.i32_store(MemArg {
        align: 2,
        offset: 8,
        memory_index: 0,
    });

    insts.local_get(dst.0);
    insts.i32_const(0);
    insts.i32_store(MemArg {
        align: 2,
        offset: 12,
        memory_index: 0,
    });
}

pub(super) fn emit_variant_load_tag(
    insts: &mut InstructionSink<'_>,
    dst: Value,
    variant: Value,
    kind: &VariantKind,
) {
    insts.local_get(variant.0);
    insts.i32_load(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });
    insts.local_set(dst.0);

    insts.local_get(dst.0);
    insts.i32_const(kind.max_tag() as i32);
    insts.i32_ge_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        insts,
        TrapCode::InvalidVariantTag,
        TrapOperand::local(dst.0),
        TrapOperand::zero(),
        kind.as_i32(),
    );
    insts.end();
}

pub(super) fn emit_variant_load_payload(
    insts: &mut InstructionSink<'_>,
    dst: Value,
    variant: Value,
    offset: u64,
) {
    insts.local_get(variant.0);
    insts.i32_load(MemArg {
        align: 2,
        offset,
        memory_index: 0,
    });
    insts.local_set(dst.0);
}

pub(super) fn emit_u128_init(
    insts: &mut InstructionSink<'_>,
    dst: Value,
    limb_lo: Value,
    limb_hi: Value,
) {
    emit_aligned_bump_alloc(insts, dst, 16, 16);

    insts.local_get(dst.0);
    insts.local_get(limb_lo.0);
    insts.i64_store(MemArg {
        align: 3,
        offset: 0,
        memory_index: 0,
    });
    insts.local_get(dst.0);
    insts.local_get(limb_hi.0);
    insts.i64_store(MemArg {
        align: 3,
        offset: 8,
        memory_index: 0,
    });
}

pub(super) fn emit_u128_load_limb(
    insts: &mut InstructionSink<'_>,
    dst: Value,
    value: Value,
    limb: u8,
) -> Result<()> {
    let offset = match limb {
        0 => 0,
        1 => 8,
        _ => return Err(anyhow::anyhow!("invalid U128 limb {}", limb)),
    };

    insts.local_get(value.0);
    insts.i64_load(MemArg {
        align: 3,
        offset,
        memory_index: 0,
    });
    insts.local_set(dst.0);
    Ok(())
}

pub(super) fn emit_u256_init(
    insts: &mut InstructionSink<'_>,
    dst: Value,
    limb0: Value,
    limb1: Value,
    limb2: Value,
    limb3: Value,
) {
    emit_aligned_bump_alloc(insts, dst, 32, 32);

    insts.local_get(dst.0);
    insts.local_get(limb0.0);
    insts.i64_store(MemArg {
        align: 3,
        offset: 0,
        memory_index: 0,
    });
    insts.local_get(dst.0);
    insts.local_get(limb1.0);
    insts.i64_store(MemArg {
        align: 3,
        offset: 8,
        memory_index: 0,
    });
    insts.local_get(dst.0);
    insts.local_get(limb2.0);
    insts.i64_store(MemArg {
        align: 3,
        offset: 16,
        memory_index: 0,
    });
    insts.local_get(dst.0);
    insts.local_get(limb3.0);
    insts.i64_store(MemArg {
        align: 3,
        offset: 24,
        memory_index: 0,
    });
}

pub(super) fn emit_u256_load_limb(
    insts: &mut InstructionSink<'_>,
    dst: Value,
    value: Value,
    limb: u8,
) -> Result<()> {
    let offset = match limb {
        0 => 0,
        1 => 8,
        2 => 16,
        3 => 24,
        _ => return Err(anyhow::anyhow!("invalid U256 limb {}", limb)),
    };

    insts.local_get(value.0);
    insts.i64_load(MemArg {
        align: 3,
        offset,
        memory_index: 0,
    });
    insts.local_set(dst.0);
    Ok(())
}

fn emit_aligned_bump_alloc(insts: &mut InstructionSink<'_>, dst: Value, size: i32, align: i32) {
    insts.global_get(HEAP_PTR_GLOBAL);
    insts.local_set(dst.0);

    insts.local_get(dst.0);
    insts.i32_const(size);
    insts.i32_add();
    insts.i32_const(align - 1);
    insts.i32_add();
    insts.i32_const(-align);
    insts.i32_and();
    emit_alloc_oom_check(insts, dst);

    insts.local_get(dst.0);
    insts.i32_const(size);
    insts.i32_add();
    insts.i32_const(align - 1);
    insts.i32_add();
    insts.i32_const(-align);
    insts.i32_and();
    insts.global_set(HEAP_PTR_GLOBAL);
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
