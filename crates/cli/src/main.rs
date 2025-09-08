use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use lumi_codegen_wasm::{emit_trivial_main, emit_from_ir_with_opts, CodegenOpts};
use lumi_typer::check as type_check;
use lumi_ir::IrType;
use lumi_parser::parse as parse_src;
use wasmtime as wt;
use serde::Serialize;

#[derive(Parser, Debug)]
#[command(name = "lumi", version, about = "Lumi CLI", long_about = None)]
struct Cli {
    /// Emit machine-readable JSON errors instead of human text
    #[arg(long, global = true, default_value_t = false)]
    json_errors: bool,
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
    /// Phase 2: Parse a Lumi source file and print the AST
    Parse {
        /// Input Lumi source file
        #[arg(value_name = "FILE")] 
        file: PathBuf,
    },
    /// Compile a Lumi source file to WASM (very minimal subset for now)
    Build {
        /// Input Lumi source file
        #[arg(value_name = "FILE")] 
        file: PathBuf,
        /// Output wasm file path
        #[arg(short, long, default_value = "out.wasm")]
        out: PathBuf,
        /// Validate output with `wasm-tools validate`
        #[arg(long, default_value_t = false)]
        validate: bool,
        /// Include debug names in Wasm (name section)
        #[arg(long, default_value_t = false)]
        debug_names: bool,
    },
    /// Run a compiled Wasm module (calls an exported function)
    Run {
        /// Input Wasm file
        #[arg(value_name = "FILE")] 
        file: PathBuf,
        /// Export to invoke (default: main)
        #[arg(long, default_value = "main")]
        invoke: String,
    },
}

#[derive(Serialize)]
struct JsonError {
    ok: bool,
    errors: Vec<JsonErrorItem>,
}

#[derive(Serialize)]
struct JsonErrorItem {
    code: &'static str,
    stage: &'static str,
    message: String,
    file: String,
    start: usize,
    end: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    function: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::EmitHello { out } => {
            let bytes = emit_trivial_main().context("emit trivial main wasm")?;
            if let Some(parent) = out.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("creating {}", parent.display()))?;
                }
            }
            fs::write(&out, bytes).with_context(|| format!("writing {}", out.display()))?;
            eprintln!("wrote {}", out.display());
        }
        Commands::Parse { file } => {
            let mut s = String::new();
            fs::File::open(&file)
                .with_context(|| format!("opening {}", file.display()))?
                .read_to_string(&mut s)
                .with_context(|| format!("reading {}", file.display()))?;
            let ast = match parse_src(&s) {
                Ok(ast) => ast,
                Err(e) => {
                    if cli.json_errors {
                        emit_parse_json_errors(&file, &e);
                        std::process::exit(1);
                    } else {
                        return Err(anyhow::anyhow!("parse failed: {}", e));
                    }
                }
            };
            eprintln!("parsed {}", file.display());
            println!("{:#?}", ast);
        }
        Commands::Build { file, out, validate, debug_names } => {
            let mut s = String::new();
            fs::File::open(&file)
                .with_context(|| format!("opening {}", file.display()))?
                .read_to_string(&mut s)
                .with_context(|| format!("reading {}", file.display()))?;
            let ast = match parse_src(&s) {
                Ok(ast) => ast,
                Err(e) => {
                    if cli.json_errors {
                        emit_parse_json_errors(&file, &e);
                        std::process::exit(1);
                    } else {
                        return Err(anyhow::anyhow!("parse failed: {}", e));
                    }
                }
            };
            // Phase 3.5: type-check and lower to IR, then codegen IR → Wasm
            let ir = match type_check(&ast) {
                Ok(ir) => ir,
                Err(e) => {
                    if cli.json_errors {
                        emit_type_json_error(&file, &format!("{e:#}"));
                        std::process::exit(1);
                    } else {
                        return Err(e.context("type-check failed"));
                    }
                }
            };
            eprintln!("type-checked and lowered to IR");
            // Require a main function returning Int (phase constraint)
            match ir.funcs.iter().find(|f| f.name == "main") {
                Some(f) => {
                    if f.ret != Some(IrType::Int) || !f.params.is_empty() {
                        if cli.json_errors {
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
                    if cli.json_errors {
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
                },
            }
            let bytes = emit_from_ir_with_opts(&ir, CodegenOpts { debug_names })
                .context("codegen (IR→Wasm) failed")?;
            eprintln!("generated Wasm ({} bytes)", bytes.len());
            if let Some(parent) = out.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("creating {}", parent.display()))?;
                }
            }
            fs::write(&out, bytes).with_context(|| format!("writing {}", out.display()))?;
            eprintln!("wrote {}", out.display());
            if validate {
                // Best-effort validation using external `wasm-tools`
                let status = std::process::Command::new("wasm-tools")
                    .arg("validate")
                    .arg(&out)
                    .status();
                match status {
                    Ok(s) if s.success() => eprintln!("validated {}", out.display()),
                    Ok(s) => anyhow::bail!("wasm-tools validate failed with status {:?}", s.code()),
                    Err(e) => anyhow::bail!("failed to run wasm-tools: {}", e),
                }
            }
        }
        Commands::Run { file, invoke } => {
            // Minimal embedded Wasmtime runner for zero-arg i32 functions
            let engine = wt::Engine::default();
            let module = wt::Module::from_file(&engine, &file)
                .with_context(|| format!("loading {}", file.display()))?;
            let mut store = wt::Store::new(&engine, ());
            let instance = wt::Instance::new(&mut store, &module, &[])
                .context("instantiating module")?;
            // For now, expect a zero-arg i32 function (e.g., main)
            let func = instance
                .get_typed_func::<(), i32>(&mut store, &invoke)
                .with_context(|| format!("export `{}` not found or wrong type", invoke))?;
            let result = func.call(&mut store, ()).context("invoking function")?;
            println!("{}", result);
        }
    }
    Ok(())
}

