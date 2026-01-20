use std::path::PathBuf;

use anyhow::Result;

use crate::commands::helpers::{make_single_json_error, CommandError};
use crate::logging::{Logger, StageTimings};
use crate::signing;

pub fn run(
    module: PathBuf,
    sig: PathBuf,
    pubkey: PathBuf,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    let mut timings = StageTimings::new();
    let result = {
        let _stage = timings.start(logger, "verify_signature");
        signing::verify_signature(&module, &sig, &pubkey)
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
