use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use lumi_codegen_wasm::emit_trivial_main;

#[derive(Parser, Debug)]
#[command(name = "lumi", version, about = "Lumi CLI", long_about = None)]
struct Args {
    /// Output wasm file path
    #[arg(short, long, default_value = "hello.wasm")]
    out: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let bytes = emit_trivial_main().context("emit trivial main wasm")?;
    fs::write(&args.out, bytes).with_context(|| format!("writing {}", args.out.display()))?;
    eprintln!("wrote {}", args.out.display());
    Ok(())
}
