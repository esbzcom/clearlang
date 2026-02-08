use anyhow::Result;
use clg_ir::Function as IrFunction;
use wasm_encoder::{BlockType, Function, MemArg, ValType};

use super::shared::{emit_memory_limit, emit_set_data_ptr, emit_validate_len_prefixed_ptr};

pub fn encode_intrinsic_str_eq(_f: &IrFunction) -> Result<Function> {
    // Locals: len(2), pa(3), pb(4), res(5), limit(6), len_b(7), end(8)
    let locals: Vec<(u32, ValType)> = vec![(7, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    emit_memory_limit(&mut insts, 6);

    emit_validate_len_prefixed_ptr(&mut insts, 0, 2, 8, 6);
    emit_set_data_ptr(&mut insts, 0, 3);

    emit_validate_len_prefixed_ptr(&mut insts, 1, 7, 8, 6);
    emit_set_data_ptr(&mut insts, 1, 4);

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
