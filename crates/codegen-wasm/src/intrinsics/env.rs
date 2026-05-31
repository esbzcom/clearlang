use anyhow::Result;
use clg_ir::Function as IrFunction;
use wasm_encoder::Function;

pub fn encode_intrinsic_env_time(_f: &IrFunction, env_time_index: u32) -> Result<Function> {
    let mut fenc = Function::new(Vec::new());
    let mut insts = fenc.instructions();
    insts.call(env_time_index);
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_env_random(_f: &IrFunction, env_random_index: u32) -> Result<Function> {
    let mut fenc = Function::new(Vec::new());
    let mut insts = fenc.instructions();
    insts.local_get(0);
    insts.call(env_random_index);
    insts.end();
    Ok(fenc)
}

pub fn encode_intrinsic_env_chain_id(_f: &IrFunction, env_chain_id_index: u32) -> Result<Function> {
    let mut fenc = Function::new(Vec::new());
    let mut insts = fenc.instructions();
    insts.call(env_chain_id_index);
    insts.end();
    Ok(fenc)
}
