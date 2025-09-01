use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use lumi_codegen_wasm::emit_trivial_main;
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::EmitHello { out } => {
            let bytes = emit_trivial_main().context("emit trivial main wasm")?;
            fs::write(&out, bytes).with_context(|| format!("writing {}", out.display()))?;
            eprintln!("wrote {}", out.display());
        }
        Commands::Parse { file } => {
            let mut s = String::new();
            fs::File::open(&file)
                .with_context(|| format!("opening {}", file.display()))?
                .read_to_string(&mut s)
                .with_context(|| format!("reading {}", file.display()))?;
            let ast = parse_src(&s).context("parse failed")?;
            println!("{:#?}", ast);
        }
    }
    Ok(())
}
