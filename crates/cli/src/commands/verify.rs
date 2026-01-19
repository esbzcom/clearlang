use std::path::PathBuf;

use anyhow::Result;

use crate::commands::helpers::{make_single_json_error, CommandError};
use crate::signing;

pub fn run(module: PathBuf, sig: PathBuf, pubkey: PathBuf, json_errors: bool) -> Result<()> {
    match signing::verify_signature(&module, &sig, &pubkey) {
        Ok(()) => Ok(()),
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
