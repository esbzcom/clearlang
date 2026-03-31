fn run_supply_chain_self_test(root: &Path) -> Result<(), String> {
    let temp_root = root.join("tmp").join("xtask").join("supply-chain-self-test");
    if temp_root.exists() {
        fs::remove_dir_all(&temp_root)
            .map_err(|e| format!("cleanup supply-chain self-test dir: {e}"))?;
    }
    fs::create_dir_all(&temp_root).map_err(|e| format!("create self-test dir: {e}"))?;

    let metadata_path = temp_root.join("metadata.json");
    let package_metadata_path = temp_root.join("fixtures").join("clg.package-metadata.json");
    let artifact_dir = temp_root.join("fixtures").join("artifact");
    fs::create_dir_all(&artifact_dir).map_err(|e| format!("create artifact dir: {e}"))?;
    fs::write(artifact_dir.join("pkg-a.wasm"), b"\0asm\x01\0\0\0")
        .map_err(|e| format!("write self-test artifact: {e}"))?;

    write_json_pretty(
        &metadata_path,
        &json!({
          "packages": [{
            "id": "pkg_a 1.0.0 (path+file:///pkg_a)",
            "name": "pkg_a",
            "version": "1.0.0",
            "license": "",
            "license_file": ""
          }],
          "workspace_members": ["pkg_a 1.0.0 (path+file:///pkg_a)"],
          "resolve": { "nodes": [{ "id": "pkg_a 1.0.0 (path+file:///pkg_a)" }] }
        }),
    )?;
    write_json_pretty(
        &package_metadata_path,
        &json!({
          "schema_version": 1,
          "packages": [{
            "name": "pkg::a",
            "version": "1.0.0",
            "digest": "sha256:abc",
            "artifact": { "path": "artifact/pkg-a.wasm" }
          }]
        }),
    )?;
    let runtime_dir = temp_root.join("tmp").join("perf").join("runtime_loader");
    fs::create_dir_all(runtime_dir.join("store"))
        .map_err(|e| format!("create runtime self-test store dir: {e}"))?;
    fs::write(runtime_dir.join("store").join("pkg-a.wasm"), b"\0asm\x01\0\0\0")
        .map_err(|e| format!("write runtime artifact: {e}"))?;
    let digest = format!(
        "sha256:{}",
        sha256_hex(&runtime_dir.join("store").join("pkg-a.wasm"))?
    );
    write_json_pretty(
        &runtime_dir.join("clg.runtime-link.json"),
        &json!({
          "schema_version": 0,
          "packages": [{
            "id": "pkg::a@1.0.0",
            "digest": digest,
            "artifact_path": "store/pkg-a.wasm"
          }],
          "bindings": [{
            "import_module": "pkg::a",
            "import_name": "add",
            "provider_package_id": "pkg::a@1.0.0"
          }]
        }),
    )?;
    write_json_pretty(
        &runtime_dir.join("clg.lock.json"),
        &json!({
          "schema_version": 1,
          "packages": [{
            "id": "pkg::a@1.0.0",
            "digest": digest
          }]
        }),
    )?;
    write_json_pretty(
        &runtime_dir.join("clg.package-store-index.json"),
        &json!({
          "schema_version": 0,
          "artifacts": [{
            "id": "pkg::a@1.0.0",
            "digest": digest,
            "path": "store/pkg-a.wasm"
          }]
        }),
    )?;

    env::set_var(
        "CLG_SUPPLY_CHAIN_METADATA_JSON",
        metadata_path.to_string_lossy().to_string(),
    );
    env::set_var(
        "CLG_SUPPLY_CHAIN_TRACKED_METADATA",
        normalize_rel_path(&temp_root, &package_metadata_path),
    );
    env::set_var(
        "CLG_SUPPLY_CHAIN_RUNTIME_LINKS",
        normalize_rel_path(&temp_root, &runtime_dir.join("clg.runtime-link.json")),
    );
    env::set_var(
        "CLG_SUPPLY_CHAIN_OUT_DIR",
        temp_root.join("out").to_string_lossy().to_string(),
    );
    if run_milestone2_supply_chain_gate(&temp_root, Vec::new()).is_ok() {
        return Err(
            "supply-chain self-test failed: expected failure when dependency license is missing"
                .into(),
        );
    }
    write_json_pretty(
        &metadata_path,
        &json!({
          "packages": [{
            "id": "pkg_a 1.0.0 (path+file:///pkg_a)",
            "name": "pkg_a",
            "version": "1.0.0",
            "license": "MIT",
            "license_file": ""
          }],
          "workspace_members": ["pkg_a 1.0.0 (path+file:///pkg_a)"],
          "resolve": { "nodes": [{ "id": "pkg_a 1.0.0 (path+file:///pkg_a)" }] }
        }),
    )?;
    run_milestone2_supply_chain_gate(&temp_root, Vec::new())?;
    Ok(())
}
