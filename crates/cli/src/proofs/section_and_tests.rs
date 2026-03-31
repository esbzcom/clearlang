pub fn proofs_hash_from_section(section: &ProofSection) -> Result<[u8; 32]> {
    let mut entries = Vec::new();
    for func in &section.functions {
        for vc in &func.vcs {
            entries.push((func.name.clone(), vc.clone()));
        }
    }
    Ok(compute_proofs_hash_from_vcs(&entries))
}

fn overwrite_module_hash(data: &mut [u8], new_hash: &[u8; 32]) -> Result<()> {
    const KEY_BYTES: &[u8] = b"module_hash";
    const HEADER: u8 = 0x60 | (KEY_BYTES.len() as u8);
    const BYTE_STRING_MARKER: u8 = 0x58;
    const BYTE_STRING_LEN: u8 = 0x20;

    let mut i = 0;
    while i + KEY_BYTES.len() + 2 <= data.len() {
        if data[i] == HEADER && &data[i + 1..i + 1 + KEY_BYTES.len()] == KEY_BYTES {
            let value_start = i + 1 + KEY_BYTES.len();
            if value_start + 2 + new_hash.len() <= data.len()
                && data[value_start] == BYTE_STRING_MARKER
                && data[value_start + 1] == BYTE_STRING_LEN
            {
                let bytes_start = value_start + 2;
                data[bytes_start..bytes_start + new_hash.len()].copy_from_slice(new_hash);
                return Ok(());
            }
        }
        i += 1;
    }
    Err(anyhow!("module hash field not found in proof section"))
}

pub fn module_bytes_with_zeroed_hash(bytes: &[u8]) -> Result<Vec<u8>> {
    const ZERO: [u8; 32] = [0u8; 32];
    rewrite_module_hash(bytes, &ZERO)
}

fn rewrite_module_hash(bytes: &[u8], new_hash: &[u8; 32]) -> Result<Vec<u8>> {
    let mut rewritten = bytes.to_vec();
    for payload in Parser::new(0).parse_all(bytes) {
        if let Payload::CustomSection(section) = payload? {
            if section.name() == "clearlang.proof" {
                let data_len = section.data().len();
                let range = section.range();
                let data_start = range.end - data_len;
                let data_end = range.end;
                overwrite_module_hash(&mut rewritten[data_start..data_end], new_hash)?;
                return Ok(rewritten);
            }
        }
    }
    Err(anyhow!("clearlang.proof section not found"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clg_ast::{Contract, Effect, Expr, Func, Program, Span, Type};
    use clg_typer::{AssumptionBoundary, AssumptionCategory, ContractExpr, VerificationCondition};

    fn sample_vc(status: &'static str) -> VerificationCondition {
        VerificationCondition {
            function: "main".to_string(),
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
            status,
            refinements: Vec::new(),
            assumptions: Vec::new(),
        }
    }

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn call_expr(callee: &str) -> Expr {
        Expr::Call {
            callee: callee.to_string(),
            type_args: Vec::new(),
            args: vec![Expr::String("x".to_string(), span())],
            span: span(),
        }
    }

    fn sample_program_with_contract_calls() -> Program {
        Program {
            module: None,
            imports: Vec::new(),
            refined_aliases: Vec::new(),
            resources: Vec::new(),
            structs: Vec::new(),
            enums: Vec::new(),
            traits: Vec::new(),
            impls: Vec::new(),
            funcs: vec![Func {
                is_exported: false,
                effect: Effect::Pure,
                effect_span: None,
                name: "main".to_string(),
                type_params: Vec::new(),
                params: Vec::new(),
                ret: Type::Int,
                where_bounds: Vec::new(),
                requires: vec![Contract {
                    span: span(),
                    expr: call_expr("std::str::len"),
                }],
                ensures: vec![Contract {
                    span: span(),
                    expr: call_expr("std::bytes::eq_ct"),
                }],
                body: Expr::Int(0, span()),
            }],
        }
    }

    #[test]
    fn proof_status_is_proved_all_only_for_strict_all_proved_without_assumptions() {
        let vcs = vec![sample_vc("proved"), sample_vc("proved")];
        assert_eq!(
            proof_status_for_vcs(&vcs, "strict"),
            PROOF_STATUS_PROVED_ALL
        );
    }

    #[test]
    fn proof_status_is_not_proved_all_when_any_vc_not_proved() {
        let vcs = vec![sample_vc("proved"), sample_vc("generated")];
        assert_eq!(
            proof_status_for_vcs(&vcs, "strict"),
            PROOF_STATUS_NOT_PROVED_ALL
        );
    }

    #[test]
    fn proof_status_is_not_proved_all_when_assumptions_exist_or_not_strict() {
        let mut vc = sample_vc("proved");
        vc.assumptions.push(AssumptionBoundary {
            id: "crypto.uninterpreted",
            category: AssumptionCategory::Crypto,
            status: "assumed",
            message: "crypto modeled as uninterpreted",
            symbols: vec!["std::crypto::hash".to_string()],
        });
        assert_eq!(
            proof_status_for_vcs(&[vc], "strict"),
            PROOF_STATUS_NOT_PROVED_ALL
        );

        let vcs = vec![sample_vc("proved")];
        assert_eq!(
            proof_status_for_vcs(&vcs, "standard"),
            PROOF_STATUS_NOT_PROVED_ALL
        );
    }

    #[test]
    fn bundle_symbols_include_contract_requires_and_ensures() {
        let program = sample_program_with_contract_calls();
        let symbols = bundle_symbols_for_program(&program);
        assert!(
            symbols.contains(&"std::str::len".to_string()),
            "bundle symbols should include require-clause std calls"
        );
        assert!(
            symbols.contains(&"std::bytes::eq_ct".to_string()),
            "bundle symbols should include ensure-clause std calls"
        );
    }
}
