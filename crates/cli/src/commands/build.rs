use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clg_codegen_wasm::{emit_from_ir_with_opts, CodegenOpts};
use clg_typer::{check as type_check, TyperError};
use clg_ir::IrType;
use clg_parser::{parse as parse_src, parse_errors as parse_src_errs};

use crate::commands::helpers::{emit_parse_structured_json_errors, emit_single_json_error, emit_type_json_error};

pub fn run(file: PathBuf, out: PathBuf, validate: bool, debug_names: bool, json_errors: bool, verbose: bool) -> Result<()> {
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
    let ir = match type_check(&ast) {
        Ok(ir) => ir,
        Err(e) => {
            if json_errors {
                if let Some(te) = e.downcast_ref::<TyperError>() {
                    emit_single_json_error(
                        te.code,
                        "type",
                        &te.message,
                        &file,
                        te.start,
                        te.end,
                        None,
                    );
                } else {
                    emit_type_json_error(&file, &format!("{e:#}"));
                }
                std::process::exit(1);
            } else {
                return Err(e.context("type-check failed"));
            }
        }
    };
    if verbose { eprintln!("type-checked and lowered to IR"); }
    match ir.funcs.iter().find(|f| f.name == "main") {
        Some(f) => {
            if f.ret != Some(IrType::Int) || !f.params.is_empty() {
                if json_errors {
                    emit_single_json_error(
                        "C001",
                        "build",
                        "only `main() -> Int` is supported in this phase",
                        &file,
                        0,
                        0,
                        None,
                    );
                    std::process::exit(1);
                } else {
                    anyhow::bail!("only `main() -> Int` is supported in this phase");
                }
            }
        }
        None => {
            if json_errors {
                emit_single_json_error(
                    "C002",
                    "build",
                    "missing `main` function",
                    &file,
                    0,
                    0,
                    None,
                );
                std::process::exit(1);
            } else {
                anyhow::bail!("missing `main` function")
            }
        }
    }
    let bytes = emit_from_ir_with_opts(&ir, CodegenOpts { debug_names })
        .context("codegen (IR→Wasm) failed")?;
    if verbose { eprintln!("generated Wasm ({} bytes)", bytes.len()); }
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    fs::write(&out, bytes).with_context(|| format!("writing {}", out.display()))?;
    if verbose { eprintln!("wrote {}", out.display()); }
    if validate {
        let status = std::process::Command::new("wasm-tools")
            .arg("validate")
            .arg(&out)
            .status();
        match status {
            Ok(s) if s.success() => if verbose { eprintln!("validated {}", out.display()) },
            Ok(s) => anyhow::bail!("wasm-tools validate failed with status {:?}", s.code()),
            Err(e) => anyhow::bail!("failed to run wasm-tools: {}", e),
        }
    }
    Ok(())
}

