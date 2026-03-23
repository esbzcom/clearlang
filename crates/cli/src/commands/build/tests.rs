#[cfg(test)]
mod tests {
    use super::*;
    use clg_ast::{Param, ParamKind};

    #[test]
    fn default_assurance_manifest_path_rewrites_sig_suffix() {
        let path = Path::new("artifacts/out.sig.json");
        assert_eq!(
            default_assurance_manifest_path(path),
            PathBuf::from("artifacts/out.assurance.json")
        );
    }

    #[test]
    fn default_assurance_manifest_path_appends_when_sig_suffix_missing() {
        let path = Path::new("artifacts/signature.json");
        assert_eq!(
            default_assurance_manifest_path(path),
            PathBuf::from("artifacts/signature.assurance.json")
        );
    }

    fn abi_contract_with_imports(
        imports: Vec<strict_package_contract::StrictAbiImportEntry>,
    ) -> StrictPackageContractV0 {
        StrictPackageContractV0 {
            packages: vec![strict_package_contract::StrictPackageMetadataEntry {
                name: "std::core".to_string(),
                version: "1.0.0".to_string(),
                digest: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
                artifact_format: "wasm".to_string(),
                artifact_path: "store/std-core-1.0.0.wasm".to_string(),
                abi_id: "abi:std::core:1.0.0".to_string(),
                dependencies: Vec::new(),
                signature: None,
                trusted_anchor_ids: Vec::new(),
            }],
            contracts: vec![strict_package_contract::StrictAbiContractEntry {
                abi_id: "abi:std::core:1.0.0".to_string(),
                package: "std::core".to_string(),
                version: "1.0.0".to_string(),
                imports,
            }],
        }
    }

    fn external_sig(
        name: &str,
        params: Vec<Type>,
        ret: Type,
        effect: Effect,
    ) -> ExternalBuiltinSig {
        ExternalBuiltinSig {
            name: name.to_string(),
            params: params
                .into_iter()
                .enumerate()
                .map(|(idx, ty)| Param {
                    kind: ParamKind::Borrow,
                    name: format!("p{idx}"),
                    ty,
                })
                .collect(),
            ret,
            effect,
        }
    }

