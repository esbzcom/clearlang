use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SOLVER_VENDOR_SIGNING_KEY_ENV: &str = "CLG_SOLVER_VENDOR_SIGNING_KEY_HEX";
const DEFAULT_SOLVER_VENDOR_KEY_ID: &str = "z3-vendor-k7-2026q2";
const BINARY_RELEASE_SIGNING_KEY_ENV: &str = "CLG_BINARY_RELEASE_SIGNING_KEY_HEX";
const DEFAULT_BINARY_RELEASE_KEY_ID: &str = "milestone3-binary-ed25519-2026q2";

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "help".to_string());
    let root = repo_root();

    match cmd.as_str() {
        "fmt" => cargo_cmd(&root, &["fmt", "--all"])?,
        "clippy" => cargo_cmd(
            &root,
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        )?,
        "test" => cargo_cmd(&root, &["test", "--workspace"])?,
        "release-precheck" => run_release_precheck(&root)?,
        "validate" => validate_samples(&root)?,
        "emit-vcs" => emit_vcs_sample(&root)?,
        "std-core-artifact" => emit_std_core_artifact(&root, args.collect())?,
        "solver-vendor-stage" => stage_solver_vendor(&root, args.collect())?,
        "manifest-lock-drift-check" => run_manifest_lock_drift_gate(&root, args.collect())?,
        "std-surface-drift-check" => check_std_surface_drift(&root, args.collect())?,
        "std-arch-sync" => run_std_arch_sync(&root, args.collect())?,
        "std-arch-conformance-check" => run_std_arch_conformance_check(&root, args.collect())?,
        "std-first-production-readiness-check" => {
            run_std_first_production_readiness_check(&root, args.collect())?
        }
        "host-capability-policy-artifact" => {
            emit_host_capability_policy_artifact(&root, args.collect())?
        }
        "binary-repro-witness" => run_binary_repro_witness(&root, args.collect())?,
        "milestone3-binary-bundle" => emit_milestone3_binary_bundle(&root, args.collect())?,
        "milestone3-binary-bundle-verify" => {
            verify_milestone3_binary_bundle(&root, args.collect())?
        }
        "milestone2-perf-gate" => run_milestone2_perf_gate(&root, args.collect())?,
        "milestone2-supply-chain-gate" => run_milestone2_supply_chain_gate(&root, args.collect())?,
        "ci" => {
            run_release_precheck(&root)?;
            cargo_cmd(&root, &["build", "--release", "-p", "clg-cli"])?;
            validate_samples(&root)?;
        }
        "help" | "-h" | "--help" => {
            print_help();
        }
        other => return Err(format!("unknown xtask command: {other}")),
    }

    Ok(())
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask in repo root/xtask")
        .to_path_buf()
}

fn cargo_cmd(root: &Path, args: &[&str]) -> Result<(), String> {
    run(Command::new("cargo").args(args).current_dir(root))
}

