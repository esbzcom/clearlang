use anyhow::Result;
use clg_ir::Function as IrFunction;
use wasm_encoder::{Function, ValType};

use super::shared::{emit_memory_limit, emit_validate_len_prefixed_ptr};

pub fn encode_intrinsic_str_len(_f: &IrFunction) -> Result<Function> {
    // Locals: limit(1), len(2), end(3)
    let locals: Vec<(u32, ValType)> = vec![(3, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    emit_memory_limit(&mut insts, 1);
    emit_validate_len_prefixed_ptr(&mut insts, 0, 2, 3, 1);

    insts.local_get(2);
    insts.end();
    Ok(fenc)
}
