use anyhow::Result;
use clg_ir::Function as IrFunction;
use wasm_encoder::Function;

pub fn encode_intrinsic_crypto_hash(_f: &IrFunction, idx: u32) -> Result<Function> {
    let mut fenc = Function::new(Vec::new());
    let mut insts = fenc.instructions();
    insts.local_get(0);
    insts.local_get(1);
    insts.call(idx);
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_crypto_hmac(_f: &IrFunction, idx: u32) -> Result<Function> {
    let mut fenc = Function::new(Vec::new());
    let mut insts = fenc.instructions();
    insts.local_get(0);
    insts.local_get(1);
    insts.local_get(2);
    insts.call(idx);
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_crypto_verify(_f: &IrFunction, idx: u32) -> Result<Function> {
    let mut fenc = Function::new(Vec::new());
    let mut insts = fenc.instructions();
    insts.local_get(0);
    insts.local_get(1);
    insts.local_get(2);
    insts.local_get(3);
    insts.call(idx);
    insts.end();
    Ok(fenc)
}
