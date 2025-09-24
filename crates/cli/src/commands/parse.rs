use std::fs;
use std::io::Read;
use std::path::PathBuf;
use anyhow::{Context, Result};
use clg_parser::{parse as parse_src, parse_errors as parse_src_errs};
use crate::commands::helpers::emit_parse_structured_json_errors;

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
                emit_parse_structured_json_errors(&file, &errs);
                std::process::exit(1);
            }
        }
    } else {
        match parse_src(&s) {
            Ok(ast) => ast,
            Err(e) => return Err(anyhow::anyhow!("parse failed: {}", e)),
        }
    };
    if verbose { eprintln!("parsed {}", file.display()); }
    println!("{:#?}", ast);
    Ok(())
}

