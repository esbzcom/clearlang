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
            &[],
            &[],
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
            &[],
            &[],
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
            &[],
            &[],
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
    fn release_module_graph_test_path_violation_detects_tests_tree() {
        let root = Path::new("C:/repo/project");
        let sources = vec![
            PathBuf::from("C:/repo/project/main.clear"),
            PathBuf::from("C:/repo/project/tests/unit/helper.clear"),
        ];
        let violation = release_module_graph_test_path_violation(root, sources.as_slice())
            .expect("tests path should violate production release isolation gate");
        assert!(violation.contains("tests/unit/helper.clear"));
    }

    #[test]
    fn strict_import_map_source_files_are_sorted_and_normalized() {
        let root = Path::new("C:/repo/project");
        let sources = vec![
            PathBuf::from("C:/repo/project/services/z.clear"),
            PathBuf::from("C:/repo/project/main.clear"),
            PathBuf::from("C:/repo/project/services/a.clear"),
        ];
        let normalized = strict_import_map_source_files(root, sources.as_slice());
        assert_eq!(
            normalized,
            vec![
                "main.clear".to_string(),
                "services/a.clear".to_string(),
                "services/z.clear".to_string(),
            ]
        );
    }

    #[test]
    fn contract_source_graph_is_computed_only_for_identity_consumers() {
        assert!(!requires_contract_source_graph(false, false, false, false, false));
        assert!(!requires_contract_source_graph(true, false, false, false, false));
        assert!(requires_contract_source_graph(false, true, false, false, false));
        assert!(requires_contract_source_graph(false, false, true, false, false));
        assert!(requires_contract_source_graph(true, false, false, true, false));
        assert!(!requires_contract_source_graph(false, false, false, true, false));
        assert!(requires_contract_source_graph(false, false, false, false, true));
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

