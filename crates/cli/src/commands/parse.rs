use crate::commands::helpers::{make_parse_json_error, CommandError};
use anyhow::{Context, Result};
use clg_parser::{parse as parse_src, parse_errors as parse_src_errs};
use std::fs;
use std::io::Read;
use std::path::PathBuf;

pub fn run(file: PathBuf, json_errors: bool, verbose: bool) -> Result<()> {
    let mut s = String::new();
    fs::File::open(&file)
        .with_context(|| format!("opening {}", file.display()))?
        .read_to_string(&mut s)
        .with_context(|| format!("reading {}", file.display()))?;
    let ast = if json_errors {
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
    };
    if verbose {
        eprintln!("parsed {}", file.display());
    }
    println!("{:#?}", ast);
    Ok(())
}
