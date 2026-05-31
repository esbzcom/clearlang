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
            external_sig("std::env::chain_id", vec![], Type::String, Effect::Io),
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
    fn precompiled_codegen_filter_keeps_locked_symbols_and_drops_builtin_overrides() {
        let imports = vec![
            ExternalImport {
                function: "std::str::len".to_string(),
                import_module: "std::str".to_string(),
                import_name: "len".to_string(),
            },
            ExternalImport {
                function: "std::env::chain_id".to_string(),
                import_module: "std::env".to_string(),
                import_name: "chain_id".to_string(),
            },
            ExternalImport {
                function: "std::core::math::add".to_string(),
                import_module: "std::core::math".to_string(),
                import_name: "add".to_string(),
            },
        ];
        let filtered = filter_precompiled_std_core_codegen_overrides(imports.as_slice());
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].function, "std::str::len");
        assert_eq!(filtered[1].function, "std::core::math::add");
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

