use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use lumi_codegen_wasm::{emit_from_ir, emit_trivial_main, emit_from_ir_with_opts, CodegenOpts};
use lumi_typer::check as type_check;
use lumi_parser::parse as parse_src;
use wasmtime as wt;

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
            eprintln!("parsed {}", file.display());
            println!("{:#?}", ast);
        }
        Commands::Build { file, out, validate, debug_names } => {
            let mut s = String::new();
            fs::File::open(&file)
                .with_context(|| format!("opening {}", file.display()))?
                .read_to_string(&mut s)
                .with_context(|| format!("reading {}", file.display()))?;
            let ast = parse_src(&s).map_err(|e| anyhow::anyhow!("parse failed: {}", e))?;
            // Phase 3.5: type-check and lower to IR, then codegen IR → Wasm
            let ir = type_check(&ast).context("type-check failed")?;
            eprintln!("type-checked and lowered to IR");
            let bytes = emit_from_ir_with_opts(&ir, CodegenOpts { debug_names })
                .context("codegen (IR→Wasm) failed")?;
            eprintln!("generated Wasm ({} bytes)", bytes.len());
            if let Some(parent) = out.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("creating {}", parent.display()))?;
                }
            }
            fs::write(&out, bytes).with_context(|| format!("writing {}", out.display()))?;
            eprintln!("wrote {}", out.display());
            if validate {
                // Best-effort validation using external `wasm-tools`
                let status = std::process::Command::new("wasm-tools")
                    .arg("validate")
                    .arg(&out)
                    .status();
                match status {
                    Ok(s) if s.success() => eprintln!("validated {}", out.display()),
                    Ok(s) => anyhow::bail!("wasm-tools validate failed with status {:?}", s.code()),
                    Err(e) => anyhow::bail!("failed to run wasm-tools: {}", e),
                }
            }
        }
        Commands::Run { file, invoke } => {
            // Minimal embedded Wasmtime runner for zero-arg i32 functions
            let engine = wt::Engine::default();
            let module = wt::Module::from_file(&engine, &file)
                .with_context(|| format!("loading {}", file.display()))?;
            let mut store = wt::Store::new(&engine, ());
            let instance = wt::Instance::new(&mut store, &module, &[])
                .context("instantiating module")?;
            // For now, expect a zero-arg i32 function (e.g., main)
            let func = instance
                .get_typed_func::<(), i32>(&mut store, &invoke)
                .with_context(|| format!("export `{}` not found or wrong type", invoke))?;
            let result = func.call(&mut store, ()).context("invoking function")?;
            println!("{}", result);
        }
    }
    Ok(())
}
