use anyhow::Result;
use clg_ir::{Function as IrFunction, TrapCode};
use wasm_encoder::{BlockType, Function, MemArg, ValType};

use super::runtime::{emit_runtime_trap, TrapOperand};
use crate::ir::HEAP_PTR_GLOBAL;

pub fn encode_intrinsic_u64_rotl(_f: &IrFunction) -> Result<Function> {
    let mut fenc = Function::new(Vec::new());
    let mut insts = fenc.instructions();
    insts.local_get(0);
    insts.local_get(1);
    insts.i64_rotl();
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_u64_rotr(_f: &IrFunction) -> Result<Function> {
    let mut fenc = Function::new(Vec::new());
    let mut insts = fenc.instructions();
    insts.local_get(0);
    insts.local_get(1);
    insts.i64_rotr();
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_u64_to_bytes_le(_f: &IrFunction) -> Result<Function> {
    // Params: value: i64, Return: i32 (Bytes pointer)
    // Locals: dest(1), end(2), limit(3)
    let locals: Vec<(u32, ValType)> = vec![(3, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    // limit = memory.size * 65536
    insts.memory_size(0);
    insts.i32_const(65536);
    insts.i32_mul();
    insts.local_set(3);

    // dest = heap_ptr
    insts.global_get(HEAP_PTR_GLOBAL);
    insts.local_set(1);

    // end = dest + 12 (4-byte len + 8 data bytes)
    insts.local_get(1);
    insts.i32_const(12);
    insts.i32_add();
    insts.local_set(2);

    // bounds check
    insts.local_get(2);
    insts.local_get(3);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::AllocatorOom,
        TrapOperand::local(1),
        TrapOperand::local(2),
        0,
    );
    insts.end();

    // store len = 8
    insts.local_get(1);
    insts.i32_const(8);
    insts.i32_store(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });

    // store value (little-endian)
    insts.local_get(1);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(0);
    insts.i64_store(MemArg {
        align: 0,
        offset: 0,
        memory_index: 0,
    });

    // heap_ptr = align4(end)
    insts.local_get(2);
    insts.i32_const(3);
    insts.i32_add();
    insts.i32_const(-4);
    insts.i32_and();
    insts.global_set(HEAP_PTR_GLOBAL);

    // return dest
    insts.local_get(1);
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_u64_to_bytes_be(_f: &IrFunction) -> Result<Function> {
    // Params: value: i64, Return: i32 (Bytes pointer)
    // Locals: dest(1), end(2), limit(3)
    let locals: Vec<(u32, ValType)> = vec![(3, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    // limit = memory.size * 65536
    insts.memory_size(0);
    insts.i32_const(65536);
    insts.i32_mul();
    insts.local_set(3);

    // dest = heap_ptr
    insts.global_get(HEAP_PTR_GLOBAL);
    insts.local_set(1);

    // end = dest + 12
    insts.local_get(1);
    insts.i32_const(12);
    insts.i32_add();
    insts.local_set(2);

    // bounds check
    insts.local_get(2);
    insts.local_get(3);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::AllocatorOom,
        TrapOperand::local(1),
        TrapOperand::local(2),
        0,
    );
    insts.end();

    // store len = 8
    insts.local_get(1);
    insts.i32_const(8);
    insts.i32_store(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });

    // store bytes big-endian
    for (idx, shift) in [56_i64, 48, 40, 32, 24, 16, 8, 0].iter().enumerate() {
        insts.local_get(1);
        insts.i32_const(4 + idx as i32);
        insts.i32_add();
        insts.local_get(0);
        insts.i64_const(*shift);
        insts.i64_shr_u();
        insts.i64_const(0xFF);
        insts.i64_and();
        insts.i32_wrap_i64();
        insts.i32_store8(MemArg {
            align: 0,
            offset: 0,
            memory_index: 0,
        });
    }

    // heap_ptr = align4(end)
    insts.local_get(2);
    insts.i32_const(3);
    insts.i32_add();
    insts.i32_const(-4);
    insts.i32_and();
    insts.global_set(HEAP_PTR_GLOBAL);

    // return dest
    insts.local_get(1);
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_u64_from_bytes_le(_f: &IrFunction) -> Result<Function> {
    // Params: bytes: i32, Return: i64
    // Locals: limit(1), len(2), end(3)
    let locals: Vec<(u32, ValType)> = vec![(3, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    // limit = memory.size * 65536
    insts.memory_size(0);
    insts.i32_const(65536);
    insts.i32_mul();
    insts.local_set(1);

    // pointer alignment check
    insts.local_get(0);
    insts.i32_const(3);
    insts.i32_and();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(0),
        TrapOperand::local(0),
        0,
    );
    insts.end();

    // header within bounds (ptr + 4 <= limit)
    insts.local_get(0);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(1);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(0),
        TrapOperand::zero(),
        0,
    );
    insts.end();

    // read length
    insts.local_get(0);
    insts.i32_load(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });
    insts.local_set(2);

    // compute end pointer ptr+4+len
    insts.local_get(0);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(2);
    insts.i32_add();
    insts.local_set(3);

    // ensure data fits in memory
    insts.local_get(3);
    insts.local_get(1);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(0),
        TrapOperand::local(3),
        0,
    );
    insts.end();

    // length must be exactly 8
    insts.local_get(2);
    insts.i32_const(8);
    insts.i32_ne();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(0),
        TrapOperand::local(3),
        0,
    );
    insts.end();

    // load value (little-endian)
    insts.local_get(0);
    insts.i32_const(4);
    insts.i32_add();
    insts.i64_load(MemArg {
        align: 0,
        offset: 0,
        memory_index: 0,
    });
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_u64_from_bytes_be(_f: &IrFunction) -> Result<Function> {
    // Params: bytes: i32, Return: i64
    // Locals: limit(1), len(2), end(3), value(4)
    let locals: Vec<(u32, ValType)> = vec![(3, ValType::I32), (1, ValType::I64)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    // limit = memory.size * 65536
    insts.memory_size(0);
    insts.i32_const(65536);
    insts.i32_mul();
    insts.local_set(1);

    // pointer alignment check
    insts.local_get(0);
    insts.i32_const(3);
    insts.i32_and();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(0),
        TrapOperand::local(0),
        0,
    );
    insts.end();

    // header within bounds (ptr + 4 <= limit)
    insts.local_get(0);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(1);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(0),
        TrapOperand::zero(),
        0,
    );
    insts.end();

    // read length
    insts.local_get(0);
    insts.i32_load(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });
    insts.local_set(2);

    // compute end pointer ptr+4+len
    insts.local_get(0);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(2);
    insts.i32_add();
    insts.local_set(3);

    // ensure data fits in memory
    insts.local_get(3);
    insts.local_get(1);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(0),
        TrapOperand::local(3),
        0,
    );
    insts.end();

    // length must be exactly 8
    insts.local_get(2);
    insts.i32_const(8);
    insts.i32_ne();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(0),
        TrapOperand::local(3),
        0,
    );
    insts.end();

    // value = 0
    insts.i64_const(0);
    insts.local_set(4);

    for idx in 0..8 {
        insts.local_get(4);
        insts.i64_const(8);
        insts.i64_shl();
        insts.local_get(0);
        insts.i32_const(4 + idx);
        insts.i32_add();
        insts.i32_load8_u(MemArg {
            align: 0,
            offset: 0,
            memory_index: 0,
        });
        insts.i64_extend_i32_u();
        insts.i64_or();
        insts.local_set(4);
    }

    insts.local_get(4);
    insts.end();
    Ok(fenc)
}
