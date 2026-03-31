use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use clg_cli::commands::{
    build::{self as cmd_build, CompilerMode, ReleaseProfile, StdCoreLinkMode},
    emit_hello as cmd_emit_hello,
    helpers::CommandError,
    parse as cmd_parse, pkg as cmd_pkg, release as cmd_release, run as cmd_run,
    verify::{self as cmd_verify, VerifyMode},
};
use clg_cli::logging::Logger;
use clg_cli::signing::SignScope;

#[derive(Parser, Debug)]
#[command(name = "clg", version, about = "ClearLang CLI", long_about = None)]
struct Cli {
    /// Emit machine-readable JSON errors instead of human text
    #[arg(long, global = true, default_value_t = false)]
    json_errors: bool,
    /// Increase verbosity (-v, -vv) for stage logs
    #[arg(short, long, action = ArgAction::Count, global = true, default_value_t = 0)]
    verbose: u8,
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
        /// Build contract entrypoints (apply/query exported as init/handle/query)
        #[arg(long, default_value_t = false)]
        contract: bool,
        /// Validate output with `wasm-tools validate`
        #[arg(long, default_value_t = false)]
        validate: bool,
        /// Include debug names in Wasm (name section)
        #[arg(long, default_value_t = false)]
        debug_names: bool,
        /// Emit verification conditions to JSON (see docs/proofs/vc-schema.md)
        #[arg(long, value_name = "FILE")]
        emit_vcs: Option<PathBuf>,
        /// Emit proof artifact summary to JSON (see docs/proofs/proof-artifact-schema.md)
        #[arg(long, value_name = "FILE")]
        emit_proof: Option<PathBuf>,
        /// Compiler strictness profile (permissive, standard, strict)
        #[arg(long, value_enum, default_value_t = CompilerMode::Standard)]
        compiler_mode: CompilerMode,
        /// Release assurance profile (dev or production)
        #[arg(long, value_enum, default_value_t = ReleaseProfile::Dev)]
        release_profile: ReleaseProfile,
        /// Std-core linkage mode (intrinsic or precompiled package contract path)
        #[arg(long, value_enum, default_value_t = StdCoreLinkMode::Intrinsic)]
        std_core_link_mode: StdCoreLinkMode,
        /// Override proof-model assumption checks for VC emission (true/false)
        #[arg(long, action = ArgAction::Set)]
        proof_strict: Option<bool>,
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
        /// Assurance manifest output path (JSON). Requires --sign.
        #[arg(long, value_name = "FILE")]
        assurance_manifest_out: Option<PathBuf>,
        /// Pinned Lean checker version to embed in signing payload
        #[arg(long, value_name = "VERSION")]
        lean_checker_version: Option<String>,
        /// Pinned Coq checker version to embed in signing payload
        #[arg(long, value_name = "VERSION")]
        coq_checker_version: Option<String>,
    },
    /// Run a ClearLang source file or compiled Wasm module (calls an exported function)
    Run {
        /// Input .clear source file or .wasm module
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Export to invoke (default: main)
        #[arg(long, default_value = "main")]
        invoke: String,
    },
    /// Phase 25.2.3: one-command release orchestration (Gate C)
    Release {
        /// Entry ClearLang source file
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Deterministic advisory policy evaluation time (UTC RFC3339)
        #[arg(long, value_name = "RFC3339_UTC")]
        advisory_as_of: String,
        /// Signing key file (JSON)
        #[arg(long, value_name = "FILE")]
        key: PathBuf,
        /// Identifier recorded in signature/manifests
        #[arg(long)]
        key_id: String,
        /// Public key file (JSON) for verify stage
        #[arg(long, value_name = "FILE")]
        pubkey: PathBuf,
        /// Module root containing strict preflight inputs
        #[arg(long, value_name = "DIR")]
        root: Option<PathBuf>,
        /// Output directory for release artifacts
        #[arg(long, value_name = "DIR")]
        out_dir: Option<PathBuf>,
        /// Trust policy file for compile-time verify
        #[arg(long, value_name = "FILE")]
        trust_policy: Option<PathBuf>,
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
        /// Verification mode (`runtime` or `compile-time`)
        #[arg(long, value_enum, default_value_t = VerifyMode::Runtime)]
        verify_mode: VerifyMode,
        /// Trust-anchor policy file for compile-time verify mode
        #[arg(long, value_name = "FILE")]
        trust_policy: Option<PathBuf>,
        /// Assurance manifest file emitted by signed builds
        /// (optional for --require-assurance; required for --release-policy)
        #[arg(long, value_name = "FILE")]
        assurance_manifest: Option<PathBuf>,
        /// Proof artifact emitted by `clg build --emit-proof`
        #[arg(long, value_name = "FILE")]
        proof_artifact: Option<PathBuf>,
        /// Release policy file for assurance-tier gate checks
        #[arg(long, value_name = "FILE", requires = "assurance_manifest")]
        release_policy: Option<PathBuf>,
        /// Require theorem-grade assurance status before accepting verification
        #[arg(long, value_name = "STATUS")]
        require_assurance: Option<String>,
        /// Print human-readable assurance explanation after successful verification
        #[arg(long, default_value_t = false)]
        explain: bool,
    },
    /// Package tooling commands
    Pkg {
        #[command(subcommand)]
        command: PkgCommands,
    },
}

