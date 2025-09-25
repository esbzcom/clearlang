use anyhow::Result;
use clg_ir::{Function as IrFunction, TrapCode};
use wasm_encoder::{BlockType, Function, MemArg, ValType};

use super::runtime::{emit_runtime_trap, TrapOperand};
use crate::ir::HEAP_PTR_GLOBAL;

pub fn encode_intrinsic_str_len(_f: &IrFunction) -> Result<Function> {
    // Locals: limit(1), len(2), end(3)
    let locals: Vec<(u32, ValType)> = vec![(3, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    // memory limit in bytes
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

    insts.local_get(2);
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_str_eq(_f: &IrFunction) -> Result<Function> {
    // Locals: len(2), pa(3), pb(4), res(5), limit(6), len_b(7), end(8)
    let locals: Vec<(u32, ValType)> = vec![(7, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    // memory limit in bytes
    insts.memory_size(0);
    insts.i32_const(65536);
    insts.i32_mul();
    insts.local_set(6);

    // Validate pointer a
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

    insts.local_get(0);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(6);
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

    insts.local_get(0);
    insts.i32_load(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });
    insts.local_set(2);

    insts.local_get(0);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(2);
    insts.i32_add();
    insts.local_set(8);

    insts.local_get(8);
    insts.local_get(6);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(0),
        TrapOperand::local(8),
        0,
    );
    insts.end();

    insts.local_get(0);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_set(3);

    // Validate pointer b
    insts.local_get(1);
    insts.i32_const(3);
    insts.i32_and();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(1),
        TrapOperand::local(1),
        0,
    );
    insts.end();

    insts.local_get(1);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(6);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(1),
        TrapOperand::zero(),
        0,
    );
    insts.end();

    insts.local_get(1);
    insts.i32_load(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });
    insts.local_set(7);

    insts.local_get(1);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(7);
    insts.i32_add();
    insts.local_set(8);

    insts.local_get(8);
    insts.local_get(6);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(1),
        TrapOperand::local(8),
        0,
    );
    insts.end();

    insts.local_get(1);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_set(4);

    // Compare lengths
    insts.local_get(2);
    insts.local_get(7);
    insts.i32_ne();
    insts.if_(BlockType::Result(ValType::I32));
    insts.i32_const(0);
    insts.else_();

    // res = 1
    insts.i32_const(1);
    insts.local_set(5);

    // block { loop { ... } }
    insts.block(BlockType::Empty);
    insts.loop_(BlockType::Empty);
    // if (len == 0) break block;
    insts.local_get(2);
    insts.i32_eqz();
    insts.br_if(1);

    // if (*pa != *pb) { res = 0; break block; }
    insts.local_get(3);
    insts.i32_load8_u(MemArg {
        align: 0,
        offset: 0,
        memory_index: 0,
    });
    insts.local_get(4);
    insts.i32_load8_u(MemArg {
        align: 0,
        offset: 0,
        memory_index: 0,
    });
    insts.i32_ne();
    insts.if_(BlockType::Empty);
    insts.i32_const(0);
    insts.local_set(5);
    insts.br(2);
    insts.end();

    // pa++; pb++; len--;
    insts.local_get(3);
    insts.i32_const(1);
    insts.i32_add();
    insts.local_set(3);
    insts.local_get(4);
    insts.i32_const(1);
    insts.i32_add();
    insts.local_set(4);
    insts.local_get(2);
    insts.i32_const(1);
    insts.i32_sub();
    insts.local_set(2);

    insts.br(0);
    insts.end();
    insts.end();

    insts.local_get(5);
    insts.end();
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_str_concat(_f: &IrFunction) -> Result<Function> {
    // Params: a: i32, b: i32; Return: i32 (ptr)
    // Locals: len_a(2), len_b(3), total(4), dest(5), pa(6), pb(7), end(8), limit(9)
    let locals: Vec<(u32, ValType)> = vec![(8, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    // limit = memory.size * 65536
    insts.memory_size(0);
    insts.i32_const(65536);
    insts.i32_mul();
    insts.local_set(9);

    // Validate first operand string
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

    insts.local_get(0);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(9);
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

    insts.local_get(0);
    insts.i32_load(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });
    insts.local_set(2);

    insts.local_get(0);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(2);
    insts.i32_add();
    insts.local_set(8);

    insts.local_get(8);
    insts.local_get(9);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(0),
        TrapOperand::local(8),
        0,
    );
    insts.end();

    insts.local_get(0);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_set(6);

    // Validate second operand string
    insts.local_get(1);
    insts.i32_const(3);
    insts.i32_and();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(1),
        TrapOperand::local(1),
        0,
    );
    insts.end();

    insts.local_get(1);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(9);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(1),
        TrapOperand::zero(),
        0,
    );
    insts.end();

    insts.local_get(1);
    insts.i32_load(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });
    insts.local_set(3);

    insts.local_get(1);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(3);
    insts.i32_add();
    insts.local_set(8);

    insts.local_get(8);
    insts.local_get(9);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(1),
        TrapOperand::local(8),
        0,
    );
    insts.end();

    insts.local_get(1);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_set(7);

    // total = len_a + len_b
    insts.local_get(2);
    insts.local_get(3);
    insts.i32_add();
    insts.local_set(4);

    // dest = heap_ptr
    insts.global_get(HEAP_PTR_GLOBAL);
    insts.local_set(5);

    // end pointer after write: dest + 4 + total
    insts.local_get(5);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(4);
    insts.i32_add();
    insts.local_set(8);

    // ensure allocation stays in bounds before writing
    insts.local_get(8);
    insts.local_get(9);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::AllocatorOom,
        TrapOperand::local(5),
        TrapOperand::local(8),
        0,
    );
    insts.end();

    // store header
    insts.local_get(5);
    insts.local_get(4);
    insts.i32_store(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });

    // copy first string
    insts.local_get(5);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(6);
    insts.local_get(2);
    insts.memory_copy(0, 0);

    // copy second string
    insts.local_get(5);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_get(2);
    insts.i32_add();
    insts.local_get(7);
    insts.local_get(3);
    insts.memory_copy(0, 0);

    // heap_ptr = align4(end)
    insts.local_get(8);
    insts.i32_const(3);
    insts.i32_add();
    insts.i32_const(-4);
    insts.i32_and();
    insts.global_set(HEAP_PTR_GLOBAL);

    // return dest
    insts.local_get(5);
    insts.end();
    Ok(fenc)
}
