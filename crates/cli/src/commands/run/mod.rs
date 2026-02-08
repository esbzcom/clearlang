use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use wasmtime as wt;
use wasmtime_wasi::sync::WasiCtxBuilder;

use super::helpers::{make_single_json_error, CommandError};
use crate::logging::{Logger, StageTimings};

mod crypto;
mod env;
mod runtime_error;
mod wasm_state;

use runtime_error::{extract_runtime_error, extract_wasmtime_limit_error, RuntimeErrorDiag};

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
        env::add_env_stubs(&mut linker).context("linking env stubs")?;
        crypto::add_crypto_stubs(&mut linker).context("linking crypto stubs")?;
        linker
            .instantiate(&mut store, &module)
            .context("instantiating module")?
    };
    let func = instance
        .get_typed_func::<(), i32>(&mut store, &invoke)
        .with_context(|| format!("export `{}` not found or wrong type", invoke))?;

    match func.call(&mut store, ()) {
        Ok(result) => {
            if let Some(diag) = extract_runtime_error(&instance, &mut store) {
                return Err(runtime_diag_to_error(Path::new(&file), json_errors, diag));
            }
            println!("{}", result);
            logger.summary(&timings);
            Ok(())
        }
        Err(trap) => {
            if let Some(diag) = extract_runtime_error(&instance, &mut store) {
                return Err(runtime_diag_to_error(Path::new(&file), json_errors, diag));
            }
            if let Some(diag) = extract_wasmtime_limit_error(&trap) {
                return Err(runtime_diag_to_error(Path::new(&file), json_errors, diag));
            }
            Err(trap)
        }
    }
}

fn runtime_diag_to_error(file: &Path, json_errors: bool, diag: RuntimeErrorDiag) -> anyhow::Error {
    if json_errors {
        let json = make_single_json_error(
            diag.code,
            "runtime",
            diag.message,
            file,
            diag.start,
            diag.end,
            diag.detail,
        );
        CommandError::json(json).into()
    } else {
        let span = if diag.start != 0 || diag.end != 0 {
            format!(" at {}..{}", diag.start, diag.end)
        } else {
            String::new()
        };
        anyhow!("{}{} (code {})", diag.message, span, diag.code)
    }
}
