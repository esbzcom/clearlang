fn validate_samples(root: &Path) -> Result<(), String> {
    let tmp = root.join("tmp").join("ci");
    fs::create_dir_all(&tmp).map_err(|e| format!("create tmp dir: {e}"))?;
    let samples = [
        "01_hello.clear",
        "02_arith.clear",
        "03_nested_calls.clear",
        "04_multiline_call.clear",
        "05_trailing_param_comma.clear",
        "08_main_const.clear",
        "18_return_simple.clear",
    ];
    for sample in samples {
        let out = tmp.join(sample.replace(".clear", ".wasm"));
        let sample_path = root.join("clearlang-tests").join(sample);
        run(Command::new("cargo")
            .arg("run")
            .arg("-p")
            .arg("clg-cli")
            .arg("--")
            .arg("build")
            .arg(&sample_path)
            .arg("-o")
            .arg(&out)
            .current_dir(root))?;
        run(Command::new("wasm-tools")
            .arg("validate")
            .arg(&out)
            .current_dir(root))?;
    }
    run(Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("clg-cli")
        .arg("--")
        .arg("emit-hello")
        .arg("-o")
        .arg(tmp.join("emit_hello.wasm"))
        .current_dir(root))?;
    run(Command::new("wasm-tools")
        .arg("validate")
        .arg(tmp.join("emit_hello.wasm"))
        .current_dir(root))?;
    Ok(())
}

fn emit_vcs_sample(root: &Path) -> Result<(), String> {
    let tmp = root.join("tmp").join("xtask");
    fs::create_dir_all(&tmp).map_err(|e| format!("create tmp dir: {e}"))?;
    let src = r#"
        pure function inc(x: Int) -> Int
            require { x >= 0 }
            ensure { result > x }
        { x + 1 }
        function main() -> Int { inc(1) }
    "#;
    let src_path = tmp.join("contract.clear");
    let wasm_path = tmp.join("contract.wasm");
    let vcs_path = tmp.join("contract.vc.json");
    fs::write(&src_path, src).map_err(|e| format!("write source: {e}"))?;
    run(Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("clg-cli")
        .arg("--")
        .arg("build")
        .arg(&src_path)
        .arg("-o")
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path)
        .current_dir(root))?;
    Ok(())
}

fn emit_std_core_artifact(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_std_core_args(raw_args)?;
    let version = opts.version;
    let out_dir = opts
        .out_dir
        .unwrap_or_else(|| root.join("dist").join("std-core").join(&version));
    fs::create_dir_all(&out_dir).map_err(|e| format!("create output dir: {e}"))?;

    let artifact_name = format!("std-core-{version}.wasm");
    let artifact_path = out_dir.join(&artifact_name);
    let wasm_bytes = minimal_wasm_module_bytes();
    fs::write(&artifact_path, &wasm_bytes)
        .map_err(|e| format!("write std-core artifact `{}`: {e}", artifact_path.display()))?;
    let digest = format!("sha256:{}", hex::encode(Sha256::digest(&wasm_bytes)));

    let std_metadata_path = root
        .join("crates")
        .join("cli")
        .join("assets")
        .join("std-metadata.json");
    let std_metadata_raw = fs::read_to_string(&std_metadata_path)
        .map_err(|e| format!("read std metadata `{}`: {e}", std_metadata_path.display()))?;
    let std_metadata: StdMetadataRoot = serde_json::from_str(&std_metadata_raw)
        .map_err(|e| format!("parse std metadata `{}`: {e}", std_metadata_path.display()))?;
    let core_modules = select_core_modules(std_metadata.modules);

    let surface_name = format!("std-core-{version}.surface.json");
    let surface_path = out_dir.join(&surface_name);
    let surface = StdCoreSurfaceMetadata {
        schema_version: 0,
        package: "std::core".to_string(),
        version: version.clone(),
        source_std_metadata_schema_version: std_metadata.schema_version,
        modules: core_modules.clone(),
    };
    write_json_pretty(&surface_path, &surface)?;

    let strict_meta_name = format!("std-core-{version}.package-metadata.json");
    let strict_meta_path = out_dir.join(&strict_meta_name);
    let strict_metadata = StrictPackageMetadataFile {
        schema_version: 0,
        packages: vec![StrictPackageMetadataEntry {
            name: "std::core".to_string(),
            version: version.clone(),
            digest: digest.clone(),
            artifact: StrictPackageArtifact {
                format: "wasm".to_string(),
                path: artifact_name.clone(),
            },
            abi_id: format!("abi:std::core:{version}"),
        }],
    };
    write_json_pretty(&strict_meta_path, &strict_metadata)?;

    let strict_abi_name = format!("std-core-{version}.package-abi.json");
    let strict_abi_path = out_dir.join(&strict_abi_name);
    let strict_abi = StrictAbiFile {
        schema_version: 0,
        contracts: vec![StrictAbiContract {
            abi_id: format!("abi:std::core:{version}"),
            package: "std::core".to_string(),
            version: version.clone(),
            imports: core_modules_to_abi_imports(&core_modules),
        }],
    };
    write_json_pretty(&strict_abi_path, &strict_abi)?;

    let manifest_name = format!("std-core-{version}.manifest.json");
    let manifest_path = out_dir.join(&manifest_name);
    let manifest = StdCoreArtifactManifest {
        schema_version: 0,
        package: "std::core".to_string(),
        version: version.clone(),
        artifact: ArtifactEntry {
            file: artifact_name,
            digest: digest.clone(),
        },
        metadata: MetadataEntry {
            surface: surface_name,
            strict_package_metadata: strict_meta_name,
            strict_package_abi: strict_abi_name,
        },
    };
    write_json_pretty(&manifest_path, &manifest)?;

    let digest_name = format!("std-core-{version}.sha256");
    let digest_path = out_dir.join(digest_name);
    fs::write(&digest_path, format!("{digest}\n"))
        .map_err(|e| format!("write digest file `{}`: {e}", digest_path.display()))?;
    Ok(())
}

