use clg_ir::TrapCode;
use wasm_encoder::{BlockType, InstructionSink, MemArg};

use crate::intrinsics::runtime::{emit_runtime_trap, TrapOperand};

const WASM_PAGE_BYTES: i32 = 65536;

pub(super) fn emit_memory_limit(insts: &mut InstructionSink<'_>, limit_local: u32) {
    insts.memory_size(0);
    insts.i32_const(WASM_PAGE_BYTES);
    insts.i32_mul();
    insts.local_set(limit_local);
}

pub(super) fn emit_validate_len_prefixed_ptr(
    insts: &mut InstructionSink<'_>,
    ptr_local: u32,
    len_local: u32,
    end_local: u32,
    limit_local: u32,
) {
    insts.local_get(ptr_local);
    insts.i32_const(3);
    insts.i32_and();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(ptr_local),
        TrapOperand::local(ptr_local),
        0,
    );
    insts.end();

    insts.local_get(ptr_local);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(limit_local);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(ptr_local),
        TrapOperand::zero(),
        0,
    );
    insts.end();

    insts.local_get(ptr_local);
    insts.i32_load(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });
    insts.local_set(len_local);

    insts.local_get(ptr_local);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(len_local);
    insts.i32_add();
    insts.local_set(end_local);

    insts.local_get(end_local);
    insts.local_get(limit_local);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(ptr_local),
        TrapOperand::local(end_local),
        0,
    );
    insts.end();
}

pub(super) fn emit_set_data_ptr(insts: &mut InstructionSink<'_>, ptr_local: u32, data_local: u32) {
    insts.local_get(ptr_local);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_set(data_local);
}
