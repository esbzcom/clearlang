fn collect_bundle_symbols_from_block(block: &clg_ast::Block, out: &mut BTreeSet<String>) {
    for stmt in &block.statements {
        match stmt {
            clg_ast::Stmt::Let { expr, .. } => collect_bundle_symbols_from_expr(expr, out),
            clg_ast::Stmt::Expr { expr, .. } => collect_bundle_symbols_from_expr(expr, out),
            clg_ast::Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                collect_bundle_symbols_from_expr(cond, out);
                collect_bundle_symbols_from_expr(invariant, out);
                if let Some(variant_expr) = variant {
                    collect_bundle_symbols_from_expr(variant_expr, out);
                }
                collect_bundle_symbols_from_block(body, out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_bundle_symbols_from_expr(tail, out);
    }
}

fn collect_bundle_symbols_from_expr(expr: &clg_ast::Expr, out: &mut BTreeSet<String>) {
    use clg_ast::Expr;
    match expr {
        Expr::Call { callee, args, .. } => {
            if callee.starts_with("std::") {
                out.insert(callee.clone());
            }
            for arg in args {
                collect_bundle_symbols_from_expr(arg, out);
            }
        }
        Expr::Bin { lhs, rhs, .. } => {
            collect_bundle_symbols_from_expr(lhs, out);
            collect_bundle_symbols_from_expr(rhs, out);
        }
        Expr::Unary { expr, .. } | Expr::Return { expr, .. } | Expr::Try { expr, .. } => {
            collect_bundle_symbols_from_expr(expr, out);
        }
        Expr::Index { base, index, .. } => {
            collect_bundle_symbols_from_expr(base, out);
            collect_bundle_symbols_from_expr(index, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_bundle_symbols_from_expr(scrutinee, out);
            for arm in arms {
                collect_bundle_symbols_from_expr(&arm.expr, out);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_bundle_symbols_from_expr(cond, out);
            collect_bundle_symbols_from_expr(then_br, out);
            collect_bundle_symbols_from_expr(else_br, out);
        }
        Expr::Block { block } => collect_bundle_symbols_from_block(block, out),
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                collect_bundle_symbols_from_expr(elem, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_bundle_symbols_from_expr(&field.expr, out);
            }
        }
        Expr::FieldAccess { base, .. } => collect_bundle_symbols_from_expr(base, out),
        Expr::Lambda { body, .. } => collect_bundle_symbols_from_expr(body, out),
        Expr::Int(..) | Expr::Bool(..) | Expr::String(..) | Expr::Var(..) => {}
    }
}

pub struct AssuranceManifestInput<'a> {
    pub vcs: &'a [VerificationCondition],
    pub toolchain: &'a str,
    pub compiler_mode: &'a str,
    pub proof_strict: bool,
    pub module_hash_hex: &'a str,
    pub proofs_hash_hex: &'a str,
    pub generated_at: &'a str,
    pub proof_artifact_hash: Option<&'a str>,
    pub solver_profile_hash: Option<&'a str>,
}

pub fn build_assurance_manifest_payload(input: AssuranceManifestInput<'_>) -> serde_json::Value {
    use serde_json::json;
    let AssuranceManifestInput {
        vcs,
        toolchain,
        compiler_mode,
        proof_strict,
        module_hash_hex,
        proofs_hash_hex,
        generated_at,
        proof_artifact_hash,
        solver_profile_hash,
    } = input;

    #[derive(Default)]
    struct AssumptionAggregate {
        symbols: BTreeSet<String>,
        vc_refs: BTreeSet<(String, String)>,
        intrinsic_levels: BTreeSet<String>,
    }

    let mut ordered_vcs: Vec<&VerificationCondition> = vcs.iter().collect();
    ordered_vcs.sort_by(|left, right| {
        left.function
            .cmp(&right.function)
            .then_with(|| left.vc_id.cmp(&right.vc_id))
    });

    let mut assumptions: BTreeMap<(String, String, String, String), AssumptionAggregate> =
        BTreeMap::new();
    let mut dependency_trust: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut assumption_count = 0usize;
    let assurance_claim = assurance_claim_for_compiler_mode(compiler_mode);
    let proof_status = proof_status_for_vcs(vcs, compiler_mode);

    for vc in ordered_vcs {
        for boundary in &vc.assumptions {
            assumption_count += 1;
            let key = (
                boundary.id.to_string(),
                boundary.category.as_str().to_string(),
                boundary.status.to_string(),
                boundary.message.to_string(),
            );
            let aggregate = assumptions.entry(key).or_default();
            aggregate
                .vc_refs
                .insert((vc.function.clone(), vc.vc_id.clone()));
            for symbol in &boundary.symbols {
                if !symbol.trim().is_empty() {
                    aggregate.symbols.insert(symbol.clone());
                }
            }
            for level in intrinsic_levels_for_boundary(boundary) {
                aggregate.intrinsic_levels.insert(level.intrinsic);
            }

            let dependency_kind = match boundary.id {
                ASSUMPTION_PRIMITIVE_ID => Some("primitive"),
                ASSUMPTION_EXTERNAL_ID => Some("external"),
                _ => None,
            };
            if let Some(kind) = dependency_kind {
                for symbol in &boundary.symbols {
                    if symbol.trim().is_empty() {
                        continue;
                    }
                    dependency_trust
                        .entry((symbol.clone(), kind.to_string()))
                        .or_insert_with(|| boundary.status.to_string());
                }
            }
        }
    }

    let assumption_items: Vec<serde_json::Value> = assumptions
        .into_iter()
        .map(|((id, category, status, message), aggregate)| {
            let symbols: Vec<String> = aggregate.symbols.into_iter().collect();
            let vc_refs: Vec<serde_json::Value> = aggregate
                .vc_refs
                .into_iter()
                .map(|(function, vc_id)| json!({ "function": function, "vc_id": vc_id }))
                .collect();
            let intrinsic_levels: Vec<serde_json::Value> = aggregate
                .intrinsic_levels
                .into_iter()
                .map(|intrinsic| {
                    json!({
                        "intrinsic": intrinsic,
                        "tier": ASSURANCE_TIER_L0,
                        "label": assurance_label_for_tier(ASSURANCE_TIER_L0),
                    })
                })
                .collect();
            let mut item = serde_json::Map::new();
            item.insert("id".to_string(), json!(id));
            item.insert("category".to_string(), json!(category));
            item.insert("status".to_string(), json!(status));
            item.insert("message".to_string(), json!(message));
            item.insert("symbols".to_string(), json!(symbols));
            item.insert("vc_refs".to_string(), json!(vc_refs));
            if !intrinsic_levels.is_empty() {
                item.insert("intrinsic_levels".to_string(), json!(intrinsic_levels));
            }
            serde_json::Value::Object(item)
        })
        .collect();

    let dependency_trust_labels: Vec<serde_json::Value> = dependency_trust
        .into_iter()
        .map(|((dependency, kind), label)| {
            json!({
                "dependency": dependency,
                "kind": kind,
                "label": label,
            })
        })
        .collect();

    let mut artifacts = serde_json::Map::new();
    artifacts.insert("module_hash".to_string(), json!(module_hash_hex));
    artifacts.insert("proofs_hash".to_string(), json!(proofs_hash_hex));
    if let Some(hash) = proof_artifact_hash {
        artifacts.insert("proof_artifact_hash".to_string(), json!(hash));
    }
    if let Some(hash) = solver_profile_hash {
        artifacts.insert("solver_profile_hash".to_string(), json!(hash));
    }

    json!({
        "format": "clg.assurance_manifest.v1",
        "generated_at": generated_at,
        "toolchain": {
            "name": toolchain,
            "fingerprint_sha256": sha256_hex(toolchain.as_bytes()),
        },
        "build": {
            "compiler_mode": compiler_mode,
            "proof_strict": proof_strict,
        },
        "assurance_claim": assurance_claim,
        "proof_status": proof_status,
        "artifacts": serde_json::Value::Object(artifacts),
        "assurance": assurance_for_vcs(vcs),
        "assumptions": {
            "total": assumption_count,
            "items": assumption_items,
        },
        "dependency_trust_labels": dependency_trust_labels,
    })
}

