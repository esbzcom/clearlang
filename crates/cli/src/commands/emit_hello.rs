use anyhow::{Context, Result};
use clg_codegen_wasm::emit_trivial_main;
use std::fs;
use std::path::PathBuf;

use crate::logging::{Logger, StageTimings};

pub fn run(out: PathBuf, logger: Logger) -> Result<()> {
    let mut timings = StageTimings::new();
    let bytes = {
        let _stage = timings.start(logger, "emit_trivial");
        emit_trivial_main().context("emit trivial main wasm")?
    };
    {
        let _stage = timings.start(logger, "write_wasm");
        if let Some(parent) = out.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
        }
        fs::write(&out, bytes).with_context(|| format!("writing {}", out.display()))?;
    }
    logger.summary(&timings);
    Ok(())
}
