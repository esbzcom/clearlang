{
    if let Some(vcs_path) = emit_vcs {
        let _stage = timings.start(logger, "emit_vcs");
        write_vcs_json(
            &vcs,
            &mangled_name_origins,
            &vcs_path,
            &file,
            compiler_mode.as_str(),
        )?;
    }
    if let Some(proof_path) = emit_proof.as_ref() {
        let _stage = timings.start(logger, "emit_proof");
        let emission = write_proof_artifact_json(
            &vcs,
            proof_path,
            toolchain.as_str(),
            compiler_mode.as_str(),
            contract_proof_source_graph,
        )?;
        if logger.enabled(LogLevel::Debug) {
            logger.event(
                LogLevel::Debug,
                "proof_artifact",
                "emit_proof",
                &[
                    ("path", proof_path.display().to_string()),
                    ("proof_artifact_hash", emission.artifact_hash.clone()),
                    ("solver_profile_hash", emission.solver_profile_hash.clone()),
                ],
            );
        }
        proof_artifact_emission = Some(emission);
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
        let manifest_path =
            assurance_manifest_out.unwrap_or_else(|| default_assurance_manifest_path(&sig_path));
        let module_hash_hex = hex::encode(hash_bytes);
        let proofs_hash_hex = pkg.proofs_hash_hex();
        let proof_artifact_hash = proof_artifact_emission
            .as_ref()
            .map(|emission| emission.artifact_hash.as_str());
        let solver_profile_hash = proof_artifact_emission
            .as_ref()
            .map(|emission| emission.solver_profile_hash.as_str());
        let solver_profile = proof_artifact_emission
            .as_ref()
            .map(|emission| &emission.solver_profile);
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
            lean_checker_version.as_deref(),
            coq_checker_version.as_deref(),
            proof_artifact_hash,
            solver_profile_hash,
            solver_profile,
            Some(&contract_source_graph),
        )?;
        let manifest_payload = build_assurance_manifest_payload(AssuranceManifestInput {
            vcs: &vcs,
            toolchain: &toolchain,
            compiler_mode: compiler_mode.as_str(),
            proof_strict: proof_strict_enabled,
            module_hash_hex: &module_hash_hex,
            proofs_hash_hex: &proofs_hash_hex,
            generated_at: &timestamp,
            proof_artifact_hash,
            solver_profile_hash,
        });
        signing::sign_assurance_manifest(manifest_payload, &key_path, &key_id, &manifest_path)?;
        if logger.enabled(LogLevel::Debug) {
            logger.event(
                LogLevel::Debug,
                "sign_detail",
                "sign",
                &[
                    ("module_hash", module_hash_hex.clone()),
                    ("proofs_hash", proofs_hash_hex),
                    (
                        "proof_artifact_hash",
                        proof_artifact_hash.unwrap_or("<none>").to_string(),
                    ),
                    (
                        "solver_profile_hash",
                        solver_profile_hash.unwrap_or("<none>").to_string(),
                    ),
                    ("sig_path", sig_path.display().to_string()),
                    ("assurance_manifest", manifest_path.display().to_string()),
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
}