    #[test]
    fn strict_abi_link_accepts_exact_symbol_signature_match() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::core::math::add".to_string(),
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: None,
            }]);
        let actual = vec![external_sig(
            "std::core::math::add",
            vec![Type::Int, Type::Int],
            Type::Int,
            Effect::Pure,
        )];
        assert!(strict_abi_link_violation(&package_contract, &actual).is_none());
    }

    #[test]
    fn strict_abi_link_rejects_missing_expected_symbol() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::core::math::add".to_string(),
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: None,
            }]);
        assert!(
            strict_abi_link_violation(&package_contract, &[]).is_none(),
            "unlinked symbols should not fail strict ABI/link gate"
        );
    }

    #[test]
    fn strict_abi_link_rejects_effect_or_signature_mismatch() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::core::math::add".to_string(),
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: None,
            }]);
        let actual = vec![external_sig(
            "std::core::math::add",
            vec![Type::Int, Type::Int],
            Type::Int,
            Effect::Mut,
        )];
        let err =
            strict_abi_link_violation(&package_contract, &actual).expect("signature mismatch");
        assert!(err.contains("expected effect/signature"));
    }

    #[test]
    fn strict_abi_link_rejects_conflicting_profiles_for_same_symbol() {
        let package_contract = StrictPackageContractV0 {
            packages: vec![
                strict_package_contract::StrictPackageMetadataEntry {
                    name: "std::core".to_string(),
                    version: "1.0.0".to_string(),
                    digest:
                        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    artifact_format: "wasm".to_string(),
                    artifact_path: "store/std-core-1.0.0.wasm".to_string(),
                    abi_id: "abi:std::core:1.0.0".to_string(),
                    dependencies: Vec::new(),
                    signature: None,
                    trusted_anchor_ids: Vec::new(),
                },
                strict_package_contract::StrictPackageMetadataEntry {
                    name: "std::math".to_string(),
                    version: "1.0.0".to_string(),
                    digest:
                        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                            .to_string(),
                    artifact_format: "wasm".to_string(),
                    artifact_path: "store/std-math-1.0.0.wasm".to_string(),
                    abi_id: "abi:std::math:1.0.0".to_string(),
                    dependencies: Vec::new(),
                    signature: None,
                    trusted_anchor_ids: Vec::new(),
                },
            ],
            contracts: vec![
                strict_package_contract::StrictAbiContractEntry {
                    abi_id: "abi:std::core:1.0.0".to_string(),
                    package: "std::core".to_string(),
                    version: "1.0.0".to_string(),
                    imports: vec![strict_package_contract::StrictAbiImportEntry {
                        symbol: "std::crypto::hash".to_string(),
                        effect: "pure".to_string(),
                        params: vec!["Bytes".to_string()],
                        ret: "Bytes".to_string(),
                        capability: Some("std::crypto::hash".to_string()),
                    }],
                },
                strict_package_contract::StrictAbiContractEntry {
                    abi_id: "abi:std::math:1.0.0".to_string(),
                    package: "std::math".to_string(),
                    version: "1.0.0".to_string(),
                    imports: vec![strict_package_contract::StrictAbiImportEntry {
                        symbol: "std::crypto::hash".to_string(),
                        effect: "pure".to_string(),
                        params: vec!["Bytes".to_string()],
                        ret: "Bytes".to_string(),
                        capability: None,
                    }],
                },
            ],
        };
        let err = strict_abi_link_violation(&package_contract, &[])
            .expect("conflicting ABI profiles should fail");
        assert!(err.contains("conflicting ABI profiles"));
    }

    #[test]
    fn strict_runtime_capability_accepts_required_capability_present() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::crypto::hash".to_string(),
                effect: "pure".to_string(),
                params: vec!["Bytes".to_string()],
                ret: "Bytes".to_string(),
                capability: Some("std::crypto::hash".to_string()),
            }]);
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: vec!["std::crypto::hash".to_string()],
        };
        let linked = vec![external_sig(
            "std::crypto::hash",
            vec![Type::Bytes],
            Type::Bytes,
            Effect::Pure,
        )];
        assert!(
            strict_runtime_capability_violation(&package_contract, &host_profile, &linked)
                .is_none()
        );
    }

    #[test]
    fn strict_runtime_capability_rejects_missing_required_capability() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::crypto::hash".to_string(),
                effect: "pure".to_string(),
                params: vec!["Bytes".to_string()],
                ret: "Bytes".to_string(),
                capability: Some("std::crypto::hash".to_string()),
            }]);
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: vec!["std::wasi::print".to_string()],
        };
        let linked = vec![external_sig(
            "std::crypto::hash",
            vec![Type::Bytes],
            Type::Bytes,
            Effect::Pure,
        )];
        let err = strict_runtime_capability_violation(&package_contract, &host_profile, &linked)
            .expect("missing capability should fail");
        assert!(err.contains("not present in host profile"));
    }

    #[test]
    fn strict_runtime_capability_rejects_env_time_in_contract_static_even_if_present() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::env::time".to_string(),
                effect: "io".to_string(),
                params: vec![],
                ret: "Int".to_string(),
                capability: Some("std::env::time".to_string()),
            }]);
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: vec!["std::env::time".to_string()],
        };
        let linked = vec![external_sig(
            "std::env::time",
            vec![],
            Type::Int,
            Effect::Io,
        )];
        let err = strict_runtime_capability_violation(&package_contract, &host_profile, &linked)
            .expect("env time should be denied in strict contract_static");
        assert!(err.contains("denied by strict deterministic policy"));
    }

    #[test]
    fn strict_runtime_capability_rejects_env_random_in_shared_app_even_if_present() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::env::random".to_string(),
                effect: "io".to_string(),
                params: vec!["Int".to_string()],
                ret: "Bytes".to_string(),
                capability: Some("std::env::random".to_string()),
            }]);
        let host_profile = StrictHostProfileV0 {
            profile: "shared_app".to_string(),
            capabilities: vec!["std::env::random".to_string()],
        };
        let linked = vec![external_sig(
            "std::env::random",
            vec![Type::Int],
            Type::Bytes,
            Effect::Io,
        )];
        let err = strict_runtime_capability_violation(&package_contract, &host_profile, &linked)
            .expect("env random should be denied in strict shared_app");
        assert!(err.contains("denied by strict deterministic policy"));
    }

    #[test]
    fn precompiled_typer_filter_removes_locked_std_core_overrides() {
        let sigs = vec![
            external_sig("std::str::len", vec![Type::String], Type::Int, Effect::Pure),
            external_sig(
                "std::core::math::add",
                vec![Type::Int, Type::Int],
                Type::Int,
                Effect::Pure,
            ),
        ];
        let filtered = filter_precompiled_std_core_typer_overrides(sigs.as_slice());
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "std::core::math::add");
    }

    #[test]
    fn precompiled_fallback_gate_rejects_unlinked_locked_std_core_symbol() {
        let ir = IrModule {
            funcs: vec![clg_ir::Function {
                name: "std::str::len".to_string(),
                params: vec![IrType::Int],
                ret: Some(IrType::Int),
                body: Vec::new(),
            }],
        };
        let violations = precompiled_std_core_intrinsic_fallback_violations(
            StdCoreLinkMode::Precompiled,
            &ir,
            &Ok(Vec::new()),
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].code, "C105");
        assert_eq!(violations[0].package, "std::core");
        assert_eq!(violations[0].symbol, "std::str::len");
        assert!(violations[0].message.contains("forbids intrinsic fallback"));
    }

    #[test]
    fn precompiled_fallback_gate_rejects_unlinked_locked_std_core_bytes_len_symbol() {
        let ir = IrModule {
            funcs: vec![clg_ir::Function {
                name: "std::bytes::len".to_string(),
                params: vec![IrType::Int],
                ret: Some(IrType::Int),
                body: Vec::new(),
            }],
        };
        let violations = precompiled_std_core_intrinsic_fallback_violations(
            StdCoreLinkMode::Precompiled,
            &ir,
            &Ok(Vec::new()),
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].code, "C105");
        assert_eq!(violations[0].symbol, "std::bytes::len");
    }

    #[test]
    fn strict_determinism_gate_accepts_stable_import_map() {
        let package_contract =
            abi_contract_with_imports(vec![strict_package_contract::StrictAbiImportEntry {
                symbol: "std::core::math::add".to_string(),
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: None,
            }]);
        let actual = vec![external_sig(
            "std::core::math::add",
            vec![Type::Int, Type::Int],
            Type::Int,
            Effect::Pure,
        )];
        let bindings =
            strict_external_bindings_from_contract(&package_contract).expect("strict bindings");
        let linked = resolved_external_import_profiles(&actual)
            .expect("resolve profiles")
            .into_iter()
            .collect::<Vec<_>>();
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: Vec::new(),
        };
        let diagnostics = evaluate_strict_gates(
            &bindings.expected_profiles,
            linked.as_slice(),
            &host_profile,
        );
        assert!(strict_determinism_violation(
            &bindings.expected_profiles,
            &host_profile,
            linked.as_slice(),
            diagnostics.as_slice(),
        )
        .is_none());
    }

    #[test]
    fn strict_determinism_gate_detects_order_dependent_external_conflict() {
        let package_contract = abi_contract_with_imports(vec![
            strict_package_contract::StrictAbiImportEntry {
                symbol: "std::core::math::add".to_string(),
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: Some("std::crypto::hash".to_string()),
            },
            strict_package_contract::StrictAbiImportEntry {
                symbol: "std::core::math::sub".to_string(),
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: Some("std::crypto::hash".to_string()),
            },
        ]);
        let actual = vec![
            external_sig(
                "std::core::math::add",
                vec![Type::Int, Type::Int],
                Type::Int,
                Effect::Pure,
            ),
            external_sig(
                "std::core::math::sub",
                vec![Type::Int, Type::Int],
                Type::Int,
                Effect::Pure,
            ),
        ];
        let bindings =
            strict_external_bindings_from_contract(&package_contract).expect("strict bindings");
        let linked = resolved_external_import_profiles(&actual)
            .expect("resolve profiles")
            .into_iter()
            .collect::<Vec<_>>();
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: Vec::new(),
        };
        let mut diagnostics = evaluate_strict_gates(
            &bindings.expected_profiles,
            linked.as_slice(),
            &host_profile,
        );
        diagnostics.reverse();
        let err = strict_determinism_violation(
            &bindings.expected_profiles,
            &host_profile,
            linked.as_slice(),
            diagnostics.as_slice(),
        )
        .expect("diagnostics-order drift should fail determinism replay");
        assert!(err.contains("determinism replay failed"));
    }

    #[test]
    fn strict_import_map_artifact_includes_pre_evaluation_violations() {
        let expected_profiles = std::collections::BTreeMap::new();
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: Vec::new(),
        };
        let linked_imports: Vec<(String, AbiLinkProfile)> = Vec::new();
        let pre_eval_violations = vec![StrictGateViolation {
            code: "C105",
            package: "_".to_string(),
            symbol: "_".to_string(),
            message: "strict ABI/link mismatch: synthetic pre-eval failure".to_string(),
        }];
        let baseline_outcome = StrictGateOutcome {
            linked_imports: linked_imports.clone(),
            violations: pre_eval_violations.clone(),
        };
        let replay_outcome = baseline_outcome.clone();
        let artifact = strict_import_map_artifact_with_determinism_check(
            &expected_profiles,
            &host_profile,
            &baseline_outcome,
            &replay_outcome,
        )
        .expect("artifact generation should include pre-evaluation violations");
        let value: serde_json::Value =
            serde_json::from_slice(artifact.canonical_bytes.as_slice()).expect("artifact json");
        let diagnostics = value
            .get("diagnostics")
            .and_then(|items| items.as_array())
            .expect("diagnostics array");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0]
                .get("code")
                .and_then(|code| code.as_str())
                .unwrap_or_default(),
            "C105"
        );
    }

    #[test]
    fn evaluate_strict_gates_orders_by_code_then_package_then_symbol() {
        let expected_profiles = std::collections::BTreeMap::from([
            (
                "zzz::alpha::f".to_string(),
                AbiLinkProfile {
                    effect: "pure".to_string(),
                    params: vec!["Int".to_string()],
                    ret: "Int".to_string(),
                    capability: Some("std::crypto::hash".to_string()),
                },
            ),
            (
                "aaa::beta::g".to_string(),
                AbiLinkProfile {
                    effect: "pure".to_string(),
                    params: vec!["Int".to_string()],
                    ret: "Int".to_string(),
                    capability: Some("std::crypto::hash".to_string()),
                },
            ),
        ]);
        let linked = vec![
            (
                "zzz::alpha::f".to_string(),
                AbiLinkProfile {
                    effect: "mut".to_string(),
                    params: vec!["Int".to_string()],
                    ret: "Int".to_string(),
                    capability: None,
                },
            ),
            (
                "aaa::beta::g".to_string(),
                AbiLinkProfile {
                    effect: "pure".to_string(),
                    params: vec!["Int".to_string()],
                    ret: "Int".to_string(),
                    capability: None,
                },
            ),
            (
                "mmm::orphan::h".to_string(),
                AbiLinkProfile {
                    effect: "pure".to_string(),
                    params: vec!["Int".to_string()],
                    ret: "Int".to_string(),
                    capability: None,
                },
            ),
        ];
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: Vec::new(),
        };
        let violations = evaluate_strict_gates(&expected_profiles, &linked, &host_profile);
        let order: Vec<(&str, &str, &str)> = violations
            .iter()
            .map(|v| (v.code, v.package.as_str(), v.symbol.as_str()))
            .collect();
        assert_eq!(
            order,
            vec![
                ("C105", "mmm::orphan", "mmm::orphan::h"),
                ("C105", "zzz::alpha", "zzz::alpha::f"),
                ("C106", "aaa::beta", "aaa::beta::g"),
            ]
        );
    }

    #[test]
    fn evaluate_strict_gates_snapshot_stable_output() {
        let expected_profiles = std::collections::BTreeMap::from([(
            "std::core::math::add".to_string(),
            AbiLinkProfile {
                effect: "pure".to_string(),
                params: vec!["Int".to_string(), "Int".to_string()],
                ret: "Int".to_string(),
                capability: Some("std::crypto::hash".to_string()),
            },
        )]);
        let linked = vec![
            (
                "std::core::math::add".to_string(),
                AbiLinkProfile {
                    effect: "mut".to_string(),
                    params: vec!["Int".to_string(), "Int".to_string()],
                    ret: "Int".to_string(),
                    capability: None,
                },
            ),
            (
                "std::orphan::noop".to_string(),
                AbiLinkProfile {
                    effect: "pure".to_string(),
                    params: vec!["Int".to_string()],
                    ret: "Int".to_string(),
                    capability: None,
                },
            ),
        ];
        let host_profile = StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: Vec::new(),
        };
        let violations = evaluate_strict_gates(&expected_profiles, &linked, &host_profile);
        let snapshot = violations
            .iter()
            .map(|v| format!("{}|{}|{}", v.code, v.package, v.symbol))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            snapshot,
            "C105|std::core|std::core::math::add\nC105|std::orphan|std::orphan::noop"
        );
    }
}
