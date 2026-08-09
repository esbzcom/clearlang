use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use clg_cli::commands::{
    build::{self as cmd_build, CompilerMode, ReleaseProfile, StdCoreLinkMode},
    check as cmd_check, emit_hello as cmd_emit_hello, fmt as cmd_fmt,
    helpers::CommandError,
    lint as cmd_lint, parse as cmd_parse, pkg as cmd_pkg, release as cmd_release, run as cmd_run,
    simulate as cmd_simulate, strict as cmd_strict,
    test::{self as cmd_test, TestReportFormat},
    verify::{self as cmd_verify, VerifyMode},
};
use clg_cli::logging::Logger;
use clg_cli::signing::SignScope;

#[derive(Parser, Debug)]
#[command(
    name = "clg",
    version,
    about = "ClearLang CLI (primary: check, test, release; advanced: parse, build, run, verify, verify-bundle, pkg, strict)",
    long_about = None
)]
struct Cli {
    /// Emit machine-readable JSON errors instead of human text
    #[arg(long, global = true, default_value_t = false)]
    json_errors: bool,
    /// Emit structured JSON stage/progress events on stderr (NDJSON)
    #[arg(long, global = true, default_value_t = false)]
    json_events: bool,
    /// Enforce non-interactive CLI behavior (no prompts; plugin-safe I/O channels)
    #[arg(long, global = true, default_value_t = false)]
    non_interactive: bool,
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
    /// Deterministically format `.clear` sources
    Fmt {
        /// Input `.clear` file or directory root
        #[arg(value_name = "PATH")]
        path: PathBuf,
        /// Check mode: fail if formatting changes are required
        #[arg(long, default_value_t = false)]
        check: bool,
    },
    /// Run deterministic quality/safety lints on `.clear` sources
    Lint {
        /// Input `.clear` file or directory root
        #[arg(value_name = "PATH")]
        path: PathBuf,
        /// Fail if any warning is reported
        #[arg(long, default_value_t = false)]
        deny_warnings: bool,
    },
    /// Fast deterministic strict-aligned preflight (non-release)
    Check {
        /// Input ClearLang source file
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Module root containing strict release-policy inputs
        #[arg(long, value_name = "DIR")]
        root: Option<PathBuf>,
    },
    /// Phase 25.3 foundation: discover unit tests and validate test contracts
    Test {
        /// Project root/tests root/unit root (defaults to current directory)
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
        /// Select tests by substring match on stable test id
        #[arg(long, value_name = "PATTERN")]
        filter: Option<String>,
        /// Report output format
        #[arg(long, value_enum, default_value_t = TestReportFormat::Human)]
        report: TestReportFormat,
    },
    /// Advanced expert/debug compile command. For production release, use `clg release`.
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
        /// Emit the canonical persistent-contract state schema JSON
        #[arg(long, value_name = "FILE")]
        emit_contract_state_schema: Option<PathBuf>,
        /// Emit the canonical target-facing contract ABI descriptor JSON
        #[arg(long, value_name = "FILE")]
        emit_contract_abi: Option<PathBuf>,
        /// Require the contract state schema to be append-only compatible with a prior schema
        #[arg(long, value_name = "FILE")]
        check_contract_state_schema: Option<PathBuf>,
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
    /// Deterministically simulate a supported scalar contract transition
    Simulate {
        #[arg(value_name = "FILE")]
        file: PathBuf,
        #[arg(long)]
        function: String,
        #[arg(long, value_name = "FILE")]
        state: PathBuf,
        #[arg(long, value_name = "FILE")]
        args: PathBuf,
        #[arg(long, value_name = "FILE")]
        state_out: PathBuf,
        #[arg(long, value_name = "FILE")]
        trace_out: PathBuf,
        #[arg(long, default_value = "clg:caller:default")]
        caller: String,
        #[arg(long, default_value_t = 0)]
        value: u64,
        #[arg(long, default_value_t = 0)]
        block_number: u64,
        #[arg(long, default_value_t = 0)]
        timestamp: u64,
        #[arg(long, default_value_t = 100000)]
        gas_limit: u64,
    },
    /// Phase 25.2.3: one-command release orchestration (Gate C)
    Release {
        /// Signing key file (JSON)
        #[arg(
            long,
            value_name = "FILE",
            required_unless_present = "check_only",
            requires = "pubkey"
        )]
        key: Option<PathBuf>,
        /// Public key file (JSON) for verify stage
        #[arg(
            long,
            value_name = "FILE",
            required_unless_present = "check_only",
            requires = "key"
        )]
        pubkey: Option<PathBuf>,
        /// Run deterministic release readiness preflight only (lock/build/prove/artifact scan; no sign/verify/bundle)
        #[arg(long, default_value_t = false, conflicts_with_all = ["key", "pubkey"])]
        check_only: bool,
        /// Module root containing strict preflight inputs
        #[arg(long, value_name = "DIR")]
        root: Option<PathBuf>,
    },
    /// Verify a release bundle manifest and its referenced artifacts
    VerifyBundle {
        /// Release bundle manifest emitted by `clg release`
        #[arg(long, value_name = "FILE")]
        bundle: PathBuf,
        /// Public key file (JSON) used to verify release signatures
        #[arg(
            long,
            value_name = "FILE",
            required_unless_present = "keyring",
            conflicts_with = "keyring"
        )]
        pubkey: Option<PathBuf>,
        /// Keyring file (JSON) used to resolve `key_id` -> public key for rotated release keys
        #[arg(
            long,
            value_name = "FILE",
            required_unless_present = "pubkey",
            conflicts_with = "pubkey"
        )]
        keyring: Option<PathBuf>,
        /// Require a signed provenance artifact in the bundle and fail closed when missing/invalid
        #[arg(long, default_value_t = false)]
        require_provenance: bool,
    },
    /// Strict workflow commands
    Strict {
        #[command(subcommand)]
        command: StrictCommands,
    },
    /// Advanced expert/debug verify command. For production release, use `clg release`.
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
    /// Advanced package tooling commands
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
    /// Generate `clg.project.json` from canonical package inputs for Gate E migration
    MigrateManifest {
        /// Module root containing canonical package metadata (and optional lockfile)
        #[arg(long, value_name = "DIR", default_value = ".")]
        root: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum StrictCommands {
    /// Initialize strict preflight inputs and release defaults
    Init {
        /// Project root directory
        #[arg(value_name = "ROOT")]
        root: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let _non_interactive = cli.non_interactive;
    let logger = Logger::from_env(cli.verbose, cli.json_events);
    let result = match cli.command {
        Commands::EmitHello { out } => cmd_emit_hello::run(out, logger.with_command("emit-hello")),
        Commands::Parse { file } => {
            cmd_parse::run(file, cli.json_errors, logger.with_command("parse"))
        }
        Commands::Fmt { path, check } => {
            cmd_fmt::run(path, check, cli.json_errors, logger.with_command("fmt"))
        }
        Commands::Lint {
            path,
            deny_warnings,
        } => cmd_lint::run(
            path,
            deny_warnings,
            cli.json_errors,
            logger.with_command("lint"),
        ),
        Commands::Check { file, root } => {
            cmd_check::run(file, root, cli.json_errors, logger.with_command("check"))
        }
        Commands::Test {
            path,
            filter,
            report,
        } => cmd_test::run(
            path,
            filter,
            report,
            cli.json_errors,
            logger.with_command("test"),
        ),
        Commands::Build {
            file,
            out,
            contract,
            validate,
            debug_names,
            emit_contract_state_schema,
            emit_contract_abi,
            check_contract_state_schema,
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
        } => {
            emit_release_migration_guidance_for_build(release_profile, sign);
            cmd_build::run(
                file,
                out,
                contract,
                validate,
                debug_names,
                emit_contract_state_schema,
                emit_contract_abi,
                check_contract_state_schema,
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
                logger.with_command("build"),
            )
        }
        Commands::Run { file, invoke } => {
            cmd_run::run(file, invoke, cli.json_errors, logger.with_command("run"))
        }
        Commands::Simulate {
            file,
            function,
            state,
            args,
            state_out,
            trace_out,
            caller,
            value,
            block_number,
            timestamp,
            gas_limit,
        } => cmd_simulate::run(
            file,
            function,
            state,
            args,
            state_out,
            trace_out,
            caller,
            value,
            block_number,
            timestamp,
            gas_limit,
            logger.with_command("simulate"),
        ),
        Commands::Release {
            key,
            pubkey,
            check_only,
            root,
        } => cmd_release::run(
            key,
            pubkey,
            check_only,
            root,
            cli.json_errors,
            logger.with_command("release"),
        ),
        Commands::VerifyBundle {
            bundle,
            pubkey,
            keyring,
            require_provenance,
        } => cmd_release::run_verify_bundle(
            bundle,
            pubkey,
            keyring,
            require_provenance,
            cli.json_errors,
            logger.with_command("verify-bundle"),
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
        } => {
            emit_release_migration_guidance_for_verify(
                verify_mode,
                &release_policy,
                &require_assurance,
            );
            cmd_verify::run(
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
                logger.with_command("verify"),
            )
        }
        Commands::Pkg { command } => match command {
            PkgCommands::Lock {
                generate,
                update,
                compiler_mode,
                advisory_as_of,
                root,
            } => cmd_pkg::run_lock(
                cmd_pkg::RunLockArgs {
                    generate,
                    update,
                    compiler_mode,
                    advisory_as_of,
                    root,
                    json_errors: cli.json_errors,
                    emit_stdout_summary: true,
                },
                logger.with_command("pkg"),
            ),
            PkgCommands::MigrateManifest { root } => cmd_pkg::run_migrate_manifest(
                cmd_pkg::RunMigrateManifestArgs {
                    root,
                    json_errors: cli.json_errors,
                    emit_stdout_summary: true,
                },
                logger.with_command("pkg"),
            ),
        },
        Commands::Strict { command } => match command {
            StrictCommands::Init { root } => {
                cmd_strict::run_init(root, cli.json_errors, logger.with_command("strict"))
            }
        },
    };

    if let Err(err) = result {
        match err.downcast::<CommandError>() {
            Ok(cmd_err) => {
                cmd_err.emit();
                std::process::exit(cmd_err.exit_code());
            }
            Err(err) => {
                eprintln!("{err:#}");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn emit_release_migration_guidance_for_build(release_profile: ReleaseProfile, sign: bool) {
    if release_profile == ReleaseProfile::Production || sign {
        eprintln!(
            "migration: prefer `clg release --key <FILE> --pubkey <FILE> [--root <DIR>]` for production release artifacts; `clg build` release flags are expert/debug-only"
        );
    }
}

fn emit_release_migration_guidance_for_verify(
    verify_mode: VerifyMode,
    release_policy: &Option<PathBuf>,
    require_assurance: &Option<String>,
) {
    if verify_mode == VerifyMode::CompileTime
        || release_policy.is_some()
        || require_assurance.is_some()
    {
        eprintln!(
            "migration: prefer `clg release --key <FILE> --pubkey <FILE> [--root <DIR>]` for production release verification/bundling; standalone `clg verify` release gates are expert/debug-only"
        );
    }
}
