use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use clg_cli::{
    commands::{contract_release, helpers::CommandError},
    logging::Logger,
};

/// Independently verify a ClearLang contract-release bundle using only explicit local inputs.
#[derive(Parser, Debug)]
#[command(
    name = "clg-contract-verifier",
    version,
    about = "Offline ClearLang contract-release bundle verifier",
    long_about = None
)]
struct Cli {
    /// Emit machine-readable JSON errors instead of human text
    #[arg(long, global = true, default_value_t = false)]
    json_errors: bool,
    /// Emit structured JSON stage/progress events on stderr (NDJSON)
    #[arg(long, global = true, default_value_t = false)]
    json_events: bool,
    /// Increase verbosity (-v, -vv) for stage logs
    #[arg(short, long, action = ArgAction::Count, global = true, default_value_t = 0)]
    verbose: u8,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Verify one contract-release bundle and all bundle-local evidence
    VerifyRelease {
        /// Contract-release bundle manifest
        #[arg(long, value_name = "FILE")]
        bundle: PathBuf,
        /// Explicit Ed25519 release public-key JSON
        #[arg(long, value_name = "FILE")]
        pubkey: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let logger = Logger::from_env(cli.verbose, cli.json_events);
    let result = match cli.command {
        Commands::VerifyRelease { bundle, pubkey } => contract_release::verify(
            bundle,
            pubkey,
            cli.json_errors,
            logger.with_command("contract-verify-release"),
        ),
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
