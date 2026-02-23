use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clg_ast::{Effect, Program, Type};
use clg_codegen_wasm::{emit_from_ir_with_opts, CodegenOpts, ExportAlias, ExternalImport};
use clg_ir::IrType;
use clg_typer::{
    check_with_vcs_with_std_and_external, AssumptionBoundary, AssumptionCategory,
    ExternalBuiltinSig, RefinementAttachmentDetail, TypecheckOutput, TyperError,
    VerificationCondition,
};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::commands::helpers::{extract_function_name, make_single_json_error, CommandError};
use crate::commands::modules::load_program;
use crate::logging::{LogLevel, Logger, StageTimings};
use crate::proofs::{
    assurance_for_assumptions, hash_module, module_bytes_with_zeroed_hash, ProofPackage,
};
use crate::signing::{self, SignScope};

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum CompilerMode {
    /// Development mode: VC strictness defaults to false unless explicitly enabled.
    Permissive,
    /// Current default behavior: VC strictness defaults to true.
    Standard,
    /// Production mode: requires --emit-vcs and enforces VC strictness.
    Strict,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    file: PathBuf,
    out: PathBuf,
    contract: bool,
    validate: bool,
    debug_names: bool,
    emit_vcs: Option<PathBuf>,
    compiler_mode: CompilerMode,
    proof_strict: Option<bool>,
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
    let loaded = {
        let _stage = timings.start(logger, "parse");
        load_program(&file, json_errors)?
    };
    let ast = &loaded.program;
    let external_typer_sigs: Vec<ExternalBuiltinSig> = loaded
        .external_imports
        .iter()
        .map(|binding| ExternalBuiltinSig {
            name: binding.function.clone(),
            params: binding.params.clone(),
            ret: binding.ret.clone(),
            effect: binding.effect,
        })
        .collect();
    let external_codegen_imports: Vec<ExternalImport> = loaded
        .external_imports
        .iter()
        .map(|binding| ExternalImport {
            function: binding.function.clone(),
            import_module: binding.import_module.clone(),
            import_name: binding.import_name.clone(),
        })
        .collect();

    let type_output = {
        let _stage = timings.start(logger, "typecheck");
        match check_with_vcs_with_std_and_external(ast, &loaded.std_types, &external_typer_sigs) {
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
        mangled_name_origins,
    } = type_output;

    let fail_build = |code: &'static str, message: &str, function: Option<String>| -> Result<()> {
        if json_errors {
            let json = make_single_json_error(code, "build", message, &file, 0, 0, function);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(message.to_string()))
        }
    };

    if compiler_mode == CompilerMode::Strict && emit_vcs.is_none() {
        fail_build(
            "C029",
            "`--compiler-mode strict` requires `--emit-vcs <FILE>`",
            None,
        )?;
    }
    let proof_strict_enabled = match proof_strict_for_mode(compiler_mode, proof_strict) {
        Ok(value) => value,
        Err(message) => {
            fail_build("C030", message, None)?;
            false
        }
    };

    if proof_strict_enabled && emit_vcs.is_some() {
        if let Some(message) = strict_proof_violation(&vcs) {
            fail_build("C014", &message, None)?;
        }
    }

    let export_aliases = if contract {
        contract_exports(ast, &file, json_errors)?
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
    let proof_package = emit_vcs.as_ref().map(|_| {
        ProofPackage::from_program(
            &mono_program,
            &vcs,
            &mangled_name_origins,
            toolchain.clone(),
        )
    });

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
                external_imports: external_codegen_imports.clone(),
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
                    external_imports: external_codegen_imports.clone(),
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
        write_vcs_json(&vcs, &mangled_name_origins, &vcs_path, &file)?;
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

fn write_vcs_json(
    vcs: &[VerificationCondition],
    mangled_name_origins: &HashMap<String, String>,
    path: &Path,
    src: &Path,
) -> Result<()> {
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

    fn assumptions_json(assumptions: &[AssumptionBoundary]) -> serde_json::Value {
        use serde_json::json;
        let items: Vec<serde_json::Value> = assumptions
            .iter()
            .map(|assumption| {
                json!({
                    "id": assumption.id,
                    "category": assumption.category.as_str(),
                    "status": assumption.status,
                    "message": assumption.message,
                    "symbols": assumption.symbols,
                })
            })
            .collect();
        json!({ "items": items })
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
        if let Some(canonical) = canonical_name_for(&vc.function, mangled_name_origins) {
            obj.insert("canonical_function".to_string(), json!(canonical));
        }
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
        obj.insert(
            "assurance".to_string(),
            serde_json::to_value(assurance_for_assumptions(&vc.assumptions))?,
        );
        if !vc.assumptions.is_empty() {
            obj.insert("assumptions".to_string(), assumptions_json(&vc.assumptions));
        }
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

fn canonical_name_for(
    emitted_name: &str,
    mangled_name_origins: &HashMap<String, String>,
) -> Option<String> {
    let canonical = mangled_name_origins.get(emitted_name)?;
    if canonical == emitted_name {
        return None;
    }
    Some(canonical.clone())
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

const ASSUMPTION_UNSIGNED_ID: &str = "unsigned.int_model";
const ASSUMPTION_BITWISE_ID: &str = "bitwise.uninterpreted";
const ASSUMPTION_CRYPTO_ID: &str = "crypto.uninterpreted";
const ASSUMPTION_PRIMITIVE_ID: &str = "primitive.unproved";
const ASSUMPTION_EXTERNAL_ID: &str = "external.dependency";

fn proof_strict_for_mode(mode: CompilerMode, value: Option<bool>) -> Result<bool, &'static str> {
    match mode {
        CompilerMode::Permissive => Ok(value.unwrap_or(false)),
        CompilerMode::Standard => Ok(value.unwrap_or(true)),
        CompilerMode::Strict => match value {
            Some(false) => {
                Err("`--compiler-mode strict` cannot be combined with `--proof-strict=false`")
            }
            _ => Ok(true),
        },
    }
}

fn strict_proof_violation(vcs: &[VerificationCondition]) -> Option<String> {
    for vc in vcs {
        let mut seen_ids = std::collections::BTreeSet::new();
        for assumption in &vc.assumptions {
            if !seen_ids.insert(assumption.id) {
                return Some(format!(
                    "strict proof mode: VC `{}` in function `{}` has duplicate assumption boundary `{}`",
                    vc.vc_id, vc.function, assumption.id
                ));
            }
            let Some(expected_category) = category_for_assumption_id(assumption.id) else {
                return Some(format!(
                    "strict proof mode: VC `{}` in function `{}` has unknown assumption boundary `{}`",
                    vc.vc_id, vc.function, assumption.id
                ));
            };
            if assumption.category != expected_category {
                return Some(format!(
                    "strict proof mode: VC `{}` in function `{}` has mismatched category `{}` for assumption `{}`",
                    vc.vc_id,
                    vc.function,
                    assumption.category.as_str(),
                    assumption.id
                ));
            }
            if assumption.status != "assumed" {
                return Some(format!(
                    "strict proof mode: VC `{}` in function `{}` has invalid status `{}` for assumption `{}`; expected `assumed`",
                    vc.vc_id, vc.function, assumption.status, assumption.id
                ));
            }
        }
    }
    None
}

fn category_for_assumption_id(id: &str) -> Option<AssumptionCategory> {
    match id {
        ASSUMPTION_UNSIGNED_ID => Some(AssumptionCategory::Unsigned),
        ASSUMPTION_BITWISE_ID => Some(AssumptionCategory::Bitwise),
        ASSUMPTION_CRYPTO_ID => Some(AssumptionCategory::Crypto),
        ASSUMPTION_PRIMITIVE_ID => Some(AssumptionCategory::Primitive),
        ASSUMPTION_EXTERNAL_ID => Some(AssumptionCategory::External),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clg_ast::Span;
    use clg_typer::{AssumptionCategory, ContractExpr, VerificationCondition};

    fn sample_vc() -> VerificationCondition {
        VerificationCondition {
            function: "f".to_string(),
            vc_id: "vc:0".to_string(),
            pre: ContractExpr {
                ast: "true".to_string(),
                smt2: "true".to_string(),
                span: Some(Span { start: 0, end: 0 }),
            },
            post: ContractExpr {
                ast: "true".to_string(),
                smt2: "true".to_string(),
                span: Some(Span { start: 0, end: 0 }),
            },
            vc_smt2: "(=> true true)".to_string(),
            status: "generated",
            refinements: Vec::new(),
            assumptions: Vec::new(),
        }
    }

    fn assumption(id: &'static str, category: AssumptionCategory) -> AssumptionBoundary {
        AssumptionBoundary {
            id,
            category,
            status: "assumed",
            message: "m",
            symbols: Vec::new(),
        }
    }

    #[test]
    fn strict_mode_accepts_vc_without_assumptions() {
        let mut vc = sample_vc();
        vc.vc_smt2 = "(=> true true)".to_string();
        assert!(strict_proof_violation(&[vc]).is_none());
    }

    #[test]
    fn strict_mode_accepts_valid_assumptions() {
        let mut vc = sample_vc();
        vc.vc_smt2 = "(=> true true)".to_string();
        vc.assumptions = vec![
            assumption(super::ASSUMPTION_UNSIGNED_ID, AssumptionCategory::Unsigned),
            assumption(super::ASSUMPTION_BITWISE_ID, AssumptionCategory::Bitwise),
            assumption(super::ASSUMPTION_CRYPTO_ID, AssumptionCategory::Crypto),
            assumption(
                super::ASSUMPTION_PRIMITIVE_ID,
                AssumptionCategory::Primitive,
            ),
            assumption(super::ASSUMPTION_EXTERNAL_ID, AssumptionCategory::External),
        ];
        assert!(
            strict_proof_violation(&[vc]).is_none(),
            "expected strict checks to pass"
        );
    }

    #[test]
    fn strict_mode_rejects_invalid_assumption_status() {
        let mut vc = sample_vc();
        vc.assumptions = vec![AssumptionBoundary {
            id: super::ASSUMPTION_UNSIGNED_ID,
            category: AssumptionCategory::Unsigned,
            status: "proved",
            message: "m",
            symbols: vec!["U64".to_string()],
        }];
        let msg = strict_proof_violation(&[vc]).expect("expected strict violation");
        assert!(msg.contains("invalid status"));
    }

    #[test]
    fn strict_mode_rejects_unknown_assumption_boundary() {
        let mut vc = sample_vc();
        vc.assumptions = vec![assumption(
            "unknown.assumption",
            AssumptionCategory::Bitwise,
        )];
        let msg = strict_proof_violation(&[vc]).expect("expected strict violation");
        assert!(msg.contains("unknown assumption boundary"));
    }

    #[test]
    fn strict_mode_rejects_mismatched_assumption_category() {
        let mut vc = sample_vc();
        vc.assumptions = vec![assumption(
            super::ASSUMPTION_BITWISE_ID,
            AssumptionCategory::Crypto,
        )];
        let msg = strict_proof_violation(&[vc]).expect("expected strict violation");
        assert!(msg.contains("mismatched category"));
    }

    #[test]
    fn compiler_mode_permissive_defaults_proof_strict_false() {
        assert_eq!(
            proof_strict_for_mode(CompilerMode::Permissive, None).expect("mode"),
            false
        );
    }

    #[test]
    fn compiler_mode_standard_defaults_proof_strict_true() {
        assert_eq!(
            proof_strict_for_mode(CompilerMode::Standard, None).expect("mode"),
            true
        );
    }

    #[test]
    fn compiler_mode_strict_forces_proof_strict_true() {
        assert_eq!(
            proof_strict_for_mode(CompilerMode::Strict, None).expect("mode"),
            true
        );
        assert_eq!(
            proof_strict_for_mode(CompilerMode::Strict, Some(true)).expect("mode"),
            true
        );
    }

    #[test]
    fn compiler_mode_strict_rejects_proof_strict_false_override() {
        assert!(
            proof_strict_for_mode(CompilerMode::Strict, Some(false)).is_err(),
            "strict mode should reject false override"
        );
    }
}
