use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use lumi_codegen_wasm::emit_trivial_main;

pub fn run(out: PathBuf) -> Result<()> {
    let bytes = emit_trivial_main().context("emit trivial main wasm")?;
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    fs::write(&out, bytes).with_context(|| format!("writing {}", out.display()))?;
    eprintln!("wrote {}", out.display());
    Ok(())
}

