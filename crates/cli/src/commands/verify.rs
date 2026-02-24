use std::path::PathBuf;

use anyhow::Result;

use crate::commands::helpers::{make_single_json_error, CommandError};
use crate::logging::{Logger, StageTimings};
use crate::signing;

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum VerifyMode {
    Runtime,
    CompileTime,
}

pub fn run(
    module: PathBuf,
    sig: PathBuf,
    pubkey: PathBuf,
    verify_mode: VerifyMode,
    trust_policy: Option<PathBuf>,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    let mut timings = StageTimings::new();
    let result = {
        let _stage = timings.start(logger, "verify_signature");
        match verify_mode {
            VerifyMode::Runtime => {
                if trust_policy.is_some() {
                    Err(signing::VerifyError::new(
                        signing::VerifyErrorCode::TrustAnchorFailure,
                        "`--trust-policy` requires `--verify-mode compile-time`",
                    ))
                } else {
                    signing::verify_signature(&module, &sig, &pubkey)
                }
            }
            VerifyMode::CompileTime => match trust_policy.as_ref() {
                Some(policy) => {
                    signing::verify_signature_with_trust_policy(&module, &sig, &pubkey, policy)
                }
                None => Err(signing::VerifyError::new(
                    signing::VerifyErrorCode::TrustAnchorFailure,
                    "`--verify-mode compile-time` requires `--trust-policy <FILE>`",
                )),
            },
        }
    };
    match result {
        Ok(()) => {
            logger.summary(&timings);
            Ok(())
        }
        Err(err) => {
            if json_errors {
                let json = make_single_json_error(
                    err.code(),
                    "verify",
                    err.to_string(),
                    &module,
                    0,
                    0,
                    None,
                );
                Err(CommandError::json(json).into())
            } else {
                Err(anyhow::anyhow!(err).context("verification failed"))
            }
        }
    }
}