fn stage_solver_vendor(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_solver_vendor_stage_args(raw_args)?;
    if !opts.from.is_file() {
        return Err(format!(
            "solver vendor source `{}` is not a file",
            opts.from.display()
        ));
    }
    let solver_file = match opts.platform.as_str() {
        "windows" => "z3.exe",
        "linux" | "macos" => "z3",
        other => {
            return Err(format!(
                "unsupported solver vendor platform `{other}` (supported: windows, linux, macos)"
            ));
        }
    };
    let out_dir = root
        .join("tools")
        .join("proof")
        .join("z3")
        .join(opts.platform.as_str());
    fs::create_dir_all(&out_dir).map_err(|e| format!("create `{}`: {e}", out_dir.display()))?;
    let out = out_dir.join(solver_file);
    fs::copy(&opts.from, &out).map_err(|e| {
        format!(
            "copy solver from `{}` to `{}`: {e}",
            opts.from.display(),
            out.display()
        )
    })?;
    let solver_bytes = fs::read(&out).map_err(|e| format!("read `{}`: {e}", out.display()))?;
    let checksum = format!("sha256:{}", hex::encode(Sha256::digest(&solver_bytes)));
    let checksum_path = sidecar_path(out.as_path(), "sha256");
    fs::write(&checksum_path, format!("{checksum}\n"))
        .map_err(|e| format!("write `{}`: {e}", checksum_path.display()))?;
    let signing_key = load_solver_vendor_signing_key()?;
    let signature = hex::encode(signing_key.sign(checksum.as_bytes()).to_bytes());
    let signature_payload = serde_json::json!({
        "schema_version": 1,
        "key_id": opts.key_id,
        "scheme": "ed25519",
        "signed_payload": checksum,
        "signature": signature
    });
    let signature_path = sidecar_path(out.as_path(), "sig");
    let mut signature_bytes = serde_json::to_vec_pretty(&signature_payload)
        .map_err(|e| format!("serialize signature sidecar: {e}"))?;
    signature_bytes.push(b'\n');
    fs::write(&signature_path, signature_bytes)
        .map_err(|e| format!("write `{}`: {e}", signature_path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&out)
            .map_err(|e| format!("read `{}` metadata: {e}", out.display()))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&out, perms)
            .map_err(|e| format!("set `{}` executable bit: {e}", out.display()))?;
    }
    println!(
        "staged solver vendor binary: {} (platform={}) checksum={} signature={} key_id={}",
        out.display(),
        opts.platform,
        checksum_path.display(),
        signature_path.display(),
        opts.key_id
    );
    Ok(())
}

