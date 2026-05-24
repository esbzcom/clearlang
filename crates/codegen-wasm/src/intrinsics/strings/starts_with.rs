use anyhow::Result;
use clg_ir::Function as IrFunction;
use wasm_encoder::{BlockType, Function, MemArg, ValType};

use super::shared::{emit_memory_limit, emit_set_data_ptr, emit_validate_len_prefixed_ptr};

pub fn encode_intrinsic_str_starts_with(_f: &IrFunction) -> Result<Function> {
    // Params: value: i32, prefix: i32
    // Locals:
    // len_value(2), data_value(3), len_prefix(4), data_prefix(5),
    // res(6), i(7), limit(8), end(9), pa(10), pb(11)
    let locals: Vec<(u32, ValType)> = vec![(10, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    emit_memory_limit(&mut insts, 8);

    emit_validate_len_prefixed_ptr(&mut insts, 0, 2, 9, 8);
    emit_set_data_ptr(&mut insts, 0, 3);

    emit_validate_len_prefixed_ptr(&mut insts, 1, 4, 9, 8);
    emit_set_data_ptr(&mut insts, 1, 5);

    // default res = true
    insts.i32_const(1);
    insts.local_set(6);

    // if prefix is longer than value => false
    insts.local_get(4);
    insts.local_get(2);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    insts.i32_const(0);
    insts.local_set(6);
    insts.else_();

    insts.i32_const(0);
    insts.local_set(7);

    insts.block(BlockType::Empty);
    insts.loop_(BlockType::Empty);
    // done when i == len_prefix
    insts.local_get(7);
    insts.local_get(4);
    insts.i32_eq();
    insts.br_if(1);

    insts.local_get(3);
    insts.local_get(7);
    insts.i32_add();
    insts.local_set(10);
    insts.local_get(5);
    insts.local_get(7);
    insts.i32_add();
    insts.local_set(11);

    insts.local_get(10);
    insts.i32_load8_u(MemArg {
        align: 0,
        offset: 0,
        memory_index: 0,
    });
    insts.local_get(11);
    insts.i32_load8_u(MemArg {
        align: 0,
        offset: 0,
        memory_index: 0,
    });
    insts.i32_ne();
    insts.if_(BlockType::Empty);
    insts.i32_const(0);
    insts.local_set(6);
    insts.br(2);
    insts.end();

    insts.local_get(7);
    insts.i32_const(1);
    insts.i32_add();
    insts.local_set(7);
    insts.br(0);
    insts.end();
    insts.end();

    insts.end();

    insts.local_get(6);
    insts.end();
    Ok(fenc)
}
