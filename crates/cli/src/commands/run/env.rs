use anyhow::Result;
use clg_ir::TrapCode;
use wasmtime as wt;

use super::runtime_error::set_runtime_error;
use super::wasm_state::{get_caller_global_i32, get_caller_memory, set_caller_global_i32};

pub(super) fn add_env_stubs(linker: &mut wt::Linker<wasmtime_wasi::WasiCtx>) -> Result<()> {
    linker.func_wrap(
        "clearlang_env",
        "env_time",
        |_caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>| -> i32 { 0 },
    )?;
    linker.func_wrap(
        "clearlang_env",
        "env_random",
        |mut caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>, len: i32| -> Result<i32> {
            if len < 0 {
                set_runtime_error(&mut caller, TrapCode::InvalidUtf8);
                return Ok(0);
            }
            let len_u32 = len as u32;
            let heap_ptr = get_caller_global_i32(&mut caller, "__clg_heap_ptr")? as u32;
            let size = 4u32.saturating_add(len_u32);
            let next = (heap_ptr + size + 3) & !3;

            let memory = get_caller_memory(&mut caller)?;
            let mem_size = memory.data_size(&caller);
            if next as usize > mem_size {
                set_runtime_error(&mut caller, TrapCode::AllocatorOom);
                return Ok(0);
            }

            let data = memory.data_mut(&mut caller);
            let start = heap_ptr as usize;
            data[start..start + 4].copy_from_slice(&len_u32.to_le_bytes());
            if len_u32 > 0 {
                data[start + 4..start + 4 + len_u32 as usize].fill(0);
            }

            set_caller_global_i32(&mut caller, "__clg_heap_ptr", next as i32)?;
            Ok(heap_ptr as i32)
        },
    )?;
    linker.func_wrap(
        "clearlang_env",
        "env_chain_id",
        |mut caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>| -> Result<i32> {
            let value = b"dev-chain";
            let len_u32 = value.len() as u32;
            let heap_ptr = get_caller_global_i32(&mut caller, "__clg_heap_ptr")? as u32;
            let size = 4u32.saturating_add(len_u32);
            let next = (heap_ptr + size + 3) & !3;

            let memory = get_caller_memory(&mut caller)?;
            let mem_size = memory.data_size(&caller);
            if next as usize > mem_size {
                set_runtime_error(&mut caller, TrapCode::AllocatorOom);
                return Ok(0);
            }

            let data = memory.data_mut(&mut caller);
            let start = heap_ptr as usize;
            data[start..start + 4].copy_from_slice(&len_u32.to_le_bytes());
            data[start + 4..start + 4 + value.len()].copy_from_slice(value);

            set_caller_global_i32(&mut caller, "__clg_heap_ptr", next as i32)?;
            Ok(heap_ptr as i32)
        },
    )?;
    Ok(())
}
