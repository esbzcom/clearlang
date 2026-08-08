#[allow(clippy::too_many_arguments)]
pub fn run(
    file: PathBuf,
    out: PathBuf,
    contract: bool,
    validate: bool,
    debug_names: bool,
    emit_contract_state_schema: Option<PathBuf>,
    check_contract_state_schema: Option<PathBuf>,
    emit_vcs: Option<PathBuf>,
    emit_proof: Option<PathBuf>,
    compiler_mode: CompilerMode,
    release_profile: ReleaseProfile,
    std_core_link_mode: StdCoreLinkMode,
    proof_strict: Option<bool>,
    sign: bool,
    key: Option<PathBuf>,
    key_id: Option<String>,
    scope: SignScope,
    sig_out: Option<PathBuf>,
    assurance_manifest_out: Option<PathBuf>,
    lean_checker_version: Option<String>,
    coq_checker_version: Option<String>,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    if sign && emit_vcs.is_none() {
        anyhow::bail!("--sign requires --emit-vcs");
    }

    let fail_preflight = |code: &'static str, message: &str| -> Result<()> {
        if json_errors {
            let json = make_single_json_error(code, "build", message, &file, 0, 0, None);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(message.to_string()))
        }
    };
    let mut strict_host_profile_for_caps: Option<StrictHostProfileV0> = None;
    let mut strict_external_bindings_for_link: Option<StrictExternalBindings> = None;

    if release_profile == ReleaseProfile::Production && compiler_mode != CompilerMode::Strict {
        fail_preflight(
            "C120",
            "`--release-profile production` requires `--compiler-mode strict`",
        )?;
    }
    if std_core_link_mode == StdCoreLinkMode::Precompiled && compiler_mode != CompilerMode::Strict {
        fail_preflight(
            "C035",
            "`--std-core-link-mode precompiled` requires `--compiler-mode strict`",
        )?;
    }
    if compiler_mode == CompilerMode::Strict && emit_vcs.is_none() {
        fail_preflight(
            "C029",
            "`--compiler-mode strict` requires `--emit-vcs <FILE>`",
        )?;
    }
    if compiler_mode == CompilerMode::Strict {
        let module_root = file.parent().unwrap_or_else(|| Path::new("."));
        let legacy_source = module_root.join(LEGACY_PACKAGE_METADATA_FILE);
        if legacy_source.exists() {
            fail_preflight(
                "C101",
                &format!(
                    "strict mode source-of-truth violation: `{}` is not allowed; use strict preflight inputs (`clg.lock.json`, `clg.package-metadata.json`, `clg.package-abi.json`) and trusted local store artifacts only",
                    legacy_source.display()
                ),
            )?;
        }
        let strict_preflight = match load_required_strict_preflight_input_v0(module_root) {
            Ok(value) => value,
            Err(err) => return fail_preflight(err.code(), err.message()),
        };
        let strict_lockfile = strict_preflight.lockfile;
        let strict_trust_policy = strict_preflight.trust_policy;
        let strict_package_contract = strict_preflight.package_contract;
        let strict_host_profile = strict_preflight.host_profile;
        if let Some(message) =
            strict_artifact_identity_violation(&strict_lockfile, &strict_package_contract)
        {
            fail_preflight("C102", &message)?;
        }
        if let Err(err) =
            enforce_trust_gate_v0(module_root, &strict_package_contract, &strict_trust_policy)
        {
            fail_preflight(err.code(), err.message())?;
        }
        let strict_external_bindings =
            match strict_external_bindings_from_contract(&strict_package_contract) {
                Ok(value) => value,
                Err(message) => return fail_preflight("C105", &message),
            };
        strict_host_profile_for_caps = Some(strict_host_profile);
        strict_external_bindings_for_link = Some(strict_external_bindings);
    }

    let mut timings = StageTimings::new();
    let loaded = {
        let _stage = timings.start(logger, "parse");
        load_program(&file, json_errors)?
    };
    let module_root = file.parent().unwrap_or_else(|| Path::new("."));
    let contract_source_graph = contract_source_graph_identity(module_root, loaded.source_files.as_slice())?;
    if release_profile == ReleaseProfile::Production {
        if let Some(message) =
            release_module_graph_test_path_violation(module_root, loaded.source_files.as_slice())
        {
            fail_preflight("C128", &message)?;
        }
    }
    let ast = &loaded.program;
    let strict_import_map_source_files =
        strict_import_map_source_files(module_root, loaded.source_files.as_slice());
    let (
        external_typer_sigs_for_typecheck,
        external_typer_sigs_for_link_resolution,
        external_codegen_imports,
    ) = if compiler_mode == CompilerMode::Strict {
        if let Some(bindings) = strict_external_bindings_for_link.as_ref() {
            let (typecheck_sigs, codegen_imports) =
                if std_core_link_mode == StdCoreLinkMode::Precompiled {
                    (
                        filter_precompiled_std_core_typer_overrides(
                            bindings.external_typer_sigs.as_slice(),
                        ),
                        filter_precompiled_std_core_codegen_overrides(
                            bindings.external_codegen_imports.as_slice(),
                        ),
                    )
                } else {
                    (
                        bindings.external_typer_sigs.clone(),
                        bindings.external_codegen_imports.clone(),
                    )
                };
            (
                typecheck_sigs,
                bindings.external_typer_sigs.clone(),
                codegen_imports,
            )
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        }
    } else {
        let typer_sigs: Vec<ExternalBuiltinSig> = loaded
            .external_imports
            .iter()
            .map(|binding| ExternalBuiltinSig {
                name: binding.function.clone(),
                params: binding.params.clone(),
                ret: binding.ret.clone(),
                effect: binding.effect,
                route: binding.route,
            })
            .collect();
        let codegen_imports = loaded
            .external_imports
            .iter()
            .filter(|binding| binding.route == clg_typer::BuiltinRoute::PackageImport)
            .map(|binding| ExternalImport {
                function: binding.function.clone(),
                import_module: binding.import_module.clone(),
                import_name: binding.import_name.clone(),
            })
            .collect();
        (typer_sigs.clone(), typer_sigs, codegen_imports)
    };

    let type_output = {
        let _stage = timings.start(logger, "typecheck");
        match check_with_vcs_with_std_and_external(
            ast,
            &loaded.std_types,
            &external_typer_sigs_for_typecheck,
        ) {
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
        mut vcs,
        mono_program,
        mangled_name_origins,
    } = type_output;
    let contract_proof_source_graph = (!mono_program.contracts.is_empty()).then_some(&contract_source_graph);

    if let Some(schema_path) = emit_contract_state_schema.as_ref() {
        let _stage = timings.start(logger, "emit_contract_state_schema");
        write_contract_state_schema(&mono_program, schema_path, Some(&contract_source_graph))?;
    }
    if let Some(prior_schema_path) = check_contract_state_schema.as_ref() {
        let _stage = timings.start(logger, "check_contract_state_schema");
        check_contract_state_schema_compatibility(&mono_program, prior_schema_path)?;
    }
    let vcs_before_solver = vcs.clone();

    let fail_build = |code: &'static str, message: &str, function: Option<String>| -> Result<()> {
        if json_errors {
            let json = make_single_json_error(code, "build", message, &file, 0, 0, function);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(message.to_string()))
        }
    };
    let fail_build_violations = |violations: &[StrictGateViolation]| -> Result<()> {
        if violations.is_empty() {
            return Ok(());
        }
        if json_errors {
            let errors = violations
                .iter()
                .map(|violation| JsonErrorItem {
                    code: violation.code,
                    stage: "build",
                    message: violation.message.clone(),
                    file: file.display().to_string(),
                    start: 0,
                    end: 0,
                    function: None,
                })
                .collect();
            let json = JsonError { ok: false, errors };
            Err(CommandError::json(json).into())
        } else {
            let mut message = String::from("strict mode gate violations:");
            for violation in violations {
                message.push_str("\n - [");
                message.push_str(violation.code);
                message.push_str("] ");
                message.push_str(violation.message.as_str());
            }
            Err(anyhow!(message))
        }
    };
    if let Err(err) = apply_solver_outcomes_if_configured(vcs.as_mut_slice()) {
        if compiler_mode == CompilerMode::Strict {
            return fail_build(
                "C124",
                &format!("strict proof execution failed: {err:#}"),
                None,
            );
        }
        return Err(err);
    }
    if compiler_mode == CompilerMode::Strict {
        if let (Some(host_profile), Some(bindings)) = (
            strict_host_profile_for_caps.as_ref(),
            strict_external_bindings_for_link.as_ref(),
        ) {
            let baseline_linked_import_resolution = linked_external_import_profiles_from_ir(
                &ir,
                external_typer_sigs_for_link_resolution.as_slice(),
            );
            let baseline_outcome = strict_gate_outcome_from_linked_import_resolution(
                &bindings.expected_profiles,
                host_profile,
                baseline_linked_import_resolution.clone(),
            );
            let mut baseline_outcome = baseline_outcome;
            baseline_outcome
                .violations
                .extend(precompiled_std_core_intrinsic_fallback_violations(
                    std_core_link_mode,
                    &ir,
                    &baseline_linked_import_resolution,
                ));
            sort_strict_gate_violations(baseline_outcome.violations.as_mut_slice());
            let mut replay_external_typer_sigs = external_typer_sigs_for_link_resolution.clone();
            replay_external_typer_sigs.reverse();
            let replay_linked_import_resolution =
                linked_external_import_profiles_from_ir(&ir, replay_external_typer_sigs.as_slice());
            let mut replay_outcome = strict_gate_outcome_from_linked_import_resolution(
                &bindings.expected_profiles,
                host_profile,
                replay_linked_import_resolution.clone(),
            );
            replay_outcome
                .violations
                .extend(precompiled_std_core_intrinsic_fallback_violations(
                    std_core_link_mode,
                    &ir,
                    &replay_linked_import_resolution,
                ));
            sort_strict_gate_violations(replay_outcome.violations.as_mut_slice());
            // Test-only hook to force deterministic replay mismatch coverage in CLI IT.
            if std::env::var_os("CLG_TEST_FORCE_STRICT_DETERMINISM_MISMATCH").is_some() {
                replay_outcome.violations.push(StrictGateViolation {
                    code: "C105",
                    package: "_".to_string(),
                    symbol: "_".to_string(),
                    message: "strict determinism test hook: forced replay mismatch".to_string(),
                });
                sort_strict_gate_violations(replay_outcome.violations.as_mut_slice());
            }
            let mut violations = baseline_outcome.violations.clone();
            let strict_import_map_shared_std = match strict_import_map_shared_std_evidence(module_root)
            {
                Ok(entries) => entries,
                Err(message) => {
                    violations.push(StrictGateViolation {
                        code: "C108",
                        package: "_".to_string(),
                        symbol: "_".to_string(),
                        message: format!("strict import-map shared std evidence load failed: {message}"),
                    });
                    Vec::new()
                }
            };
            let strict_import_map_artifact = match strict_import_map_artifact_with_determinism_check(
                &bindings.expected_profiles,
                host_profile,
                strict_import_map_source_files.as_slice(),
                strict_import_map_shared_std.as_slice(),
                &baseline_outcome,
                &replay_outcome,
            ) {
                Ok(artifact) => Some(artifact),
                Err(message) => {
                    violations.push(StrictGateViolation {
                        code: "C107",
                        package: "_".to_string(),
                        symbol: "_".to_string(),
                        message,
                    });
                    None
                }
            };
            if let Some(artifact) = strict_import_map_artifact.as_ref() {
                match write_strict_import_map_artifact(&out, artifact) {
                    Ok(path) => {
                        if logger.enabled(LogLevel::Debug) {
                            logger.event(
                                LogLevel::Debug,
                                "strict_import_map",
                                "strict_preflight",
                                &[
                                    ("path", path.display().to_string()),
                                    ("sha256", artifact.canonical_hash.clone()),
                                ],
                            );
                        }
                    }
                    Err(err) => violations.push(StrictGateViolation {
                        code: "C108",
                        package: "_".to_string(),
                        symbol: "_".to_string(),
                        message: format!("strict import-map artifact emission failed: {err:#}"),
                    }),
                }
            }
            sort_strict_gate_violations(violations.as_mut_slice());
            fail_build_violations(violations.as_slice())?;
        }
    }

    if (lean_checker_version.is_some() || coq_checker_version.is_some()) && !sign {
        fail_build(
            "C032",
            "`--lean-checker-version`/`--coq-checker-version` require `--sign`",
            None,
        )?;
    }
    if assurance_manifest_out.is_some() && !sign {
        fail_build("C034", "`--assurance-manifest-out` requires `--sign`", None)?;
    }
    if lean_checker_version.is_some() ^ coq_checker_version.is_some() {
        fail_build(
            "C032",
            "`--lean-checker-version` and `--coq-checker-version` must be provided together",
            None,
        )?;
    }
    if let (Some(lean_checker_version), Some(coq_checker_version)) =
        (lean_checker_version.as_ref(), coq_checker_version.as_ref())
    {
        if lean_checker_version.trim().is_empty() || coq_checker_version.trim().is_empty() {
            fail_build(
                "C032",
                "`--lean-checker-version` and `--coq-checker-version` must be non-empty",
                None,
            )?;
        }
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
    if compiler_mode == CompilerMode::Strict && emit_vcs.is_some() {
        if release_profile == ReleaseProfile::Production {
            if let Some(message) = release_crypto_boundary_violation(&vcs) {
                fail_build("C123", &message, None)?;
            }
        }
        if let Some(message) = strict_l3_claim_violation(&vcs) {
            fail_build("C031", &message, None)?;
        }
        if let Some(message) = strict_language_profile_violation(&vcs) {
            fail_build("C033", &message, None)?;
        }
    }
    if compiler_mode == CompilerMode::Strict && !vcs.is_empty() {
        if vcs.iter().any(|vc| vc.status == "generated") {
            fail_build(
                "C124",
                "strict proof execution failed because configured theorem prover is unavailable",
                None,
            )?;
        }
        if vcs.iter().any(|vc| vc.status == "timeout") {
            fail_build(
                "C125",
                "strict proof execution reached configured theorem-prover timeout budget",
                None,
            )?;
        }
        if release_profile == ReleaseProfile::Production {
            let mut replay_vcs = vcs_before_solver.clone();
            if let Err(err) = apply_solver_outcomes_if_configured(replay_vcs.as_mut_slice()) {
                return fail_build(
                    "C124",
                    &format!("strict proof replay failed: {err:#}"),
                    None,
                );
            }
            if replay_vcs.iter().any(|vc| vc.status == "generated") {
                fail_build(
                    "C124",
                    "strict proof replay failed because configured theorem prover is unavailable",
                    None,
                )?;
            }
            if replay_vcs.iter().any(|vc| vc.status == "timeout") {
                fail_build(
                    "C125",
                    "strict proof replay reached configured theorem-prover timeout budget",
                    None,
                )?;
            }
            let baseline_statuses = vcs.iter().map(|vc| vc.status).collect::<Vec<_>>();
            let replay_statuses = replay_vcs.iter().map(|vc| vc.status).collect::<Vec<_>>();
            if baseline_statuses != replay_statuses {
                fail_build(
                    "C127",
                    "deterministic solver replay mismatch on identical strict inputs",
                    None,
                )?;
            }
        }
    }
    if release_profile == ReleaseProfile::Production {
        match production_release_surface_violation(&mono_program) {
            Ok(Some(message)) => fail_build("C122", &message, None)?,
            Ok(None) => {}
            Err(err) => fail_build("C122", &format!("{err:#}"), None)?,
        }
        let proof_status = proof_status_for_vcs(&vcs, compiler_mode.as_str());
        if proof_status != PROOF_STATUS_PROVED_ALL {
            fail_build(
                "C121",
                &format!(
                    "release profile `production` requires theorem-grade assurance (`proof_status={}`); got `{}`",
                    PROOF_STATUS_PROVED_ALL, proof_status
                ),
                None,
            )?;
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
    let mut proof_artifact_emission: Option<ProofArtifactEmission> = None;
    let proof_package = emit_vcs.as_ref().map(|_| {
        ProofPackage::from_program(
            &mono_program,
            &vcs,
            &mangled_name_origins,
            toolchain.clone(),
            compiler_mode.as_str(),
        )
    });

    let zero_section = proof_package
        .as_ref()
        .map(|pkg| pkg.encode_section(&[0u8; 32]))
        .transpose()?;

    let (wasm_bytes, module_hash_bytes) = {
        let _stage = timings.start(logger, "codegen");
        let wasm_std_core_link_mode = WasmStdCoreLinkMode::from(std_core_link_mode);
        let mut wasm_bytes = emit_from_ir_with_opts(
            &ir,
            CodegenOpts {
                debug_names,
                proof_section: zero_section.clone(),
                export_aliases: export_aliases.clone(),
                external_imports: external_codegen_imports.clone(),
                std_core_link_mode: wasm_std_core_link_mode,
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
                    std_core_link_mode: wasm_std_core_link_mode,
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

    include!("finalize_block.rs");

    logger.summary(&timings);
    Ok(())
}
