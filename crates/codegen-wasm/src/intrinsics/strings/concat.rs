use anyhow::Result;
use clg_ir::{Function as IrFunction, TrapCode};
use wasm_encoder::{BlockType, Function, MemArg, ValType};

use super::shared::{emit_memory_limit, emit_set_data_ptr, emit_validate_len_prefixed_ptr};
use crate::intrinsics::runtime::{emit_runtime_trap, TrapOperand};
use crate::ir::HEAP_PTR_GLOBAL;

pub fn encode_intrinsic_str_concat(_f: &IrFunction) -> Result<Function> {
    // Params: a: i32, b: i32; Return: i32 (ptr)
    // Locals: len_a(2), len_b(3), total(4), dest(5), pa(6), pb(7), end(8), limit(9)
    let locals: Vec<(u32, ValType)> = vec![(8, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    emit_memory_limit(&mut insts, 9);

    emit_validate_len_prefixed_ptr(&mut insts, 0, 2, 8, 9);
    emit_set_data_ptr(&mut insts, 0, 6);

    emit_validate_len_prefixed_ptr(&mut insts, 1, 3, 8, 9);
    emit_set_data_ptr(&mut insts, 1, 7);

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
