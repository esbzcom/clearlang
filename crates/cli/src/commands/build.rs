use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clg_ast::{Effect, Program, Type};
use clg_codegen_wasm::{emit_from_ir_with_opts, CodegenOpts, ExportAlias};
use clg_ir::IrType;
use clg_typer::{
    check_with_vcs, RefinementAttachmentDetail, TypecheckOutput, TyperError, VerificationCondition,
};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::commands::helpers::{extract_function_name, make_single_json_error, CommandError};
use crate::commands::modules::load_program;
use crate::logging::{LogLevel, Logger, StageTimings};
use crate::proofs::{hash_module, module_bytes_with_zeroed_hash, ProofPackage};
use crate::signing::{self, SignScope};

#[allow(clippy::too_many_arguments)]
pub fn run(
    file: PathBuf,
    out: PathBuf,
    contract: bool,
    validate: bool,
    debug_names: bool,
    emit_vcs: Option<PathBuf>,
    sign: bool,
    key: Option<PathBuf>,
    key_id: Option<String>,
    scope: SignScope,
    sig_out: Option<PathBuf>,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    if sign && emit_vcs.is_none() {
        anyhow::bail!("--sign requires --emit-vcs");
    }

    let mut timings = StageTimings::new();
    let ast = {
        let _stage = timings.start(logger, "parse");
        load_program(&file, json_errors)?
    };

    let type_output = {
        let _stage = timings.start(logger, "typecheck");
        match check_with_vcs(&ast) {
            Ok(result) => result,
            Err(e) => {
                if json_errors {
                    if let Some((typer, function)) = find_typer_error(&e) {
                        let json = make_single_json_error(
                            typer.code,
                            "type",
                            typer.message.clone(),
                            &file,
                            typer.start,
                            typer.end,
                            function,
                        );
                        return Err(CommandError::json(json).into());
                    }
                    let json =
                        make_single_json_error("T000", "type", format!("{e:#}"), &file, 0, 0, None);
                    return Err(CommandError::json(json).into());
                } else {
                    return Err(e.context("type-check failed"));
                }
            }
        }
    };

    let TypecheckOutput {
        ir,
        vcs,
        mono_program,
    } = type_output;

    let fail_build = |code: &'static str, message: &str, function: Option<String>| -> Result<()> {
        if json_errors {
            let json = make_single_json_error(code, "build", message, &file, 0, 0, function);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(message.to_string()))
        }
    };

    let export_aliases = if contract {
        contract_exports(&ast, &file, json_errors)?
    } else {
        match ir.funcs.iter().find(|f| f.name == "main") {
            Some(f) => {
                if f.ret != Some(IrType::Int) || !f.params.is_empty() {
                    fail_build(
                        "C001",
                        "only `main() -> Int` is supported in this phase",
                        Some("main".to_string()),
                    )?;
                }
            }
            None => {
                fail_build("C002", "missing `main` function", None)?;
            }
        }
        Vec::new()
    };

    let toolchain = format!("clg-cli/{}", env!("CARGO_PKG_VERSION"));
    let proof_package = emit_vcs
        .as_ref()
        .map(|_| ProofPackage::from_program(&mono_program, &vcs, toolchain.clone()));

    let zero_section = proof_package
        .as_ref()
        .map(|pkg| pkg.encode_section(&[0u8; 32]))
        .transpose()?;

    let (wasm_bytes, module_hash_bytes) = {
        let _stage = timings.start(logger, "codegen");
        let mut wasm_bytes = emit_from_ir_with_opts(
            &ir,
            CodegenOpts {
                debug_names,
                proof_section: zero_section.clone(),
                export_aliases: export_aliases.clone(),
            },
        )
        .context("codegen (IR+Wasm) failed")?;

        let mut module_hash_bytes: Option<[u8; 32]> = None;
        if let Some(pkg) = &proof_package {
            let hash_bytes = hash_module(&wasm_bytes);
            let proof_section = pkg.encode_section(&hash_bytes)?;
            wasm_bytes = emit_from_ir_with_opts(
                &ir,
                CodegenOpts {
                    debug_names,
                    proof_section: Some(proof_section),
                    export_aliases: export_aliases.clone(),
                },
            )
            .context("codegen (IR+Wasm) failed")?;
            let zeroed = module_bytes_with_zeroed_hash(&wasm_bytes)?;
            module_hash_bytes = Some(hash_module(&zeroed));
        }
        (wasm_bytes, module_hash_bytes)
    };

    {
        let _stage = timings.start(logger, "write_wasm");
        if let Some(parent) = out.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
        }
        fs::write(&out, &wasm_bytes).with_context(|| format!("writing {}", out.display()))?;
    }

    if let Some(vcs_path) = emit_vcs {
        let _stage = timings.start(logger, "emit_vcs");
        write_vcs_json(&vcs, &vcs_path, &file)?;
    }

    if sign {
        let _stage = timings.start(logger, "sign");
        let pkg = proof_package
            .as_ref()
            .ok_or_else(|| anyhow!("proof data unavailable for signing"))?;
        let hash_bytes =
            module_hash_bytes.ok_or_else(|| anyhow!("module hash unavailable for signing"))?;
        let key_path = key.expect("clap ensures key when sign");
        let key_id = key_id.expect("clap ensures key_id when sign");
        let sig_path = sig_out.expect("clap ensures sig_out when sign");
        let module_hash_hex = hex::encode(hash_bytes);
        let timestamp = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
        signing::sign_bundle(
            pkg,
            &module_hash_hex,
            scope,
            &key_path,
            &key_id,
            &sig_path,
            &timestamp,
        )?;
        if logger.enabled(LogLevel::Debug) {
            logger.event(
                LogLevel::Debug,
                "sign_detail",
                "sign",
                &[
                    ("module_hash", module_hash_hex.clone()),
                    ("proofs_hash", pkg.proofs_hash_hex()),
                    ("sig_path", sig_path.display().to_string()),
                ],
            );
        }
    }

    if validate {
        let _stage = timings.start(logger, "validate_wasm");
        let status = std::process::Command::new("wasm-tools")
            .arg("validate")
            .arg(&out)
            .status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => anyhow::bail!("wasm-tools validate failed with status {:?}", s.code()),
            Err(e) => anyhow::bail!("failed to run wasm-tools: {}", e),
        }
    }

    logger.summary(&timings);
    Ok(())
}

