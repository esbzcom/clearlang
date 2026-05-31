use anyhow::Result;
use clg_ir::TrapCode;
use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex};
use wasmtime as wt;

use super::runtime_error::set_runtime_error;
use super::wasm_state::{get_caller_global_i32, get_caller_memory, set_caller_global_i32};

static HOST_STORAGE: LazyLock<Mutex<BTreeMap<Vec<u8>, Vec<u8>>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

pub(super) fn add_host_stubs(linker: &mut wt::Linker<wasmtime_wasi::WasiCtx>) -> Result<()> {
    linker.func_wrap(
        "clearlang_host",
        "host_storage_contains",
        |mut caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>, key: i32| -> Result<i32> {
            let Some(key_bytes) = read_bytes_param(&mut caller, key) else {
                return Ok(0);
            };
            let store = HOST_STORAGE.lock().expect("host storage lock");
            Ok(if store.contains_key(&key_bytes) { 1 } else { 0 })
        },
    )?;
    linker.func_wrap(
        "clearlang_host",
        "host_storage_get",
        |mut caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>, key: i32| -> Result<i32> {
            let Some(key_bytes) = read_bytes_param(&mut caller, key) else {
                return Ok(-1);
            };
            let store = HOST_STORAGE.lock().expect("host storage lock");
            if let Some(value) = store.get(&key_bytes) {
                write_bytes(&mut caller, value)
            } else {
                Ok(-1)
            }
        },
    )?;
    linker.func_wrap(
        "clearlang_host",
        "host_storage_set",
        |mut caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>, key: i32, value: i32| -> Result<i32> {
            let Some(key_bytes) = read_bytes_param(&mut caller, key) else {
                return Ok(0);
            };
            let Some(value_bytes) = read_bytes_param(&mut caller, value) else {
                return Ok(0);
            };
            let mut store = HOST_STORAGE.lock().expect("host storage lock");
            let changed = store.get(&key_bytes) != Some(&value_bytes);
            store.insert(key_bytes, value_bytes);
            Ok(if changed { 1 } else { 0 })
        },
    )?;
    linker.func_wrap(
        "clearlang_host",
        "host_storage_delete",
        |mut caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>, key: i32| -> Result<i32> {
            let Some(key_bytes) = read_bytes_param(&mut caller, key) else {
                return Ok(0);
            };
            let mut store = HOST_STORAGE.lock().expect("host storage lock");
            Ok(if store.remove(&key_bytes).is_some() {
                1
            } else {
                0
            })
        },
    )?;
    for name in ["host_log_info", "host_log_warn", "host_log_error"] {
        linker.func_wrap(
            "clearlang_host",
            name,
            |mut caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>,
             _code: i32,
             message: i32|
             -> Result<i32> {
                let Some(_) = read_string_param(&mut caller, message) else {
                    return Ok(0);
                };
                Ok(1)
            },
        )?;
    }
    linker.func_wrap(
        "clearlang_host",
        "host_env_chain_id",
        |_caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>| -> i64 { 1337 },
    )?;
    linker.func_wrap(
        "clearlang_host",
        "host_env_caller",
        |mut caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>| -> Result<i32> {
            write_bytes(&mut caller, b"dev-caller")
        },
    )?;
    linker.func_wrap(
        "clearlang_host",
        "host_env_block_height",
        |_caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>| -> i64 { 1 },
    )?;
    linker.func_wrap(
        "clearlang_host",
        "host_env_timestamp",
        |_caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>| -> i64 { 1_700_000_000 },
    )?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum ReadBufferError {
    InvalidLayout,
    InvalidUtf8,
}

fn read_bytes<T>(caller: &mut wt::Caller<'_, T>, ptr: i32) -> Result<Vec<u8>, ReadBufferError> {
    if ptr < 0 || (ptr & 3) != 0 {
        return Err(ReadBufferError::InvalidLayout);
    }
    let memory = get_caller_memory(caller).map_err(|_| ReadBufferError::InvalidLayout)?;
    let data = memory.data(caller);
    let start = ptr as usize;
    if start + 4 > data.len() {
        return Err(ReadBufferError::InvalidLayout);
    }
    let header = data
        .get(start..start + 4)
        .ok_or(ReadBufferError::InvalidLayout)?;
    let len = u32::from_le_bytes(
        header
            .try_into()
            .map_err(|_| ReadBufferError::InvalidLayout)?,
    ) as usize;
    let end = start + 4 + len;
    if end > data.len() {
        return Err(ReadBufferError::InvalidLayout);
    }
    Ok(data[start + 4..end].to_vec())
}

fn read_string<T>(caller: &mut wt::Caller<'_, T>, ptr: i32) -> Result<String, ReadBufferError> {
    let bytes = read_bytes(caller, ptr)?;
    String::from_utf8(bytes).map_err(|_| ReadBufferError::InvalidUtf8)
}

fn read_bytes_param<T>(caller: &mut wt::Caller<'_, T>, ptr: i32) -> Option<Vec<u8>> {
    match read_bytes(caller, ptr) {
        Ok(value) => Some(value),
        Err(ReadBufferError::InvalidLayout) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            None
        }
        Err(ReadBufferError::InvalidUtf8) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            None
        }
    }
}

fn read_string_param<T>(caller: &mut wt::Caller<'_, T>, ptr: i32) -> Option<String> {
    match read_string(caller, ptr) {
        Ok(value) => Some(value),
        Err(ReadBufferError::InvalidLayout) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            None
        }
        Err(ReadBufferError::InvalidUtf8) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            None
        }
    }
}

fn write_bytes<T>(caller: &mut wt::Caller<'_, T>, bytes: &[u8]) -> Result<i32> {
    let len_u32 = bytes.len() as u32;
    let heap_ptr = get_caller_global_i32(caller, "__clg_heap_ptr")? as u32;
    let size = 4u32.saturating_add(len_u32);
    let next = (heap_ptr + size + 3) & !3;

    let memory = get_caller_memory(caller)?;
    let mem_size = memory.data_size(&caller);
    if next as usize > mem_size {
        set_runtime_error(caller, TrapCode::AllocatorOom);
        return Ok(0);
    }

    let data = memory.data_mut(&mut *caller);
    let start = heap_ptr as usize;
    data[start..start + 4].copy_from_slice(&len_u32.to_le_bytes());
    if len_u32 > 0 {
        data[start + 4..start + 4 + len_u32 as usize].copy_from_slice(bytes);
    }

    set_caller_global_i32(caller, "__clg_heap_ptr", next as i32)?;
    Ok(heap_ptr as i32)
}