fn run_release_precheck(root: &Path) -> Result<(), String> {
    cargo_cmd(root, &["fmt", "--all", "--", "--check"])?;
    cargo_cmd(
        root,
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    cargo_cmd(root, &["test", "--workspace"])?;
    run_manifest_lock_drift_gate(root, Vec::new())?;
    run_std_arch_conformance_check(root, Vec::new())?;
    run_clg_test_schema_gate(root)?;
    Ok(())
}

fn run_manifest_lock_drift_gate(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_manifest_lock_drift_args(raw_args)?;
    if opts.paths.is_empty() {
        return Err("manifest-lock drift gate requires at least one check path".to_string());
    }
    for relative in &opts.paths {
        let dir = root.join(relative);
        check_manifest_lock_consistency_for_dir(&dir).map_err(|err| {
            format!(
                "manifest/lock drift at `{}`: {err}",
                dir.strip_prefix(root).unwrap_or(dir.as_path()).display()
            )
        })?;
    }
    Ok(())
}

fn run_binary_repro_witness(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_binary_repro_witness_args(raw_args)?;
    let out_path = opts.out.unwrap_or_else(|| {
        root.join("tmp")
            .join("binary-repro")
            .join(format!("{}.json", current_platform_id()))
    });
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create binary repro output dir `{}`: {e}", parent.display()))?;
    }

    let run1_dir = root.join("tmp").join("binary-repro").join("run1-target");
    let run2_dir = root.join("tmp").join("binary-repro").join("run2-target");
    if run1_dir.exists() {
        fs::remove_dir_all(&run1_dir)
            .map_err(|e| format!("cleanup `{}`: {e}", run1_dir.display()))?;
    }
    if run2_dir.exists() {
        fs::remove_dir_all(&run2_dir)
            .map_err(|e| format!("cleanup `{}`: {e}", run2_dir.display()))?;
    }

    run(Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("-p")
        .arg("clg-cli")
        .arg("--target-dir")
        .arg(&run1_dir)
        .current_dir(root))?;
    run(Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("-p")
        .arg("clg-cli")
        .arg("--target-dir")
        .arg(&run2_dir)
        .current_dir(root))?;

    let bin1 = clg_binary_path(&run1_dir);
    let bin2 = clg_binary_path(&run2_dir);
    let hash1 = file_sha256_hex(&bin1)?;
    let hash2 = file_sha256_hex(&bin2)?;
    let equal = hash1 == hash2;

    let artifact = serde_json::json!({
        "schema_version": 1,
        "platform": current_platform_id(),
        "binary_name": bin1.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| "clg".to_string()),
        "run1": {
            "path": bin1.display().to_string(),
            "sha256": hash1,
        },
        "run2": {
            "path": bin2.display().to_string(),
            "sha256": hash2,
        },
        "equal": equal,
    });
    write_json_pretty(&out_path, &artifact)?;
    if !equal {
        return Err(format!(
            "binary reproducibility mismatch on {}: `{}` vs `{}`",
            current_platform_id(),
            bin1.display(),
            bin2.display()
        ));
    }
    println!(
        "binary reproducibility witness written to {}",
        out_path.display()
    );
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Milestone3BundleArtifact {
    path: String,
    sha256: String,
    size_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Milestone3BundlePayload {
    schema_version: u32,
    kind: String,
    platform: String,
    key_id: String,
    binary: Milestone3BundleArtifact,
    checksums_manifest: Milestone3BundleArtifact,
    sbom_artifacts: Vec<Milestone3BundleArtifact>,
    license_artifacts: Vec<Milestone3BundleArtifact>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Milestone3BundleSignature {
    key_id: String,
    signature_format: String,
    payload_hash: String,
    signature: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Milestone3SignedBundleMetadata {
    schema_version: u32,
    payload: Milestone3BundlePayload,
    signature: Milestone3BundleSignature,
}

#[derive(Clone, Debug, Deserialize)]
struct Milestone3BundlePubkeyMetadata {
    schema_version: u32,
    key_id: String,
    scheme: String,
    public_key: String,
}

fn emit_milestone3_binary_bundle(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_milestone3_binary_bundle_args(raw_args)?;
    let out_dir = opts.out_dir.clone().unwrap_or_else(|| {
        root.join("tmp")
            .join("milestone3-binary")
            .join(opts.platform.as_str())
    });
    if out_dir.exists() {
        fs::remove_dir_all(&out_dir).map_err(|e| {
            format!(
                "cleanup milestone3 binary bundle dir `{}`: {e}",
                out_dir.display()
            )
        })?;
    }
    fs::create_dir_all(&out_dir).map_err(|e| {
        format!(
            "create milestone3 binary bundle dir `{}`: {e}",
            out_dir.display()
        )
    })?;

    let binary_source =
        resolve_milestone3_binary_source(root, opts.binary.as_ref(), opts.platform.as_str())?;
    let binary_name = binary_source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("clg")
        .to_string();
    let bin_dir = out_dir.join("bin").join(opts.platform.as_str());
    let checksums_dir = out_dir.join("checksums");
    let metadata_dir = out_dir.join("metadata");
    let sbom_dir = out_dir.join("sbom");
    let licenses_dir = out_dir.join("licenses");
    for dir in [
        &bin_dir,
        &checksums_dir,
        &metadata_dir,
        &sbom_dir,
        &licenses_dir,
    ] {
        fs::create_dir_all(dir).map_err(|e| format!("create `{}`: {e}", dir.display()))?;
    }

    let binary_dest = bin_dir.join(&binary_name);
    fs::copy(&binary_source, &binary_dest).map_err(|e| {
        format!(
            "copy binary from `{}` to `{}`: {e}",
            binary_source.display(),
            binary_dest.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_dest)
            .map_err(|e| format!("read `{}` metadata: {e}", binary_dest.display()))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_dest, perms)
            .map_err(|e| format!("set `{}` executable bit: {e}", binary_dest.display()))?;
    }

    run_milestone2_supply_chain_gate(
        root,
        vec![
            "--out-dir".to_string(),
            sbom_dir.to_string_lossy().to_string(),
        ],
    )?;

    let root_license = root.join("LICENSE");
    if !root_license.is_file() {
        return Err(format!(
            "missing repository LICENSE at `{}`",
            root_license.display()
        ));
    }
    let project_license_path = licenses_dir.join("clearlang-LICENSE.txt");
    fs::copy(&root_license, &project_license_path).map_err(|e| {
        format!(
            "copy repository LICENSE from `{}` to `{}`: {e}",
            root_license.display(),
            project_license_path.display()
        )
    })?;

    let third_party_summary = build_third_party_license_summary(sbom_dir.as_path())?;
    let third_party_summary_path = licenses_dir.join("third-party-licenses.json");
    write_json_pretty(&third_party_summary_path, &third_party_summary)?;

    let release_notes_src = root.join("release_notes").join("milestone_3.md");
    if release_notes_src.is_file() {
        let release_notes_dir = out_dir.join("release_notes");
        fs::create_dir_all(&release_notes_dir)
            .map_err(|e| format!("create `{}`: {e}", release_notes_dir.display()))?;
        fs::copy(&release_notes_src, release_notes_dir.join("milestone_3.md")).map_err(|e| {
            format!(
                "copy release notes from `{}`: {e}",
                release_notes_src.display()
            )
        })?;
    }

    let mut checksum_entries = Vec::<(String, String)>::new();
    let mut collect_entry = |path: &Path| -> Result<(), String> {
        let rel = normalize_rel_path(&out_dir, path);
        let sha = file_sha256_hex(path)?;
        checksum_entries.push((rel, sha));
        Ok(())
    };
    collect_entry(&binary_dest)?;
    collect_entry(&project_license_path)?;
    collect_entry(&third_party_summary_path)?;
    let sbom_files = list_json_files(sbom_dir.as_path())?;
    for sbom_file in &sbom_files {
        collect_entry(sbom_file.as_path())?;
    }

    checksum_entries.sort_by(|a, b| a.0.cmp(&b.0));
    let checksums_path = checksums_dir.join("SHA256SUMS");
    let checksums_text = checksum_entries
        .iter()
        .map(|(path, sha)| format!("{sha}  {path}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&checksums_path, format!("{checksums_text}\n"))
        .map_err(|e| format!("write `{}`: {e}", checksums_path.display()))?;

    let binary_artifact = manifest_artifact(&out_dir, binary_dest.as_path())?;
    let checksums_artifact = manifest_artifact(&out_dir, checksums_path.as_path())?;
    let sbom_artifacts = sbom_files
        .iter()
        .map(|path| manifest_artifact(&out_dir, path.as_path()))
        .collect::<Result<Vec<_>, _>>()?;
    let license_artifacts = vec![
        manifest_artifact(&out_dir, project_license_path.as_path())?,
        manifest_artifact(&out_dir, third_party_summary_path.as_path())?,
    ];
    let payload = Milestone3BundlePayload {
        schema_version: 1,
        kind: "clearlang.milestone3_binary_bundle".to_string(),
        platform: opts.platform.clone(),
        key_id: opts.key_id.clone(),
        binary: binary_artifact,
        checksums_manifest: checksums_artifact,
        sbom_artifacts,
        license_artifacts,
    };

    let signing_key = load_binary_release_signing_key()?;
    let payload_canonical =
        serde_json::to_string(&payload).map_err(|e| format!("serialize bundle payload: {e}"))?;
    let payload_hash = hex::encode(Sha256::digest(payload_canonical.as_bytes()));
    let signature = hex::encode(signing_key.sign(payload_canonical.as_bytes()).to_bytes());
    let signed = Milestone3SignedBundleMetadata {
        schema_version: 1,
        payload,
        signature: Milestone3BundleSignature {
            key_id: opts.key_id,
            signature_format: "ed25519".to_string(),
            payload_hash,
            signature,
        },
    };
    let signed_path = metadata_dir.join("milestone3-binary-bundle.signed.json");
    write_json_pretty(&signed_path, &signed)?;

    let pubkey_path = metadata_dir.join("milestone3-binary-bundle.pubkey.json");
    let pubkey_payload = serde_json::json!({
        "schema_version": 1,
        "key_id": signed.signature.key_id,
        "scheme": "ed25519",
        "public_key": hex::encode(signing_key.verifying_key().to_bytes()),
    });
    write_json_pretty(&pubkey_path, &pubkey_payload)?;

    println!(
        "milestone3 binary bundle emitted: {} (platform={}, binary={})",
        out_dir.display(),
        opts.platform,
        binary_name
    );
    Ok(())
}

fn verify_milestone3_binary_bundle(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_milestone3_binary_bundle_verify_args(raw_args)?;
    let bundle_dir = if opts.bundle_dir.is_absolute() {
        opts.bundle_dir
    } else {
        root.join(opts.bundle_dir)
    };
    let metadata_dir = bundle_dir.join("metadata");
    let signed_path = metadata_dir.join("milestone3-binary-bundle.signed.json");
    let pubkey_path = metadata_dir.join("milestone3-binary-bundle.pubkey.json");
    let checksums_path = bundle_dir.join("checksums").join("SHA256SUMS");

    let signed: Milestone3SignedBundleMetadata = serde_json::from_slice(
        &fs::read(&signed_path).map_err(|e| format!("read `{}`: {e}", signed_path.display()))?,
    )
    .map_err(|e| format!("parse `{}`: {e}", signed_path.display()))?;
    if signed.schema_version != 1 {
        return Err(format!(
            "unsupported bundle metadata schema_version {} in `{}`",
            signed.schema_version,
            signed_path.display()
        ));
    }
    if signed.payload.schema_version != 1 {
        return Err(format!(
            "unsupported bundle payload schema_version {} in `{}`",
            signed.payload.schema_version,
            signed_path.display()
        ));
    }

    let pubkey: Milestone3BundlePubkeyMetadata = serde_json::from_slice(
        &fs::read(&pubkey_path).map_err(|e| format!("read `{}`: {e}", pubkey_path.display()))?,
    )
    .map_err(|e| format!("parse `{}`: {e}", pubkey_path.display()))?;
    if pubkey.schema_version != 1 {
        return Err(format!(
            "unsupported pubkey schema_version {} in `{}`",
            pubkey.schema_version,
            pubkey_path.display()
        ));
    }
    if pubkey.scheme != "ed25519" {
        return Err(format!(
            "unsupported pubkey scheme `{}` in `{}`",
            pubkey.scheme,
            pubkey_path.display()
        ));
    }
    if pubkey.key_id != signed.signature.key_id {
        return Err(format!(
            "pubkey key_id `{}` does not match signature key_id `{}`",
            pubkey.key_id, signed.signature.key_id
        ));
    }

    let payload_canonical = serde_json::to_string(&signed.payload)
        .map_err(|e| format!("serialize bundle payload for verification: {e}"))?;
    let payload_hash = hex::encode(Sha256::digest(payload_canonical.as_bytes()));
    if payload_hash != signed.signature.payload_hash {
        return Err(format!(
            "payload hash mismatch in `{}`: expected {}, got {}",
            signed_path.display(),
            signed.signature.payload_hash,
            payload_hash
        ));
    }

    let pubkey_bytes_vec =
        hex::decode(pubkey.public_key.as_str()).map_err(|e| format!("decode public_key hex: {e}"))?;
    let pubkey_bytes: [u8; 32] = pubkey_bytes_vec
        .as_slice()
        .try_into()
        .map_err(|_| "public_key must decode to 32 bytes".to_string())?;
    let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
        .map_err(|e| format!("parse public key bytes: {e}"))?;
    let signature_bytes_vec =
        hex::decode(signed.signature.signature.as_str()).map_err(|e| format!("decode signature hex: {e}"))?;
    let signature = Signature::try_from(signature_bytes_vec.as_slice())
        .map_err(|e| format!("parse signature bytes: {e}"))?;
    verifying_key
        .verify(payload_canonical.as_bytes(), &signature)
        .map_err(|e| format!("verify signature: {e}"))?;

    let checksums_raw = fs::read_to_string(&checksums_path)
        .map_err(|e| format!("read `{}`: {e}", checksums_path.display()))?;
    let lines: Vec<&str> = checksums_raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return Err(format!(
            "checksum manifest `{}` must contain at least one checksum line",
            checksums_path.display()
        ));
    }
    let mut entries = Vec::<(&str, &str)>::new();
    for line in &lines {
        let (expected_hash, rel_path) = line.split_once("  ").ok_or_else(|| {
            format!(
                "invalid checksum manifest line `{line}` in `{}`",
                checksums_path.display()
            )
        })?;
        entries.push((expected_hash, rel_path));
    }
    let mut sorted_paths = entries.iter().map(|(_, path)| *path).collect::<Vec<_>>();
    sorted_paths.sort();
    let listed_paths = entries.iter().map(|(_, path)| *path).collect::<Vec<_>>();
    if listed_paths != sorted_paths {
        return Err(format!(
            "checksum manifest `{}` paths are not lexicographically sorted",
            checksums_path.display()
        ));
    }
    for (expected_hash, rel_path) in entries {
        let absolute = bundle_dir.join(rel_path.replace('/', &std::path::MAIN_SEPARATOR.to_string()));
        if !absolute.is_file() {
            return Err(format!(
                "checksum manifest member `{}` missing from `{}`",
                rel_path,
                bundle_dir.display()
            ));
        }
        let actual_hash = file_sha256_hex(absolute.as_path())?;
        if actual_hash != expected_hash {
            return Err(format!(
                "checksum mismatch for `{}`: expected {}, got {}",
                rel_path, expected_hash, actual_hash
            ));
        }
    }

    println!(
        "milestone3 binary bundle verified: {}",
        bundle_dir.display()
    );
    Ok(())
}

fn parse_binary_repro_witness_args(
    raw_args: Vec<String>,
) -> Result<BinaryReproWitnessOpts, String> {
    let mut out: Option<PathBuf> = None;
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--out" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--out`".to_string())?;
                out = Some(PathBuf::from(value));
            }
            other => {
                return Err(format!(
                    "unknown binary-repro-witness arg `{other}` (supported: --out)"
                ));
            }
        }
        idx += 1;
    }
    Ok(BinaryReproWitnessOpts { out })
}

