use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
mod commands;
use commands::{build as cmd_build, emit_hello as cmd_emit_hello, parse as cmd_parse, run as cmd_run};

#[derive(Parser, Debug)]
#[command(name = "clearlang", version, about = "ClearLang CLI", long_about = None)]
struct Cli {
    /// Emit machine-readable JSON errors instead of human text
    #[arg(long, global = true, default_value_t = false)]
    json_errors: bool,
    /// Emit verbose stage logs
    #[arg(long, global = true, default_value_t = false)]
    verbose: bool,
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
    /// Phase 2: Parse a ClearLang source file and print the AST
    Parse {
        /// Input ClearLang source file
        #[arg(value_name = "FILE")] 
        file: PathBuf,
    },
    /// Compile a ClearLang source file to WASM (very minimal subset for now)
    Build {
        /// Input ClearLang source file
        #[arg(value_name = "FILE")] 
        file: PathBuf,
        /// Output wasm file path
        #[arg(short, long, default_value = "out.wasm")]
        out: PathBuf,
        /// Validate output with `wasm-tools validate`
        #[arg(long, default_value_t = false)]
        validate: bool,
        /// Include debug names in Wasm (name section)
        #[arg(long, default_value_t = false)]
        debug_names: bool,
    },
    /// Run a compiled Wasm module (calls an exported function)
    Run {
        /// Input Wasm file
        #[arg(value_name = "FILE")] 
        file: PathBuf,
        /// Export to invoke (default: main)
        #[arg(long, default_value = "main")]
        invoke: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::EmitHello { out } => { cmd_emit_hello::run(out)?; }
        Commands::Parse { file } => { cmd_parse::run(file, cli.json_errors, cli.verbose)?; }
        Commands::Build { file, out, validate, debug_names } => {
            cmd_build::run(file, out, validate, debug_names, cli.json_errors, cli.verbose)?;
        }
        Commands::Run { file, invoke } => { cmd_run::run(file, invoke)?; }
    }
    Ok(())
}
 
