use anyhow::Result;
use clg_ir::Function as IrFunction;
use wasm_encoder::{BlockType, Function, MemArg, ValType};

use super::shared::{emit_memory_limit, emit_set_data_ptr, emit_validate_len_prefixed_ptr};

pub fn encode_intrinsic_str_contains(_f: &IrFunction) -> Result<Function> {
    // Params: value: i32, needle: i32
    // Locals:
    // len_value(2), data_value(3), len_needle(4), data_needle(5),
    // found(6), i(7), j(8), limit(9), memory_limit(10), end(11),
    // pa(12), pb(13), matched(14), pos(15)
    let locals: Vec<(u32, ValType)> = vec![(14, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    emit_memory_limit(&mut insts, 10);

    emit_validate_len_prefixed_ptr(&mut insts, 0, 2, 11, 10);
    emit_set_data_ptr(&mut insts, 0, 3);

    emit_validate_len_prefixed_ptr(&mut insts, 1, 4, 11, 10);
    emit_set_data_ptr(&mut insts, 1, 5);

    // Empty needle is always contained.
    insts.local_get(4);
    insts.i32_eqz();
    insts.if_(BlockType::Result(ValType::I32));
    insts.i32_const(1);
    insts.else_();

    // If needle is longer than value, it's never contained.
    insts.local_get(4);
    insts.local_get(2);
    insts.i32_gt_u();
    insts.if_(BlockType::Result(ValType::I32));
    insts.i32_const(0);
    insts.else_();

    // limit = len_value - len_needle
    insts.local_get(2);
    insts.local_get(4);
    insts.i32_sub();
    insts.local_set(9);

    // found = false; i = 0
    insts.i32_const(0);
    insts.local_set(6);
    insts.i32_const(0);
    insts.local_set(7);

    insts.block(BlockType::Empty);
    insts.loop_(BlockType::Empty);
    // stop when i > limit
    insts.local_get(7);
    insts.local_get(9);
    insts.i32_gt_u();
    insts.br_if(1);

    // j = 0; matched = true
    insts.i32_const(0);
    insts.local_set(8);
    insts.i32_const(1);
    insts.local_set(14);

    insts.block(BlockType::Empty);
    insts.loop_(BlockType::Empty);
    // done comparing this window when j == len_needle
    insts.local_get(8);
    insts.local_get(4);
    insts.i32_eq();
    insts.br_if(1);

    // pos = i + j
    insts.local_get(7);
    insts.local_get(8);
    insts.i32_add();
    insts.local_set(15);

    // pa = data_value + pos; pb = data_needle + j
    insts.local_get(3);
    insts.local_get(15);
    insts.i32_add();
    insts.local_set(12);
    insts.local_get(5);
    insts.local_get(8);
    insts.i32_add();
    insts.local_set(13);

    insts.local_get(12);
    insts.i32_load8_u(MemArg {
        align: 0,
        offset: 0,
        memory_index: 0,
    });
    insts.local_get(13);
    insts.i32_load8_u(MemArg {
        align: 0,
        offset: 0,
        memory_index: 0,
    });
    insts.i32_ne();
    insts.if_(BlockType::Empty);
    insts.i32_const(0);
    insts.local_set(14);
    insts.br(2);
    insts.end();

    insts.local_get(8);
    insts.i32_const(1);
    insts.i32_add();
    insts.local_set(8);
    insts.br(0);
    insts.end();
    insts.end();

    // If matched remained true, we found the needle.
    insts.local_get(14);
    insts.if_(BlockType::Empty);
    insts.i32_const(1);
    insts.local_set(6);
    insts.br(2);
    insts.end();

    // Continue with next start index.
    insts.local_get(7);
    insts.i32_const(1);
    insts.i32_add();
    insts.local_set(7);
    insts.br(0);
    insts.end();
    insts.end();

    insts.local_get(6);
    insts.end();
    insts.end();

    insts.end();
    Ok(fenc)
}
