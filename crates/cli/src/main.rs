use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use lumi_codegen_wasm::{emit_from_ast, emit_trivial_main};
use lumi_parser::parse as parse_src;

#[derive(Parser, Debug)]
#[command(name = "lumi", version, about = "Lumi CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Phase 1: Emit a trivial WASM with main() returning 42
    EmitHello {
        /// Output wasm file path
        #[arg(short, long, default_value = "hello.wasm")]
        out: PathBuf,
    },
    /// Phase 2: Parse a Lumi source file and print the AST
    Parse {
        /// Input Lumi source file
        #[arg(value_name = "FILE")] 
        file: PathBuf,
    },
    /// Compile a Lumi source file to WASM (very minimal subset for now)
    Build {
        /// Input Lumi source file
        #[arg(value_name = "FILE")] 
        file: PathBuf,
        /// Output wasm file path
        #[arg(short, long, default_value = "out.wasm")]
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::EmitHello { out } => {
            let bytes = emit_trivial_main().context("emit trivial main wasm")?;
            if let Some(parent) = out.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("creating {}", parent.display()))?;
                }
            }
            fs::write(&out, bytes).with_context(|| format!("writing {}", out.display()))?;
            eprintln!("wrote {}", out.display());
        }
        Commands::Parse { file } => {
            let mut s = String::new();
            fs::File::open(&file)
                .with_context(|| format!("opening {}", file.display()))?
                .read_to_string(&mut s)
                .with_context(|| format!("reading {}", file.display()))?;
            let ast = parse_src(&s).map_err(|e| anyhow::anyhow!("parse failed: {}", e))?;
            println!("{:#?}", ast);
        }
        Commands::Build { file, out } => {
            let mut s = String::new();
            fs::File::open(&file)
                .with_context(|| format!("opening {}", file.display()))?
                .read_to_string(&mut s)
                .with_context(|| format!("reading {}", file.display()))?;
            let ast = parse_src(&s).map_err(|e| anyhow::anyhow!("parse failed: {}", e))?;
            let bytes = emit_from_ast(&ast).context("codegen failed")?;
            if let Some(parent) = out.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("creating {}", parent.display()))?;
                }
            }
            fs::write(&out, bytes).with_context(|| format!("writing {}", out.display()))?;
            eprintln!("wrote {}", out.display());
        }
    }
    Ok(())
}
