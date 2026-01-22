use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use wasmtime as wt;
use wasmtime_wasi::sync::WasiCtxBuilder;

use super::helpers::{make_single_json_error, CommandError};
use crate::logging::{Logger, StageTimings};

pub fn run(file: PathBuf, invoke: String, json_errors: bool, logger: Logger) -> Result<()> {
    let mut timings = StageTimings::new();
    let engine = wt::Engine::new(&wt::Config::new())?;
    let module = {
        let _stage = timings.start(logger, "load_module");
        wt::Module::from_file(&engine, &file)
            .with_context(|| format!("loading {}", file.display()))?
    };
    let wasi = WasiCtxBuilder::new()
        .inherit_stdout()
        .inherit_stderr()
        .build();
    let mut store = wt::Store::new(&engine, wasi);
    let instance = {
        let _stage = timings.start(logger, "instantiate");
        let mut linker = wt::Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |cx| cx).context("linking WASI")?;
        add_env_stubs(&mut linker).context("linking env stubs")?;
        linker
            .instantiate(&mut store, &module)
            .context("instantiating module")?
    };
    let func = instance
        .get_typed_func::<(), i32>(&mut store, &invoke)
        .with_context(|| format!("export `{}` not found or wrong type", invoke))?;

    match func.call(&mut store, ()) {
        Ok(result) => {
            println!("{}", result);
            logger.summary(&timings);
            Ok(())
        }
        Err(trap) => {
            if let Some(diag) = extract_runtime_error(&instance, &mut store) {
                if json_errors {
                    let json = make_single_json_error(
                        diag.code,
                        "runtime",
                        diag.message.clone(),
                        Path::new(&file),
                        diag.start,
                        diag.end,
                        diag.detail.clone(),
                    );
                    return Err(CommandError::json(json).into());
                } else {
                    let span = if diag.start != 0 || diag.end != 0 {
                        format!(" at {}..{}", diag.start, diag.end)
                    } else {
                        String::new()
                    };
                    return Err(anyhow!("{}{} (code {})", diag.message, span, diag.code));
                }
            }
            if let Some(diag) = extract_wasmtime_limit_error(&trap) {
                if json_errors {
                    let json = make_single_json_error(
                        diag.code,
                        "runtime",
                        diag.message.clone(),
                        Path::new(&file),
                        0,
                        0,
                        None,
                    );
                    return Err(CommandError::json(json).into());
                } else {
                    return Err(anyhow!("{}", diag.message));
                }
            }
            Err(trap)
        }
    }
}

fn add_env_stubs(linker: &mut wt::Linker<wasmtime_wasi::WasiCtx>) -> Result<()> {
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
                return Err(anyhow!("env_random length must be non-negative"));
            }
            let len_u32 = len as u32;
            let heap_ptr = get_caller_global_i32(&mut caller, "__clg_heap_ptr")? as u32;
            let size = 4u32.saturating_add(len_u32);
            let next = (heap_ptr + size + 3) & !3;

            let memory = get_caller_memory(&mut caller)?;
            let mem_size = memory.data_size(&caller);
            if next as usize > mem_size {
                return Err(anyhow!("env_random out of memory"));
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
    Ok(())
}

fn get_caller_memory<T>(caller: &mut wt::Caller<'_, T>) -> Result<wt::Memory> {
    match caller.get_export("memory") {
        Some(wt::Extern::Memory(mem)) => Ok(mem),
        _ => Err(anyhow!("missing memory export")),
    }
}

fn get_caller_global_i32<T>(caller: &mut wt::Caller<'_, T>, name: &str) -> Result<i32> {
    let global = match caller.get_export(name) {
        Some(wt::Extern::Global(global)) => global,
        _ => return Err(anyhow!("missing global `{}`", name)),
    };
    global
        .get(&mut *caller)
        .i32()
        .ok_or_else(|| anyhow!("global `{}` is not i32", name))
}

fn set_caller_global_i32<T>(
    caller: &mut wt::Caller<'_, T>,
    name: &str,
    value: i32,
) -> Result<()> {
    let global = match caller.get_export(name) {
        Some(wt::Extern::Global(global)) => global,
        _ => return Err(anyhow!("missing global `{}`", name)),
    };
    global
        .set(&mut *caller, wt::Val::I32(value))
        .context("set global")?;
    Ok(())
}

struct RuntimeErrorDiag {
    code: &'static str,
    message: String,
    start: usize,
    end: usize,
    detail: Option<String>,
}

fn extract_runtime_error<T>(
    instance: &wt::Instance,
    store: &mut wt::Store<T>,
) -> Option<RuntimeErrorDiag> {
    let code = get_global(instance, store, "__clg_runtime_error_code")?;
    if code == 0 {
        return None;
    }

    let mut start = get_global(instance, store, "__clg_runtime_error_start").unwrap_or(0) as usize;
    let mut end = get_global(instance, store, "__clg_runtime_error_end").unwrap_or(0) as usize;
    let detail = get_global(instance, store, "__clg_runtime_error_detail").unwrap_or(0);

    let (code_label, mut message, detail_label, reset_span) = match code {
        1 => {
            let label = match detail {
                1 => "ensure",
                _ => "require",
            };
            (
                "R000",
                format!("contract `{}` guard failed", label),
                Some(label.to_string()),
                false,
            )
        }
        2 => (
            "R001",
            "string allocator ran out of memory".to_string(),
            None,
            false,
        ),
        3 => (
            "R002",
            "string runtime detected invalid UTF-8 input".to_string(),
            None,
            false,
        ),
        4 => {
            let kind = match detail {
                1 => "Result",
                _ => "Option",
            };
            let tag = start;
            (
                "R003",
                format!("{kind} variant observed invalid tag {}", tag),
                Some(kind.to_string()),
                true,
            )
        }
        5 => (
            "R004",
            "runtime limits exceeded".to_string(),
            None,
            false,
        ),
        _ => (
            "R999",
            format!("runtime trap with unknown code {}", code),
            None,
            false,
        ),
    };

    if reset_span {
        start = 0;
        end = 0;
    }

    if code_label == "R000" && start == 0 && end == 0 {
        message.push_str(" (no span available)");
    }

    Some(RuntimeErrorDiag {
        code: code_label,
        message,
        start,
        end,
        detail: detail_label,
    })
}
fn get_global<T>(instance: &wt::Instance, store: &mut wt::Store<T>, name: &str) -> Option<i32> {
    let global = instance.get_global(&mut *store, name)?;
    global.get(&mut *store).i32()
}

fn extract_wasmtime_limit_error(trap: &anyhow::Error) -> Option<RuntimeErrorDiag> {
    let trap = trap
        .chain()
        .find_map(|err| err.downcast_ref::<wt::Trap>())?;
    match trap {
        wt::Trap::OutOfFuel | wt::Trap::Interrupt => Some(RuntimeErrorDiag {
            code: "R004",
            message: "runtime limits exceeded".to_string(),
            start: 0,
            end: 0,
            detail: None,
        }),
        _ => None,
    }
}
