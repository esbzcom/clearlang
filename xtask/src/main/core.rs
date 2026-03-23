use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
        "validate" => validate_samples(&root)?,
        "emit-vcs" => emit_vcs_sample(&root)?,
        "std-core-artifact" => emit_std_core_artifact(&root, args.collect())?,
        "std-surface-drift-check" => check_std_surface_drift(&root, args.collect())?,
        "host-capability-policy-artifact" => {
            emit_host_capability_policy_artifact(&root, args.collect())?
        }
        "ci" => {
            cargo_cmd(&root, &["fmt", "--all", "--", "--check"])?;
            cargo_cmd(
                &root,
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
            )?;
            cargo_cmd(&root, &["test", "--workspace"])?;
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