#[derive(Subcommand, Debug)]
enum PkgCommands {
    /// Strict lockfile workflow helpers
    Lock {
        /// Generate a new lockfile from canonical package metadata
        #[arg(long, default_value_t = false, conflicts_with = "update")]
        generate: bool,
        /// Update an existing lockfile from canonical package metadata
        #[arg(long, default_value_t = false, conflicts_with = "generate")]
        update: bool,
        /// Compiler strictness profile (permissive, standard, strict)
        #[arg(long, value_enum, default_value_t = CompilerMode::Standard)]
        compiler_mode: CompilerMode,
        /// Deterministic advisory policy evaluation time (UTC RFC3339)
        #[arg(long, value_name = "RFC3339_UTC")]
        advisory_as_of: Option<String>,
        /// Module root containing package metadata and lockfile
        #[arg(long, value_name = "DIR", default_value = ".")]
        root: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let logger = Logger::from_env(cli.verbose);
    let result = match cli.command {
        Commands::EmitHello { out } => cmd_emit_hello::run(out, logger),
        Commands::Parse { file } => cmd_parse::run(file, cli.json_errors, logger),
        Commands::Build {
            file,
            out,
            contract,
            validate,
            debug_names,
            emit_vcs,
            emit_proof,
            compiler_mode,
            release_profile,
            std_core_link_mode,
            proof_strict,
            sign,
            key,
            key_id,
            scope,
            sig_out,
            assurance_manifest_out,
            lean_checker_version,
            coq_checker_version,
        } => cmd_build::run(
            file,
            out,
            contract,
            validate,
            debug_names,
            emit_vcs,
            emit_proof,
            compiler_mode,
            release_profile,
            std_core_link_mode,
            proof_strict,
            sign,
            key,
            key_id,
            scope.unwrap_or(SignScope::Both),
            sig_out,
            assurance_manifest_out,
            lean_checker_version,
            coq_checker_version,
            cli.json_errors,
            logger,
        ),
        Commands::Run { file, invoke } => cmd_run::run(file, invoke, cli.json_errors, logger),
        Commands::Release {
            file,
            advisory_as_of,
            key,
            key_id,
            pubkey,
            root,
            out_dir,
            trust_policy,
        } => cmd_release::run(
            file,
            advisory_as_of,
            key,
            key_id,
            pubkey,
            root,
            out_dir,
            trust_policy,
            cli.json_errors,
            logger,
        ),
        Commands::Verify {
            module,
            sig,
            pubkey,
            verify_mode,
            trust_policy,
            assurance_manifest,
            proof_artifact,
            release_policy,
            require_assurance,
            explain,
        } => cmd_verify::run(
            module,
            sig,
            pubkey,
            verify_mode,
            trust_policy,
            assurance_manifest,
            proof_artifact,
            release_policy,
            require_assurance,
            explain,
            cli.json_errors,
            logger,
        ),
        Commands::Pkg { command } => match command {
            PkgCommands::Lock {
                generate,
                update,
                compiler_mode,
                advisory_as_of,
                root,
            } => cmd_pkg::run_lock(
                generate,
                update,
                compiler_mode,
                advisory_as_of,
                root,
                cli.json_errors,
                logger,
            ),
        },
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