fn emit_parse_json_errors(file: &PathBuf, err: &str) {
    // Split multi-line parse error string; each line contains: "error at S..E: ..."
    let mut items = Vec::new();
    for line in err.lines() {
        if let Some((start, end)) = extract_span(line) {
            items.push(JsonErrorItem {
                code: "P001",
                stage: "parse",
                message: line.trim().to_string(),
                file: file.display().to_string(),
                start,
                end,
                function: None,
            });
        } else {
            items.push(JsonErrorItem {
                code: "P001",
                stage: "parse",
                message: line.trim().to_string(),
                file: file.display().to_string(),
                start: 0,
                end: 0,
                function: None,
            });
        }
    }
    let out = JsonError { ok: false, errors: items };
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

fn emit_type_json_error(file: &PathBuf, err_pretty: &str) {
    let (code, start, end, func_opt) = classify_type_error(err_pretty);
    emit_single_json_error(code, "type", err_pretty.trim(), file, start, end, func_opt);
}

fn emit_single_json_error(
    code: &'static str,
    stage: &'static str,
    message: &str,
    file: &PathBuf,
    start: usize,
    end: usize,
    function: Option<String>,
) {
    let item = JsonErrorItem {
        code,
        stage,
        message: message.to_string(),
        file: file.display().to_string(),
        start,
        end,
        function,
    };
    let out = JsonError { ok: false, errors: vec![item] };
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

fn extract_span(s: &str) -> Option<(usize, usize)> {
    // Look for "at S..E:" pattern
    if let Some(idx) = s.find("at ") {
        let rest = &s[idx + 3..];
        let mut parts = rest.split("..");
        if let (Some(a), Some(brest)) = (parts.next(), parts.next()) {
            let mut bchars = brest.chars();
            let mut num = String::new();
            for ch in bchars.by_ref() {
                if ch.is_ascii_digit() { num.push(ch); } else { break; }
            }
            if let (Ok(st), Ok(en)) = (a.trim().parse::<usize>(), num.parse::<usize>()) {
                return Some((st, en));
            }
        }
    }
    None
}

fn classify_type_error(s: &str) -> (&'static str, usize, usize, Option<String>) {
    // Order matters: match more specific phrases before general ones
    let code = if s.contains("unknown function") {
        "T001"
    } else if s.contains("arity mismatch") {
        "T002"
    } else if s.contains("return type mismatch") {
        // Must come before generic "type mismatch"
        "T004"
    } else if s.contains("type mismatch") {
        // Covers arg type mismatch and other generic type mismatches
        "T003"
    } else if s.contains("must be Int") {
        "T005"
    } else if s.contains("unknown variable") {
        "T006"
    } else {
        "T000"
    };
    let span = extract_span(s).unwrap_or((0, 0));
    let func = extract_function_name(s);
    (code, span.0, span.1, func)
}

fn extract_function_name(s: &str) -> Option<String> {
    // Errors may include context lines like: "in function `name`"
    if let Some(idx) = s.find("in function `") {
        let rest = &s[idx + "in function `".len()..];
        if let Some(end) = rest.find('`') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_span_parses_basic_pattern() {
        let s = "some error at 12..34: details here";
        assert_eq!(extract_span(s), Some((12, 34)));
    }

    #[test]
    fn classify_type_error_maps_codes_and_spans() {
        // unknown function → T001
        let s1 = "at 5..8: unknown function `foo`";
        let (c1, st1, en1, f1) = classify_type_error(s1);
        assert_eq!(c1, "T001");
        assert_eq!((st1, en1), (5, 8));
        assert!(f1.is_none());

        // arity mismatch with function context → T002 + function name
        let s2 = "in function `main`\nat 1..2: arity mismatch calling `add`: expected 2, found 1";
        let (c2, st2, en2, f2) = classify_type_error(s2);
        assert_eq!(c2, "T002");
        assert_eq!((st2, en2), (1, 2));
        assert_eq!(f2.as_deref(), Some("main"));

        // arg type mismatch → T003
        let s3 = "at 10..12: arg 1 type mismatch calling `f`: expected `Int`, found `String`";
        let (c3, st3, en3, _) = classify_type_error(s3);
        assert_eq!(c3, "T003");
        assert_eq!((st3, en3), (10, 12));

        // return type mismatch → T004
        let s4 = "at 20..21: return type mismatch: declared `Int`, found `Bool`";
        let (c4, st4, en4, _) = classify_type_error(s4);
        assert_eq!(c4, "T004");
        assert_eq!((st4, en4), (20, 21));

        // int operand expected → T005
        let s5 = "at 30..31: left operand must be Int, found `Bool`";
        let (c5, st5, en5, _) = classify_type_error(s5);
        assert_eq!(c5, "T005");
        assert_eq!((st5, en5), (30, 31));

        // unknown variable → T006
        let s6 = "at 40..41: unknown variable `x`";
        let (c6, st6, en6, _) = classify_type_error(s6);
        assert_eq!(c6, "T006");
        assert_eq!((st6, en6), (40, 41));

        // fallback → T000, no span
        let s7 = "unexpected other error format";
        let (c7, st7, en7, _) = classify_type_error(s7);
        assert_eq!(c7, "T000");
        assert_eq!((st7, en7), (0, 0));
    }
}