fn sidecar_path(solver_bin: &Path, suffix: &str) -> PathBuf {
    let mut name = solver_bin
        .file_name()
        .unwrap_or_else(|| OsStr::new("solver"))
        .to_os_string();
    name.push(format!(".{suffix}"));
    solver_bin
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(name)
}

fn parse_solver_vendor_stage_args(raw_args: Vec<String>) -> Result<SolverVendorStageOpts, String> {
    let mut from: Option<PathBuf> = None;
    let mut platform = "windows".to_string();
    let mut key_id = DEFAULT_SOLVER_VENDOR_KEY_ID.to_string();
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--from" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--from`".to_string())?;
                from = Some(PathBuf::from(value));
            }
            "--platform" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--platform`".to_string())?;
                platform = value.to_ascii_lowercase();
            }
            "--key-id" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--key-id`".to_string())?;
                if value.trim().is_empty() {
                    return Err("`--key-id` cannot be empty".to_string());
                }
                key_id = value.trim().to_string();
            }
            other => {
                return Err(format!(
                    "unknown solver-vendor-stage arg `{other}` (supported: --from, --platform, --key-id)"
                ));
            }
        }
        idx += 1;
    }
    let from = from.ok_or_else(|| "missing required `--from`".to_string())?;
    Ok(SolverVendorStageOpts {
        from,
        platform,
        key_id,
    })
}

fn load_solver_vendor_signing_key() -> Result<SigningKey, String> {
    let raw = env::var(SOLVER_VENDOR_SIGNING_KEY_ENV).map_err(|_| {
        format!(
            "missing `{SOLVER_VENDOR_SIGNING_KEY_ENV}` env var (expected 32-byte Ed25519 private key hex)"
        )
    })?;
    let bytes = decode_fixed_hex32(raw.trim()).ok_or_else(|| {
        format!(
            "`{SOLVER_VENDOR_SIGNING_KEY_ENV}` must be a lowercase 32-byte hex key (64 hex chars)"
        )
    })?;
    Ok(SigningKey::from_bytes(&bytes))
}

fn load_binary_release_signing_key() -> Result<SigningKey, String> {
    let raw = env::var(BINARY_RELEASE_SIGNING_KEY_ENV).map_err(|_| {
        format!(
            "missing `{BINARY_RELEASE_SIGNING_KEY_ENV}` env var (expected 32-byte Ed25519 private key hex)"
        )
    })?;
    let bytes = decode_fixed_hex32(raw.trim()).ok_or_else(|| {
        format!(
            "`{BINARY_RELEASE_SIGNING_KEY_ENV}` must be a lowercase 32-byte hex key (64 hex chars)"
        )
    })?;
    Ok(SigningKey::from_bytes(&bytes))
}

fn decode_fixed_hex32(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64
        || value
            .bytes()
            .any(|b| !b.is_ascii_hexdigit() || b.is_ascii_uppercase())
    {
        return None;
    }
    let raw = hex::decode(value).ok()?;
    raw.try_into().ok()
}

fn parse_std_core_args(raw_args: Vec<String>) -> Result<StdCoreArtifactOpts, String> {
    let mut version = "1.0.0".to_string();
    let mut out_dir: Option<PathBuf> = None;

    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--version" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--version`".to_string())?;
                validate_semver(value)?;
                version = value.clone();
            }
            "--out-dir" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--out-dir`".to_string())?;
                out_dir = Some(PathBuf::from(value));
            }
            other => {
                return Err(format!(
                    "unknown std-core-artifact arg `{other}` (supported: --version, --out-dir)"
                ));
            }
        }
        idx += 1;
    }

    Ok(StdCoreArtifactOpts { version, out_dir })
}

fn validate_semver(version: &str) -> Result<(), String> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return Err(format!(
            "invalid version `{version}`: expected `MAJOR.MINOR.PATCH`"
        ));
    }
    if parts
        .iter()
        .any(|part| part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()))
    {
        return Err(format!(
            "invalid version `{version}`: expected numeric `MAJOR.MINOR.PATCH`"
        ));
    }
    Ok(())
}