fn parse_milestone3_binary_bundle_args(
    raw_args: Vec<String>,
) -> Result<Milestone3BinaryBundleOpts, String> {
    let mut platform = current_platform_id().to_string();
    let mut binary: Option<PathBuf> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut key_id = DEFAULT_BINARY_RELEASE_KEY_ID.to_string();
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--platform" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--platform`".to_string())?;
                platform = value.trim().to_ascii_lowercase();
            }
            "--binary" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--binary`".to_string())?;
                binary = Some(PathBuf::from(value));
            }
            "--out-dir" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--out-dir`".to_string())?;
                out_dir = Some(PathBuf::from(value));
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
                    "unknown milestone3-binary-bundle arg `{other}` (supported: --platform, --binary, --out-dir, --key-id)"
                ));
            }
        }
        idx += 1;
    }
    if !matches!(platform.as_str(), "windows" | "linux" | "macos") {
        return Err(format!(
            "unsupported `--platform` value `{platform}` (expected windows|linux|macos)"
        ));
    }
    Ok(Milestone3BinaryBundleOpts {
        platform,
        binary,
        out_dir,
        key_id,
    })
}

fn parse_milestone3_binary_bundle_verify_args(
    raw_args: Vec<String>,
) -> Result<Milestone3BinaryBundleVerifyOpts, String> {
    let mut bundle_dir: Option<PathBuf> = None;
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--bundle-dir" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--bundle-dir`".to_string())?;
                bundle_dir = Some(PathBuf::from(value));
            }
            other => {
                return Err(format!(
                    "unknown milestone3-binary-bundle-verify arg `{other}` (supported: --bundle-dir)"
                ));
            }
        }
        idx += 1;
    }
    let bundle_dir =
        bundle_dir.ok_or_else(|| "missing required `--bundle-dir`".to_string())?;
    Ok(Milestone3BinaryBundleVerifyOpts { bundle_dir })
}