fn write_vcs_json(vcs: &[VerificationCondition], path: &Path, src: &Path) -> Result<()> {
    use serde_json::json;

    fn refinement_attachment_json(att: &clg_typer::RefinementAttachment) -> serde_json::Value {
        match &att.detail {
            RefinementAttachmentDetail::Param { param } => json!({
                "kind": "param",
                "detail": { "param": param },
            }),
            RefinementAttachmentDetail::Return { result } => json!({
                "kind": "return",
                "detail": { "result": result },
            }),
            RefinementAttachmentDetail::Flow(flow) => {
                let mut detail = serde_json::Map::new();
                detail.insert("flow_kind".to_string(), json!(flow.flow_kind.as_str()));
                if let Some(name) = &flow.name {
                    detail.insert("name".to_string(), json!(name));
                }
                if let Some(callee) = &flow.callee {
                    detail.insert("callee".to_string(), json!(callee));
                }
                if let Some(arg_index) = flow.arg_index {
                    detail.insert("arg_index".to_string(), json!(arg_index));
                }
                if let Some(variant) = &flow.variant {
                    detail.insert("variant".to_string(), json!(variant));
                }
                if let Some(arm) = flow.arm {
                    detail.insert("arm".to_string(), json!(arm));
                }
                json!({
                    "kind": "flow",
                    "detail": serde_json::Value::Object(detail),
                })
            }
        }
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }

    let file_str = src.to_string_lossy();
    let mut items = Vec::with_capacity(vcs.len());
    for vc in vcs {
        let positions = match (vc.pre.span, vc.post.span) {
            (None, None) => None,
            (pre, post) => Some(json!({
                "file": file_str,
                "pre_start": pre.map(|s| s.start).unwrap_or(0),
                "pre_end": pre.map(|s| s.end).unwrap_or(0),
                "post_start": post.map(|s| s.start).unwrap_or(0),
                "post_end": post.map(|s| s.end).unwrap_or(0),
            })),
        };
        let mut obj = serde_json::Map::new();
        obj.insert("version".to_string(), json!(2));
        obj.insert("function".to_string(), json!(vc.function));
        obj.insert("vc_id".to_string(), json!(vc.vc_id));
        obj.insert(
            "pre".to_string(),
            json!({ "ast": vc.pre.ast, "smt2": vc.pre.smt2 }),
        );
        obj.insert(
            "post".to_string(),
            json!({ "ast": vc.post.ast, "smt2": vc.post.smt2 }),
        );
        obj.insert("vc".to_string(), json!({ "smt2": vc.vc_smt2 }));
        obj.insert("status".to_string(), json!(vc.status));
        if let Some(pos) = positions {
            obj.insert("positions".to_string(), pos);
        }
        if !vc.refinements.is_empty() {
            let mut premises = Vec::with_capacity(vc.refinements.len());
            for (idx, premise) in vc.refinements.iter().enumerate() {
                let mut prem = serde_json::Map::new();
                prem.insert("id".to_string(), json!(format!("ref:{}", idx)));
                prem.insert("alias".to_string(), json!(premise.alias.as_str()));
                prem.insert("binder".to_string(), json!(premise.binder.as_str()));
                prem.insert(
                    "substitution".to_string(),
                    json!({
                        "ast": premise.substitution.ast.as_str(),
                        "smt2": premise.substitution.smt2.as_str(),
                    }),
                );
                prem.insert(
                    "predicate".to_string(),
                    json!({
                        "ast": premise.predicate.ast.as_str(),
                        "smt2": premise.predicate.smt2.as_str(),
                    }),
                );
                prem.insert(
                    "attachment".to_string(),
                    refinement_attachment_json(&premise.attachment),
                );
                premises.push(serde_json::Value::Object(prem));
            }
            obj.insert("refinements".to_string(), json!({ "premises": premises }));
        }
        items.push(serde_json::Value::Object(obj));
    }
    let data = serde_json::to_vec_pretty(&serde_json::Value::Array(items))?;
    fs::write(path, data).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn contract_exports(ast: &Program, file: &Path, json_errors: bool) -> Result<Vec<ExportAlias>> {
    let fail = |code: &'static str, message: &str, function: Option<String>| -> Result<()> {
        if json_errors {
            let json = make_single_json_error(code, "build", message, file, 0, 0, function);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(message.to_string()))
        }
    };

    if ast
        .funcs
        .iter()
        .any(|f| f.name == "init" || f.name == "handle")
    {
        fail(
            "C013",
            "contract build reserves `init` and `handle`; define `apply` instead",
            None,
        )?;
    }

    let apply = ast.funcs.iter().find(|f| f.name == "apply");
    let query = ast.funcs.iter().find(|f| f.name == "query");
    let Some(apply_fn) = apply else {
        fail("C010", "missing `apply` function for contract build", None)?;
        return Ok(Vec::new());
    };
    let Some(query_fn) = query else {
        fail("C011", "missing `query` function for contract build", None)?;
        return Ok(Vec::new());
    };

    let expect = "pure function apply(state: Bytes, msg: Bytes) -> Bytes";
    if !matches!(apply_fn.effect, Effect::None | Effect::Pure)
        || apply_fn.params.len() != 2
        || apply_fn.params[0].ty != Type::Bytes
        || apply_fn.params[1].ty != Type::Bytes
        || apply_fn.ret != Type::Bytes
    {
        fail(
            "C012",
            &format!("`apply` must have signature `{}`", expect),
            Some("apply".to_string()),
        )?;
    }

    let expect = "pure function query(state: Bytes, msg: Bytes) -> Bytes";
    if !matches!(query_fn.effect, Effect::None | Effect::Pure)
        || query_fn.params.len() != 2
        || query_fn.params[0].ty != Type::Bytes
        || query_fn.params[1].ty != Type::Bytes
        || query_fn.ret != Type::Bytes
    {
        fail(
            "C012",
            &format!("`query` must have signature `{}`", expect),
            Some("query".to_string()),
        )?;
    }

    Ok(vec![
        ExportAlias {
            export: "init".to_string(),
            target: "apply".to_string(),
        },
        ExportAlias {
            export: "handle".to_string(),
            target: "apply".to_string(),
        },
        ExportAlias {
            export: "query".to_string(),
            target: "query".to_string(),
        },
    ])
}

fn find_typer_error(err: &anyhow::Error) -> Option<(&TyperError, Option<String>)> {
    let mut function: Option<String> = None;
    for cause in err.chain() {
        if function.is_none() {
            let msg = cause.to_string();
            if let Some(name) = extract_function_name(&msg) {
                function = Some(name);
            }
        }
        if let Some(typer) = cause.downcast_ref::<TyperError>() {
            return Some((typer, function));
        }
    }
    None
}
