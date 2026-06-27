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
    emit_std_core_artifact_bundle(root, version.as_str(), out_dir.as_path())?;
    Ok(())
}

fn publish_shared_std_artifact(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_shared_std_publish_args(raw_args)?;
    let out_dir = opts.out_dir.unwrap_or_else(|| {
        root.join("dist")
            .join("shared-std")
            .join("std-core")
            .join(&opts.version)
    });
    fs::create_dir_all(&out_dir).map_err(|e| format!("create `{}`: {e}", out_dir.display()))?;
    let bundle = emit_std_core_artifact_bundle(root, opts.version.as_str(), out_dir.as_path())?;

    let signing_key = load_shared_std_signing_key()?;
    let signature_payload = canonical_package_signature_payload(
        bundle.package_id.as_str(),
        bundle.version.as_str(),
        bundle.artifact_digest.as_str(),
        opts.signed_at.as_str(),
    );
    let signature_hex = hex::encode(signing_key.sign(signature_payload.as_bytes()).to_bytes());
    let signatures_name = format!("std-core-{}.package-signatures.json", bundle.version);
    let signatures_path = out_dir.join(&signatures_name);
    let signatures = SharedStdPackageSignaturesFile {
        schema_version: 0,
        signatures: vec![SharedStdPackageSignatureEntry {
            name: bundle.package_id.clone(),
            version: bundle.version.clone(),
            digest: bundle.artifact_digest.clone(),
            key_id: opts.key_id.clone(),
            signed_at: opts.signed_at.clone(),
            signature_format: "ed25519".to_string(),
            signature: signature_hex.clone(),
        }],
    };
    write_json_pretty(&signatures_path, &signatures)?;

    let pubkey_name = format!("std-core-{}.pubkey.json", bundle.version);
    let pubkey_path = out_dir.join(&pubkey_name);
    write_json_pretty(
        &pubkey_path,
        &serde_json::json!({
            "schema_version": 1,
            "key_id": opts.key_id,
            "scheme": "ed25519",
            "public_key": hex::encode(signing_key.verifying_key().to_bytes())
        }),
    )?;

    let verified_std_abi = SharedStdVerifiedAbiDescriptor {
        major: opts.abi_major,
        minor_min: opts.abi_minor_min,
        minor_max: opts.abi_minor_max,
    };
    let algorithm = "ed25519".to_string();
    let artifact = SharedStdArtifactDescriptor {
        format: "wasm".to_string(),
        path: bundle.artifact_name.clone(),
        digest: bundle.artifact_digest.clone(),
        size_bytes: bundle.artifact_size_bytes,
    };
    let registry_mode = if opts.registry_dir.is_some() {
        "file_registry"
    } else {
        "unpublished"
    };
    let provenance_name = format!("std-core-{}.provenance.json", bundle.version);
    let provenance_path = out_dir.join(&provenance_name);
    let provenance = SharedStdProvenanceStatement {
        schema_version: 1,
        statement_type: opts.statement_format.clone(),
        package_id: bundle.package_id.clone(),
        version: bundle.version.clone(),
        verified_std_abi: verified_std_abi.clone(),
        artifact: artifact.clone(),
        metadata: SharedStdProvenanceMetadata {
            manifest_path: bundle.manifest_name.clone(),
            manifest_digest: file_sha256_prefixed(bundle.manifest_path.as_path())?,
            package_metadata_path: bundle.strict_meta_name.clone(),
            package_metadata_digest: file_sha256_prefixed(bundle.strict_meta_path.as_path())?,
            package_abi_path: bundle.strict_abi_name.clone(),
            package_abi_digest: file_sha256_prefixed(bundle.strict_abi_path.as_path())?,
            package_signatures_path: signatures_name.clone(),
        },
        signature: SharedStdProvenanceSignatureRef {
            key_id: opts.key_id.clone(),
            algorithm: algorithm.clone(),
            signed_at: opts.signed_at.clone(),
        },
        registry: SharedStdProvenanceRegistry {
            mode: registry_mode.to_string(),
            registry_dir: opts
                .registry_dir
                .as_ref()
                .map(|path| path.display().to_string()),
        },
    };
    write_json_pretty(&provenance_path, &provenance)?;
    let provenance_digest = file_sha256_prefixed(provenance_path.as_path())?;

    let published_package_name = format!("std-core-{}.shared-std-package.json", bundle.version);
    let published_package_path = out_dir.join(&published_package_name);
    let published_package = SharedStdPublishedPackage {
        schema_version: 1,
        package_id: bundle.package_id.clone(),
        version: bundle.version.clone(),
        verified_std_abi: verified_std_abi.clone(),
        artifact: artifact.clone(),
        signature: SharedStdPublishedPackageSignature {
            key_id: opts.key_id.clone(),
            algorithm: algorithm.clone(),
            signed_at: opts.signed_at.clone(),
            signature: signature_hex,
        },
        provenance: SharedStdPublishedProvenance {
            statement_digest: provenance_digest,
            statement_format: opts.statement_format.clone(),
        },
        symbols: bundle.symbols.clone(),
        dependencies: Vec::new(),
    };
    write_json_pretty(&published_package_path, &published_package)?;

    let mut files = vec![
        publish_file_entry(bundle.artifact_path.as_path(), out_dir.as_path())?,
        publish_file_entry(bundle.surface_path.as_path(), out_dir.as_path())?,
        publish_file_entry(bundle.strict_meta_path.as_path(), out_dir.as_path())?,
        publish_file_entry(bundle.strict_abi_path.as_path(), out_dir.as_path())?,
        publish_file_entry(bundle.manifest_path.as_path(), out_dir.as_path())?,
        publish_file_entry(bundle.digest_path.as_path(), out_dir.as_path())?,
        publish_file_entry(signatures_path.as_path(), out_dir.as_path())?,
        publish_file_entry(pubkey_path.as_path(), out_dir.as_path())?,
        publish_file_entry(provenance_path.as_path(), out_dir.as_path())?,
        publish_file_entry(published_package_path.as_path(), out_dir.as_path())?,
    ];
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let publish_manifest_name = format!("std-core-{}.publish.json", bundle.version);
    let publish_manifest_path = out_dir.join(&publish_manifest_name);
    let publish_manifest = SharedStdPublishManifest {
        schema_version: 1,
        package_id: bundle.package_id.clone(),
        version: bundle.version.clone(),
        verified_std_abi,
        key_id: opts.key_id.clone(),
        signed_at: opts.signed_at.clone(),
        statement_format: opts.statement_format,
        package_manifest: published_package_name,
        provenance_statement: provenance_name,
        public_key: pubkey_name,
        registry_mode: registry_mode.to_string(),
        registry_dir: opts
            .registry_dir
            .as_ref()
            .map(|path| path.display().to_string()),
        files: files.clone(),
    };
    write_json_pretty(&publish_manifest_path, &publish_manifest)?;

    if let Some(registry_dir) = opts.registry_dir {
        let registry_target = registry_dir
            .join(sanitize_package_dir_name(bundle.package_id.as_str()))
            .join(bundle.version.as_str());
        fs::create_dir_all(&registry_target)
            .map_err(|e| format!("create `{}`: {e}", registry_target.display()))?;
        for entry in &files {
            let source = out_dir.join(&entry.path);
            let dest = registry_target.join(&entry.path);
            fs::copy(&source, &dest).map_err(|e| {
                format!(
                    "copy `{}` -> `{}`: {e}",
                    source.display(),
                    dest.display()
                )
            })?;
        }
        fs::copy(&publish_manifest_path, registry_target.join(&publish_manifest_name)).map_err(
            |e| {
                format!(
                    "copy `{}` -> `{}`: {e}",
                    publish_manifest_path.display(),
                    registry_target.join(&publish_manifest_name).display()
                )
            },
        )?;
    }

    println!(
        "published shared std package {}@{} into {}",
        bundle.package_id,
        bundle.version,
        out_dir.display()
    );
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

fn load_shared_std_signing_key() -> Result<SigningKey, String> {
    let raw = env::var(SHARED_STD_SIGNING_KEY_ENV).map_err(|_| {
        format!(
            "missing `{SHARED_STD_SIGNING_KEY_ENV}` env var (expected 32-byte Ed25519 private key hex)"
        )
    })?;
    let bytes = decode_fixed_hex32(raw.trim()).ok_or_else(|| {
        format!(
            "`{SHARED_STD_SIGNING_KEY_ENV}` must be a lowercase 32-byte hex key (64 hex chars)"
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

fn parse_shared_std_publish_args(raw_args: Vec<String>) -> Result<SharedStdPublishOpts, String> {
    let mut version = "1.0.0".to_string();
    let mut out_dir: Option<PathBuf> = None;
    let mut registry_dir: Option<PathBuf> = None;
    let mut key_id = DEFAULT_SHARED_STD_KEY_ID.to_string();
    let mut signed_at: Option<String> = None;
    let mut statement_format = "in-toto-v1".to_string();
    let mut abi_major = 1u32;
    let mut abi_minor_min = 0u32;
    let mut abi_minor_max = 0u32;

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
            "--registry-dir" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--registry-dir`".to_string())?;
                registry_dir = Some(PathBuf::from(value));
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
            "--signed-at" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--signed-at`".to_string())?;
                validate_utc_rfc3339_z(value)?;
                signed_at = Some(value.clone());
            }
            "--statement-format" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--statement-format`".to_string())?;
                if value.trim().is_empty() {
                    return Err("`--statement-format` cannot be empty".to_string());
                }
                statement_format = value.trim().to_string();
            }
            "--abi-major" => {
                idx += 1;
                abi_major = parse_u32_arg(raw_args.get(idx), "--abi-major")?;
            }
            "--abi-minor-min" => {
                idx += 1;
                abi_minor_min = parse_u32_arg(raw_args.get(idx), "--abi-minor-min")?;
            }
            "--abi-minor-max" => {
                idx += 1;
                abi_minor_max = parse_u32_arg(raw_args.get(idx), "--abi-minor-max")?;
            }
            other => {
                return Err(format!(
                    "unknown shared-std-publish arg `{other}` (supported: --version, --out-dir, --registry-dir, --key-id, --signed-at, --statement-format, --abi-major, --abi-minor-min, --abi-minor-max)"
                ));
            }
        }
        idx += 1;
    }

    if abi_minor_min > abi_minor_max {
        return Err(format!(
            "`--abi-minor-min` ({abi_minor_min}) must be <= `--abi-minor-max` ({abi_minor_max})"
        ));
    }

    Ok(SharedStdPublishOpts {
        version,
        out_dir,
        registry_dir,
        key_id,
        signed_at: signed_at.ok_or_else(|| "missing required `--signed-at`".to_string())?,
        statement_format,
        abi_major,
        abi_minor_min,
        abi_minor_max,
    })
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

fn validate_utc_rfc3339_z(value: &str) -> Result<(), String> {
    let valid = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
        .expect("valid regex")
        .is_match(value);
    if valid {
        Ok(())
    } else {
        Err(format!(
            "invalid timestamp `{value}`: expected UTC RFC3339 form `YYYY-MM-DDTHH:MM:SSZ`"
        ))
    }
}

fn parse_u32_arg(value: Option<&String>, flag: &str) -> Result<u32, String> {
    let value = value.ok_or_else(|| format!("missing value for `{flag}`"))?;
    value
        .parse::<u32>()
        .map_err(|_| format!("`{flag}` must be a non-negative integer"))
}

fn canonical_package_signature_payload(
    name: &str,
    version: &str,
    digest: &str,
    signed_at: &str,
) -> String {
    format!("clg-package-signature-v0\n{name}\n{version}\n{digest}\n{signed_at}\n")
}

fn file_sha256_prefixed(path: &Path) -> Result<String, String> {
    Ok(format!("sha256:{}", file_sha256_hex(path)?))
}

fn sanitize_package_dir_name(package_id: &str) -> String {
    package_id.replace("::", "__")
}

fn publish_file_entry(path: &Path, root: &Path) -> Result<SharedStdPublishFileEntry, String> {
    let digest = file_sha256_prefixed(path)?;
    let size_bytes = fs::metadata(path)
        .map_err(|e| format!("read `{}` metadata: {e}", path.display()))?
        .len();
    let relative = path
        .strip_prefix(root)
        .map_err(|e| format!("strip prefix `{}` from `{}`: {e}", root.display(), path.display()))?
        .to_string_lossy()
        .replace('\\', "/");
    Ok(SharedStdPublishFileEntry {
        path: relative,
        digest,
        size_bytes,
    })
}

#[derive(Clone, Debug)]
struct StdCoreArtifactBundle {
    package_id: String,
    version: String,
    artifact_name: String,
    artifact_path: PathBuf,
    artifact_digest: String,
    artifact_size_bytes: u64,
    surface_path: PathBuf,
    strict_meta_path: PathBuf,
    strict_abi_path: PathBuf,
    manifest_path: PathBuf,
    digest_path: PathBuf,
    manifest_name: String,
    strict_meta_name: String,
    strict_abi_name: String,
    symbols: Vec<String>,
}

fn emit_std_core_artifact_bundle(
    root: &Path,
    version: &str,
    out_dir: &Path,
) -> Result<StdCoreArtifactBundle, String> {
    fs::create_dir_all(out_dir).map_err(|e| format!("create output dir: {e}"))?;

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
    let imports = core_modules_to_abi_imports(&core_modules);
    let mut symbols = imports.iter().map(|entry| entry.symbol.clone()).collect::<Vec<_>>();
    symbols.sort();

    let surface_name = format!("std-core-{version}.surface.json");
    let surface_path = out_dir.join(&surface_name);
    let surface = StdCoreSurfaceMetadata {
        schema_version: 0,
        package: "std::core".to_string(),
        version: version.to_string(),
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
            version: version.to_string(),
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
            version: version.to_string(),
            imports,
        }],
    };
    write_json_pretty(&strict_abi_path, &strict_abi)?;

    let manifest_name = format!("std-core-{version}.manifest.json");
    let manifest_path = out_dir.join(&manifest_name);
    let manifest = StdCoreArtifactManifest {
        schema_version: 0,
        package: "std::core".to_string(),
        version: version.to_string(),
        artifact: ArtifactEntry {
            file: artifact_name.clone(),
            digest: digest.clone(),
        },
        metadata: MetadataEntry {
            surface: surface_name,
            strict_package_metadata: strict_meta_name.clone(),
            strict_package_abi: strict_abi_name.clone(),
        },
    };
    write_json_pretty(&manifest_path, &manifest)?;

    let digest_name = format!("std-core-{version}.sha256");
    let digest_path = out_dir.join(&digest_name);
    fs::write(&digest_path, format!("{digest}\n"))
        .map_err(|e| format!("write digest file `{}`: {e}", digest_path.display()))?;

    Ok(StdCoreArtifactBundle {
        package_id: "std::core".to_string(),
        version: version.to_string(),
        artifact_name,
        artifact_path,
        artifact_digest: digest,
        artifact_size_bytes: wasm_bytes.len() as u64,
        surface_path,
        strict_meta_path,
        strict_abi_path,
        manifest_path,
        digest_path,
        manifest_name,
        strict_meta_name,
        strict_abi_name,
        symbols,
    })
}
