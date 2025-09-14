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
    // Robust byte-wise equality: compare lengths, then decrementing length while advancing pointers
    // Locals: len(2), pa(3), pb(4), res(5)
    let locals: Vec<(u32, ValType)> = vec![(4, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();
    // len_a = load32(a); len_b = load32(b)
    insts.local_get(0); insts.i32_load(MemArg { align: 2, offset: 0, memory_index: 0 });
    insts.local_get(1); insts.i32_load(MemArg { align: 2, offset: 0, memory_index: 0 });
    insts.i32_ne();
    insts.if_(BlockType::Result(ValType::I32));
      // lengths differ → false
      insts.i32_const(0);
    insts.else_();
      // len = len_a (we still have it on stack? No; reload)
      insts.local_get(0); insts.i32_load(MemArg { align: 2, offset: 0, memory_index: 0 }); insts.local_set(2);
      // pa = a + 4; pb = b + 4; res = 1
      insts.local_get(0); insts.i32_const(4); insts.i32_add(); insts.local_set(3);
      insts.local_get(1); insts.i32_const(4); insts.i32_add(); insts.local_set(4);
      insts.i32_const(1); insts.local_set(5);
      // block { loop { if (len == 0) break; if (*pa != *pb) { res = 0; break; } pa++; pb++; len--; } }
      insts.block(BlockType::Empty);
      insts.loop_(BlockType::Empty);
        // if (len == 0) break;
        insts.local_get(2); insts.i32_eqz(); insts.br_if(1);
        // if (load8(pa) != load8(pb)) { res = 0; break; }
        insts.local_get(3);
        insts.i32_load8_u(MemArg { align: 0, offset: 0, memory_index: 0 });
        insts.local_get(4);
        insts.i32_load8_u(MemArg { align: 0, offset: 0, memory_index: 0 });
        insts.i32_ne();
        insts.if_(BlockType::Empty);
          insts.i32_const(0); insts.local_set(5);
          insts.br(1);
        insts.end();
        // pa++; pb++; len--
        insts.local_get(3); insts.i32_const(1); insts.i32_add(); insts.local_set(3);
        insts.local_get(4); insts.i32_const(1); insts.i32_add(); insts.local_set(4);
        insts.local_get(2); insts.i32_const(1); insts.i32_sub(); insts.local_set(2);
        insts.br(0);
      insts.end(); // loop
      insts.end(); // block
      insts.local_get(5);
    insts.end(); // if
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_str_concat(_f: &IrFunction) -> Result<Function> {
    // Params: a: i32, b: i32; Return: i32 (ptr)
    // Locals: len_a(2), len_b(3), total(4), dest(5), pa(6), pb(7)
    let locals: Vec<(u32, ValType)> = vec![(6, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();
    // len_a = load32(a); len_b = load32(b)
    insts.local_get(0); insts.i32_load(MemArg { align: 2, offset: 0, memory_index: 0 }); insts.local_set(2);
    insts.local_get(1); insts.i32_load(MemArg { align: 2, offset: 0, memory_index: 0 }); insts.local_set(3);
    // total = len_a + len_b
    insts.local_get(2); insts.local_get(3); insts.i32_add(); insts.local_set(4);
    // dest = heap_ptr (global 0)
    insts.global_get(0); insts.local_set(5);
    // store header: *(dest) = total
    insts.local_get(5); insts.local_get(4); insts.i32_store(MemArg { align: 2, offset: 0, memory_index: 0 });
    // pa = a + 4; pb = b + 4
    insts.local_get(0); insts.i32_const(4); insts.i32_add(); insts.local_set(6);
    insts.local_get(1); insts.i32_const(4); insts.i32_add(); insts.local_set(7);
    // copy first: memory.copy(dest+4, pa, len_a)
    insts.local_get(5); insts.i32_const(4); insts.i32_add(); // dest
    insts.local_get(6); // src
    insts.local_get(2); // len
    insts.memory_copy(0, 0);
    // copy second: memory.copy(dest+4+len_a, pb, len_b)
    insts.local_get(5); insts.i32_const(4); insts.i32_add(); insts.local_get(2); insts.i32_add();
    insts.local_get(7);
    insts.local_get(3);
    insts.memory_copy(0, 0);
    // heap_ptr = align4(dest + 4 + total)
    insts.local_get(5); insts.i32_const(4); insts.i32_add(); insts.local_get(4); insts.i32_add();
    insts.i32_const(3); insts.i32_add();
    insts.i32_const(-4); insts.i32_and();
    insts.global_set(0);
    // return dest
    insts.local_get(5);
    insts.end();
    Ok(fenc)
}