fn clg_binary_path(target_dir: &Path) -> PathBuf {
    let file = if cfg!(windows) { "clg.exe" } else { "clg" };
    target_dir.join("release").join(file)
}

fn file_sha256_hex(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("read `{}`: {e}", path.display()))?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

fn current_platform_id() -> &'static str {
    match env::consts::OS {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "macos",
        _ => "unknown",
    }
}

fn resolve_milestone3_binary_source(
    root: &Path,
    binary: Option<&PathBuf>,
    platform: &str,
) -> Result<PathBuf, String> {
    let binary_path = match binary {
        Some(path) => path.clone(),
        None => {
            let file = if platform == "windows" {
                "clg.exe"
            } else {
                "clg"
            };
            root.join("target").join("release").join(file)
        }
    };
    if !binary_path.is_file() {
        return Err(format!(
            "missing binary `{}`; run `cargo build --release -p clg-cli` first or pass `--binary <FILE>`",
            binary_path.display()
        ));
    }
    Ok(binary_path)
}

fn manifest_artifact(root: &Path, path: &Path) -> Result<Milestone3BundleArtifact, String> {
    let sha256 = file_sha256_hex(path)?;
    let size_bytes = fs::metadata(path)
        .map_err(|e| format!("read `{}` metadata: {e}", path.display()))?
        .len();
    Ok(Milestone3BundleArtifact {
        path: normalize_rel_path(root, path),
        sha256,
        size_bytes,
    })
}

