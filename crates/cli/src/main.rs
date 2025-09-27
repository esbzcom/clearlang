use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
mod commands;
mod proofs;
mod signing;
use commands::{
    build as cmd_build, emit_hello as cmd_emit_hello, helpers::CommandError, parse as cmd_parse,
    run as cmd_run, verify as cmd_verify,
};
use signing::SignScope;

#[derive(Parser, Debug)]
#[command(name = "clg", version, about = "ClearLang CLI", long_about = None)]
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
        /// Emit verification conditions to JSON (see docs/proofs/vc-schema.md)
        #[arg(long, value_name = "FILE")]
        emit_vcs: Option<PathBuf>,
        /// Sign outputs using an Ed25519 key (requires --emit-vcs)
        #[arg(long, default_value_t = false, requires_all = ["key", "key_id", "sig_out"], requires = "emit_vcs")]
        sign: bool,
        /// Signing key file (JSON)
        #[arg(long, value_name = "FILE", requires = "sign")]
        key: Option<PathBuf>,
        /// Identifier recorded in the signature file
        #[arg(long, requires = "sign")]
        key_id: Option<String>,
        /// Signing scope (module/proofs/both)
        #[arg(long, value_enum, requires = "sign")]
        scope: Option<SignScope>,
        /// Signature output path (JSON)
        #[arg(long, value_name = "FILE", requires = "sign")]
        sig_out: Option<PathBuf>,
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
    /// Verify a signed proof bundle against a Wasm module
    Verify {
        /// Wasm module to verify
        #[arg(long, value_name = "FILE")]
        module: PathBuf,
        /// Signature file emitted by `clg build`
        #[arg(long, value_name = "FILE")]
        sig: PathBuf,
        /// Public key file (JSON)
        #[arg(long, value_name = "FILE")]
        pubkey: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::EmitHello { out } => cmd_emit_hello::run(out),
        Commands::Parse { file } => cmd_parse::run(file, cli.json_errors, cli.verbose),
        Commands::Build {
            file,
            out,
            validate,
            debug_names,
            emit_vcs,
            sign,
            key,
            key_id,
            scope,
            sig_out,
        } => cmd_build::run(
            file,
            out,
            validate,
            debug_names,
            emit_vcs,
            sign,
            key,
            key_id,
            scope.unwrap_or(SignScope::Both),
            sig_out,
            cli.json_errors,
            cli.verbose,
        ),
        Commands::Run { file, invoke } => cmd_run::run(file, invoke, cli.json_errors),
        Commands::Verify {
            module,
            sig,
            pubkey,
        } => cmd_verify::run(module, sig, pubkey, cli.json_errors),
    };

    if let Err(err) = result {
        match err.downcast::<CommandError>() {
            Ok(cmd_err) => {
                cmd_err.emit();
                std::process::exit(cmd_err.exit_code());
            }
            Err(err) => return Err(err),
        }
    }

    Ok(())
}
