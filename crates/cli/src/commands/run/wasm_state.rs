use anyhow::{anyhow, Context, Result};
use wasmtime as wt;

pub(super) fn get_caller_memory<T>(caller: &mut wt::Caller<'_, T>) -> Result<wt::Memory> {
    match caller.get_export("memory") {
        Some(wt::Extern::Memory(mem)) => Ok(mem),
        _ => Err(anyhow!("missing memory export")),
    }
}

pub(super) fn get_caller_global_i32<T>(caller: &mut wt::Caller<'_, T>, name: &str) -> Result<i32> {
    let global = match caller.get_export(name) {
        Some(wt::Extern::Global(global)) => global,
        _ => return Err(anyhow!("missing global `{}`", name)),
    };
    global
        .get(&mut *caller)
        .i32()
        .ok_or_else(|| anyhow!("global `{}` is not i32", name))
}

pub(super) fn set_caller_global_i32<T>(
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