fn list_json_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("read dir `{}`: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read dir entry `{}`: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("json"))
                .unwrap_or(false)
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn build_third_party_license_summary(sbom_dir: &Path) -> Result<serde_json::Value, String> {
    let sbom_path = sbom_dir.join("milestone2-sbom.json");
    let sbom_bytes =
        fs::read(&sbom_path).map_err(|e| format!("read `{}`: {e}", sbom_path.display()))?;
    let sbom: serde_json::Value = serde_json::from_slice(&sbom_bytes)
        .map_err(|e| format!("parse `{}`: {e}", sbom_path.display()))?;
    let packages = sbom
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("SBOM `{}` missing `packages[]` array", sbom_path.display()))?;
    let mut entries = packages
        .iter()
        .map(|pkg| {
            serde_json::json!({
                "id": pkg.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "name": pkg.get("name").cloned().unwrap_or(serde_json::Value::Null),
                "version": pkg.get("version").cloned().unwrap_or(serde_json::Value::Null),
                "license": pkg.get("license").cloned().unwrap_or(serde_json::Value::Null),
                "license_file": pkg.get("license_file").cloned().unwrap_or(serde_json::Value::Null),
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        let ak = (
            a.get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
            a.get("version")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
            a.get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
        );
        let bk = (
            b.get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
            b.get("version")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
            b.get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
        );
        ak.cmp(&bk)
    });
    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "cargo_third_party_license_summary",
        "source_sbom": "sbom/milestone2-sbom.json",
        "entries": entries
    }))
}

fn parse_manifest_lock_drift_args(raw_args: Vec<String>) -> Result<ManifestLockDriftOpts, String> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--path" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--path`".to_string())?;
                paths.push(PathBuf::from(value));
            }
            other => {
                return Err(format!(
                    "unknown manifest-lock-drift-check arg `{other}` (supported: --path)"
                ));
            }
        }
        idx += 1;
    }
    if paths.is_empty() {
        paths.push(PathBuf::from(
            "docs/fixtures/phase-25.4/manifest-lock-consistency",
        ));
    }
    Ok(ManifestLockDriftOpts { paths })
}

