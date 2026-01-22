use anyhow::Result;
use clg_ir::{Function as IrFunction, TrapCode};
use wasm_encoder::{BlockType, Function, MemArg, ValType};

use super::runtime::{emit_runtime_trap, TrapOperand};
use crate::ir::HEAP_PTR_GLOBAL;

pub fn encode_intrinsic_wasi_print(
    _f: &IrFunction,
    fd_write_index: u32,
) -> Result<Function> {
    // Params: buf_ptr (i32). Return: errno (i32).
    // Locals: len(1), data_ptr(2), iovec_ptr(3), nwritten_ptr(4),
    // data_end(5), limit(6), iovec_end(7)
    let locals: Vec<(u32, ValType)> = vec![(7, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    // limit = memory.size * 65536
    insts.memory_size(0);
    insts.i32_const(65536);
    insts.i32_mul();
    insts.local_set(6);

    // len = *(buf_ptr)
    insts.local_get(0);
    insts.i32_load(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });
    insts.local_set(1);

    // data_ptr = buf_ptr + 4
    insts.local_get(0);
    insts.i32_const(4);
    insts.i32_add();
    insts.local_set(2);

    // data_end = data_ptr + len
    insts.local_get(2);
    insts.local_get(1);
    insts.i32_add();
    insts.local_set(5);

    // data_end must be within memory
    insts.local_get(5);
    insts.local_get(6);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::InvalidUtf8,
        TrapOperand::local(0),
        TrapOperand::local(5),
        0,
    );
    insts.end();

    // iovec_ptr = heap_ptr
    insts.global_get(HEAP_PTR_GLOBAL);
    insts.local_set(3);

    // iovec_end = iovec_ptr + 12 (iovec + nwritten)
    insts.local_get(3);
    insts.i32_const(12);
    insts.i32_add();
    insts.local_set(7);

    insts.local_get(7);
    insts.local_get(6);
    insts.i32_gt_u();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        &mut insts,
        TrapCode::AllocatorOom,
        TrapOperand::local(3),
        TrapOperand::local(7),
        0,
    );
    insts.end();

    // store iovec { data_ptr, len }
    insts.local_get(3);
    insts.local_get(2);
    insts.i32_store(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });

    insts.local_get(3);
    insts.local_get(1);
    insts.i32_store(MemArg {
        align: 2,
        offset: 4,
        memory_index: 0,
    });

    // nwritten_ptr = iovec_ptr + 8
    insts.local_get(3);
    insts.i32_const(8);
    insts.i32_add();
    insts.local_set(4);

    insts.local_get(4);
    insts.i32_const(0);
    insts.i32_store(MemArg {
        align: 2,
        offset: 0,
        memory_index: 0,
    });

    // heap_ptr = align4(iovec_end)
    insts.local_get(7);
    insts.i32_const(3);
    insts.i32_add();
    insts.i32_const(-4);
    insts.i32_and();
    insts.global_set(HEAP_PTR_GLOBAL);

    // fd_write(fd=1, iovec_ptr, 1, nwritten_ptr)
    insts.i32_const(1);
    insts.local_get(3);
    insts.i32_const(1);
    insts.local_get(4);
    insts.call(fd_write_index);

    insts.end();
    Ok(fenc)
}
