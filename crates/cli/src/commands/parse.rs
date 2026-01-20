use crate::commands::helpers::{make_parse_json_error, CommandError};
use crate::logging::{Logger, StageTimings};
use anyhow::{Context, Result};
use clg_parser::{parse as parse_src, parse_errors as parse_src_errs};
use std::fs;
use std::io::Read;
use std::path::PathBuf;

pub fn run(file: PathBuf, json_errors: bool, logger: Logger) -> Result<()> {
    let mut timings = StageTimings::new();
    let mut s = String::new();
    {
        let _stage = timings.start(logger, "read_source");
        fs::File::open(&file)
            .with_context(|| format!("opening {}", file.display()))?
            .read_to_string(&mut s)
            .with_context(|| format!("reading {}", file.display()))?;
    }
    let ast = {
        let _stage = timings.start(logger, "parse");
        if json_errors {
            match parse_src_errs(&s) {
                Ok(ast) => ast,
                Err(errs) => {
                    let json = make_parse_json_error(&file, &errs);
                    return Err(CommandError::json(json).into());
                }
            }
        } else {
            match parse_src(&s) {
                Ok(ast) => ast,
                Err(e) => return Err(anyhow::anyhow!("parse failed: {}", e)),
            }
        }
    };
    {
        let _stage = timings.start(logger, "emit_ast");
        println!("{:#?}", ast);
    }
    logger.summary(&timings);
    Ok(())
}