fn check_manifest_lock_consistency_for_dir(dir: &Path) -> Result<(), String> {
    let manifest_path = dir.join("clg.project.json");
    let lock_path = dir.join("clg.lock.json");
    let graph_path = dir.join("clg.resolved-graph.json");
    let graph_hash_path = dir.join("clg.resolved-graph.sha256");
    if !manifest_path.is_file() {
        return Err(format!("missing manifest `{}`", manifest_path.display()));
    }
    if !lock_path.is_file() {
        return Err(format!("missing lockfile `{}`", lock_path.display()));
    }
    if !graph_path.is_file() {
        return Err(format!("missing resolved graph `{}`", graph_path.display()));
    }
    if !graph_hash_path.is_file() {
        return Err(format!(
            "missing resolved graph hash `{}`",
            graph_hash_path.display()
        ));
    }

    let manifest_raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("read manifest `{}`: {e}", manifest_path.display()))?;
    let lock_raw_bytes = fs::read(&lock_path)
        .map_err(|e| format!("read lockfile `{}`: {e}", lock_path.display()))?;
    let lock_raw = std::str::from_utf8(&lock_raw_bytes)
        .map_err(|e| format!("lockfile `{}` is not valid utf-8: {e}", lock_path.display()))?;
    let graph_raw_bytes = fs::read(&graph_path)
        .map_err(|e| format!("read resolved graph `{}`: {e}", graph_path.display()))?;
    let graph_raw = std::str::from_utf8(&graph_raw_bytes).map_err(|e| {
        format!(
            "resolved graph `{}` is not valid utf-8: {e}",
            graph_path.display()
        )
    })?;
    let graph_hash_raw = fs::read_to_string(&graph_hash_path).map_err(|e| {
        format!(
            "read resolved graph hash `{}`: {e}",
            graph_hash_path.display()
        )
    })?;

    let manifest: DriftManifestFile = serde_json::from_str(&manifest_raw)
        .map_err(|e| format!("parse manifest `{}`: {e}", manifest_path.display()))?;
    let lock: DriftLockFile = serde_json::from_str(lock_raw)
        .map_err(|e| format!("parse lockfile `{}`: {e}", lock_path.display()))?;
    let graph: DriftLockFile = serde_json::from_str(graph_raw)
        .map_err(|e| format!("parse resolved graph `{}`: {e}", graph_path.display()))?;

    if manifest.schema_version != 1 {
        return Err(format!(
            "manifest `{}` must use schema_version 1 for drift checks",
            manifest_path.display()
        ));
    }
    if lock.schema_version != 1 || lock.resolver_version != 1 {
        return Err(format!(
            "lockfile `{}` must use schema_version=1 and resolver_version=1",
            lock_path.display()
        ));
    }
    if graph.schema_version != 1 || graph.resolver_version != 1 {
        return Err(format!(
            "resolved graph `{}` must use schema_version=1 and resolver_version=1",
            graph_path.display()
        ));
    }

    let expected_dependencies = normalize_manifest_dependencies(&manifest.dependencies)?;
    let expected_roots = vec![DriftLockRoot {
        name: manifest.project.name.trim().to_string(),
        dependencies: expected_dependencies,
    }];
    let actual_roots = normalize_lock_roots(lock.roots)?;
    let lock_packages = normalize_lock_packages(lock.packages)?;
    let canonical_lock = DriftLockFile {
        schema_version: 1,
        resolver_version: 1,
        roots: actual_roots.clone(),
        packages: lock_packages.clone(),
    };
    let canonical_lock_bytes = pretty_json_bytes(&canonical_lock)?;
    if lock_raw_bytes != canonical_lock_bytes {
        return Err(format!(
            "lockfile `{}` must match canonical tool-owned format; regenerate with `clg pkg lock --update --root {}`",
            lock_path.display(),
            dir.display()
        ));
    }

    if actual_roots != expected_roots {
        let expected_json = serde_json::to_string(&expected_roots)
            .map_err(|e| format!("serialize expected roots: {e}"))?;
        let actual_json = serde_json::to_string(&actual_roots)
            .map_err(|e| format!("serialize lock roots: {e}"))?;
        return Err(format!(
            "manifest/lock inconsistency (`{}` vs `{}`): expected roots {}, found {}",
            manifest_path.display(),
            lock_path.display(),
            expected_json,
            actual_json
        ));
    }

    let graph_roots = normalize_lock_roots(graph.roots)?;
    let graph_packages = normalize_lock_packages(graph.packages)?;
    if graph_roots != actual_roots || graph_packages != lock_packages {
        return Err(format!(
            "lock/resolved-graph inconsistency (`{}` vs `{}`): roots/packages must match exactly",
            lock_path.display(),
            graph_path.display()
        ));
    }
    let graph_canonical_bytes = if graph_raw_bytes.last() == Some(&b'\n') {
        &graph_raw_bytes[..graph_raw_bytes.len() - 1]
    } else {
        graph_raw_bytes.as_slice()
    };
    let expected_graph_hash = hex::encode(Sha256::digest(graph_canonical_bytes));
    let actual_graph_hash = graph_hash_raw.trim();
    if actual_graph_hash != expected_graph_hash {
        return Err(format!(
            "resolved graph hash mismatch (`{}`): expected {}, found {}",
            graph_hash_path.display(),
            expected_graph_hash,
            actual_graph_hash
        ));
    }

    Ok(())
}

