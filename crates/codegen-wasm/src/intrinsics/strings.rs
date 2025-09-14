use anyhow::Result;
use wasm_encoder::{Function, ValType, MemArg, BlockType};
use lumi_ir::Function as IrFunction;

pub fn encode_intrinsic_str_len(_f: &IrFunction) -> Result<Function> {
    let locals: Vec<(u32, ValType)> = Vec::new();
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();
    insts.local_get(0);
    insts.i32_load(MemArg { align: 2, offset: 0, memory_index: 0 });
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_str_eq(_f: &IrFunction) -> Result<Function> {
    // For now use pointer equality (correct for pooled literals)
    let locals: Vec<(u32, ValType)> = Vec::new();
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();
    insts.local_get(0);
    insts.local_get(1);
    insts.i32_eq();
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_str_concat(_f: &IrFunction) -> Result<Function> {
    // Params: a: i32, b: i32; Return: i32 (ptr)
    let locals: Vec<(u32, ValType)> = vec![(7, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();
    // len_a = load32(a); len_b = load32(b)
    insts.local_get(0); insts.i32_load(MemArg { align: 2, offset: 0, memory_index: 0 }); insts.local_set(2);
    insts.local_get(1); insts.i32_load(MemArg { align: 2, offset: 0, memory_index: 0 }); insts.local_set(3);
    // total = len_a + len_b
    insts.local_get(2); insts.local_get(3); insts.i32_add(); insts.local_set(4);
    // dest = heap_ptr (global 0)
    insts.global_get(0); insts.local_set(7);
    // store header: *(dest) = total
    insts.local_get(7); insts.local_get(4); insts.i32_store(MemArg { align: 2, offset: 0, memory_index: 0 });
    // pa = a + 4; pb = b + 4
    insts.local_get(0); insts.i32_const(4); insts.i32_add(); insts.local_set(5);
    insts.local_get(1); insts.i32_const(4); insts.i32_add(); insts.local_set(6);
    // copy first: for i in 0..len_a: *(dest+4+i) = *(pa+i)
    insts.i32_const(0); insts.local_set(8);
    insts.block(BlockType::Empty);
    insts.loop_(BlockType::Empty);
      insts.local_get(8); insts.local_get(2); insts.i32_ge_u();
      insts.br_if(1);
      // store byte
      insts.local_get(7); insts.i32_const(4); insts.i32_add(); insts.local_get(8); insts.i32_add();
      insts.local_get(5); insts.local_get(8); insts.i32_add();
      insts.i32_load8_u(MemArg { align: 0, offset: 0, memory_index: 0 });
      insts.i32_store8(MemArg { align: 0, offset: 0, memory_index: 0 });
      // i++
      insts.local_get(8); insts.i32_const(1); insts.i32_add(); insts.local_set(8);
      insts.br(0);
    insts.end(); // loop
    insts.end(); // block
    // copy second: for i in 0..len_b: *(dest+4+len_a+i) = *(pb+i)
    insts.i32_const(0); insts.local_set(8);
    insts.block(BlockType::Empty);
    insts.loop_(BlockType::Empty);
      insts.local_get(8); insts.local_get(3); insts.i32_ge_u();
      insts.br_if(1);
      // store byte
      insts.local_get(7); insts.i32_const(4); insts.i32_add(); insts.local_get(2); insts.i32_add(); insts.local_get(8); insts.i32_add();
      insts.local_get(6); insts.local_get(8); insts.i32_add();
      insts.i32_load8_u(MemArg { align: 0, offset: 0, memory_index: 0 });
      insts.i32_store8(MemArg { align: 0, offset: 0, memory_index: 0 });
      // i++
      insts.local_get(8); insts.i32_const(1); insts.i32_add(); insts.local_set(8);
      insts.br(0);
    insts.end(); // loop
    insts.end(); // block
    // heap_ptr = align4(dest + 4 + total)
    insts.local_get(7); insts.i32_const(4); insts.i32_add(); insts.local_get(4); insts.i32_add();
    insts.i32_const(3); insts.i32_add();
    insts.i32_const(-4); insts.i32_and();
    insts.global_set(0);
    // return dest
    insts.local_get(7);
    insts.end();
    Ok(fenc)
}
