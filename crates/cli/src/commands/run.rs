use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use wasmtime as wt;

use super::helpers::{make_single_json_error, CommandError};
use crate::logging::{Logger, StageTimings};

pub fn run(file: PathBuf, invoke: String, json_errors: bool, logger: Logger) -> Result<()> {
    let mut timings = StageTimings::new();
    let engine = wt::Engine::default();
    let module = {
        let _stage = timings.start(logger, "load_module");
        wt::Module::from_file(&engine, &file)
            .with_context(|| format!("loading {}", file.display()))?
    };
    let mut store = wt::Store::new(&engine, ());
    let instance = {
        let _stage = timings.start(logger, "instantiate");
        wt::Instance::new(&mut store, &module, &[]).context("instantiating module")?
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
            Err(trap)
        }
    }
}

struct RuntimeErrorDiag {
    code: &'static str,
    message: String,
    start: usize,
    end: usize,
    detail: Option<String>,
}

fn extract_runtime_error(
    instance: &wt::Instance,
    store: &mut wt::Store<()>,
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
fn get_global(instance: &wt::Instance, store: &mut wt::Store<()>, name: &str) -> Option<i32> {
    let global = instance.get_global(&mut *store, name)?;
    global.get(&mut *store).i32()
}