fn normalize_manifest_dependencies(
    dependencies: &[DriftManifestDependency],
) -> Result<Vec<DriftLockRootDependency>, String> {
    let mut out = Vec::with_capacity(dependencies.len());
    let mut seen = BTreeSet::new();
    for dep in dependencies {
        let name = dep.name.trim();
        let requirement = dep.requirement.trim();
        if name.is_empty() {
            return Err("manifest dependency name must be non-empty".to_string());
        }
        if requirement.is_empty() {
            return Err(format!(
                "manifest dependency `{name}` requirement must be non-empty"
            ));
        }
        if !seen.insert(name.to_string()) {
            return Err(format!("manifest has duplicate dependency `{name}`"));
        }
        out.push(DriftLockRootDependency {
            name: name.to_string(),
            requirement: requirement.to_string(),
        });
    }
    out.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.requirement.cmp(&b.requirement))
    });
    Ok(out)
}

fn normalize_lock_roots(roots: Vec<DriftLockRoot>) -> Result<Vec<DriftLockRoot>, String> {
    if roots.len() != 1 {
        return Err(format!(
            "lockfile roots must contain exactly one root for manifest v1 (found {})",
            roots.len()
        ));
    }
    let mut out = Vec::with_capacity(roots.len());
    let mut seen_root_names = BTreeSet::new();
    for mut root in roots {
        let root_name = root.name.trim();
        if root_name.is_empty() {
            return Err("lockfile root name must be non-empty".to_string());
        }
        if !seen_root_names.insert(root_name.to_string()) {
            return Err(format!("lockfile has duplicate root `{root_name}`"));
        }
        root.name = root_name.to_string();

        let mut seen_dep_names = BTreeSet::new();
        for dep in &mut root.dependencies {
            let name = dep.name.trim();
            let requirement = dep.requirement.trim();
            if name.is_empty() {
                return Err(format!(
                    "lockfile root `{}` has dependency with empty name",
                    root.name
                ));
            }
            if requirement.is_empty() {
                return Err(format!(
                    "lockfile root `{}` dependency `{}` has empty requirement",
                    root.name, name
                ));
            }
            if !seen_dep_names.insert(name.to_string()) {
                return Err(format!(
                    "lockfile root `{}` has duplicate dependency `{}`",
                    root.name, name
                ));
            }
            dep.name = name.to_string();
            dep.requirement = requirement.to_string();
        }
        root.dependencies.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.requirement.cmp(&b.requirement))
        });
        out.push(root);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn normalize_lock_packages(
    packages: Vec<DriftLockPackage>,
) -> Result<Vec<DriftLockPackage>, String> {
    let mut out = Vec::with_capacity(packages.len());
    let mut seen_ids = BTreeSet::new();
    for mut pkg in packages {
        let id = pkg.id.trim().to_string();
        let name = pkg.name.trim().to_string();
        let version = pkg.version.trim().to_string();
        let digest = pkg.digest.trim().to_string();
        let abi_id = pkg.abi_id.trim().to_string();
        if id.is_empty()
            || name.is_empty()
            || version.is_empty()
            || digest.is_empty()
            || abi_id.is_empty()
        {
            return Err("lockfile package identity fields must be non-empty".to_string());
        }
        if id != format!("{name}@{version}") {
            return Err(format!(
                "lockfile package id `{}` must match `{}@{}`",
                id, name, version
            ));
        }
        if !seen_ids.insert(id.clone()) {
            return Err(format!("lockfile has duplicate package id `{id}`"));
        }

        let mut seen_deps = BTreeSet::new();
        let mut normalized_deps = Vec::with_capacity(pkg.dependencies.len());
        for dep in pkg.dependencies {
            let dep_id = dep.trim().to_string();
            if dep_id.is_empty() {
                return Err(format!("lockfile package `{}` has empty dependency id", id));
            }
            if !seen_deps.insert(dep_id.clone()) {
                return Err(format!(
                    "lockfile package `{}` has duplicate dependency id `{}`",
                    id, dep_id
                ));
            }
            normalized_deps.push(dep_id);
        }
        normalized_deps.sort();

        pkg.id = id;
        pkg.name = name;
        pkg.version = version;
        pkg.digest = digest;
        pkg.abi_id = abi_id;
        pkg.dependencies = normalized_deps;
        out.push(pkg);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn run_clg_test_schema_gate(root: &Path) -> Result<(), String> {
    let project_root = root.join("examples").join("projects").join("testing");
    let output = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("clg-cli")
        .arg("--")
        .arg("test")
        .arg(&project_root)
        .arg("--report")
        .arg("json")
        .current_dir(root)
        .output()
        .map_err(|e| format!("failed to run `clg test` schema gate: {e}"))?;

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("`clg test` schema gate stdout is not utf-8: {e}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "`clg test` schema gate failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            stdout,
            stderr
        ));
    }

    validate_clg_test_report_schema(stdout.trim())
}

fn validate_clg_test_report_schema(raw: &str) -> Result<(), String> {
    if raw.is_empty() {
        return Err("`clg test` schema gate emitted empty stdout".to_string());
    }
    let payload: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("parsing `clg test` report json: {e}"))?;
    let obj = payload
        .as_object()
        .ok_or_else(|| "`clg test` report must be a JSON object".to_string())?;

    let schema_version = obj
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "`clg test` report missing numeric `schema_version`".to_string())?;
    if schema_version != 1 {
        return Err(format!(
            "`clg test` report schema_version must be 1, found {schema_version}"
        ));
    }

    let report = obj
        .get("report")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "`clg test` report missing string `report`".to_string())?;
    if report != "json" {
        return Err(format!(
            "`clg test` report kind must be `json`, found `{report}`"
        ));
    }

    let status = obj
        .get("status")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "`clg test` report missing string `status`".to_string())?;
    if status != "ok" {
        return Err(format!(
            "`clg test` schema gate expects `status=ok` for examples/projects/testing, found `{status}`"
        ));
    }

    let discovered = obj
        .get("discovered")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "`clg test` report missing numeric `discovered`".to_string())?;
    let selected = obj
        .get("selected")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "`clg test` report missing numeric `selected`".to_string())?;
    let executed = obj
        .get("executed")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "`clg test` report missing numeric `executed`".to_string())?;
    let passed = obj
        .get("passed")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "`clg test` report missing numeric `passed`".to_string())?;
    let failed = obj
        .get("failed")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "`clg test` report missing numeric `failed`".to_string())?;

    if discovered == 0 {
        return Err("`clg test` schema gate requires at least one discovered test".to_string());
    }
    if selected == 0 || executed == 0 {
        return Err(
            "`clg test` schema gate requires selected/executed counts to be non-zero".to_string(),
        );
    }
    if selected != executed {
        return Err(format!(
            "`clg test` schema gate requires selected ({selected}) == executed ({executed})"
        ));
    }
    if failed != 0 {
        return Err(format!(
            "`clg test` schema gate requires zero failed tests, found {failed}"
        ));
    }
    if passed != executed {
        return Err(format!(
            "`clg test` schema gate requires passed ({passed}) == executed ({executed})"
        ));
    }

    let tests = obj
        .get("tests")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "`clg test` report missing array `tests`".to_string())?;
    if tests.len() as u64 != executed {
        return Err(format!(
            "`clg test` schema gate requires tests.len ({}) == executed ({executed})",
            tests.len()
        ));
    }

    let mut prev_id: Option<String> = None;
    let mut mocked_cases = 0usize;
    let mut non_mocked_cases = 0usize;
    for case in tests {
        let case_obj = case
            .as_object()
            .ok_or_else(|| "each `tests[]` entry must be an object".to_string())?;
        let id = case_obj
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "each `tests[]` entry must include string `id`".to_string())?;
        let _file = case_obj
            .get("file")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("test `{id}` missing string `file`"))?;
        let _function = case_obj
            .get("function")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("test `{id}` missing string `function`"))?;
        let _timeout_ms = case_obj
            .get("timeout_ms")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("test `{id}` missing numeric `timeout_ms`"))?;
        let mock_sets = case_obj
            .get("mock_sets")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("test `{id}` missing array `mock_sets`"))?;
        if mock_sets.is_empty() {
            non_mocked_cases += 1;
        } else {
            mocked_cases += 1;
        }
        let status = case_obj
            .get("status")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("test `{id}` missing string `status`"))?;
        if status != "passed" {
            return Err(format!(
                "`clg test` schema gate expected test `{id}` status `passed`, found `{status}`"
            ));
        }
        let _captured_stdout = case_obj
            .get("captured_stdout")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("test `{id}` missing string `captured_stdout`"))?;
        let _captured_stderr = case_obj
            .get("captured_stderr")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("test `{id}` missing string `captured_stderr`"))?;
        let replay_argv = case_obj
            .get("replay")
            .and_then(serde_json::Value::as_object)
            .and_then(|replay| replay.get("argv"))
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("test `{id}` missing replay.argv[]"))?;
        if replay_argv.len() < 3 {
            return Err(format!(
                "test `{id}` replay.argv[] must include at least command tokens"
            ));
        }
        if replay_argv.first().and_then(serde_json::Value::as_str) != Some("clg")
            || replay_argv.get(1).and_then(serde_json::Value::as_str) != Some("test")
        {
            return Err(format!(
                "test `{id}` replay.argv[] must start with [`clg`, `test`]"
            ));
        }

        if let Some(prev) = prev_id.as_ref() {
            if prev.as_str() > id {
                return Err(format!(
                    "`clg test` report test ids must be sorted deterministically (`{prev}` > `{id}`)"
                ));
            }
        }
        prev_id = Some(id.to_string());
    }
    if mocked_cases == 0 || non_mocked_cases == 0 {
        return Err(format!(
            "`clg test` schema gate requires balanced critical-path coverage with mocked and non-mocked cases (mocked={mocked_cases}, non_mocked={non_mocked_cases})"
        ));
    }

    Ok(())
}
