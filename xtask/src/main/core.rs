use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ed25519_dalek::{Signer, SigningKey};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SOLVER_VENDOR_SIGNING_KEY_ENV: &str = "CLG_SOLVER_VENDOR_SIGNING_KEY_HEX";
const DEFAULT_SOLVER_VENDOR_KEY_ID: &str = "z3-vendor-k7-2026q2";

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
        "host-capability-policy-artifact" => {
            emit_host_capability_policy_artifact(&root, args.collect())?
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
    if !manifest_path.is_file() {
        return Err(format!(
            "missing manifest `{}`",
            manifest_path.display()
        ));
    }
    if !lock_path.is_file() {
        return Err(format!("missing lockfile `{}`", lock_path.display()));
    }

    let manifest_raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("read manifest `{}`: {e}", manifest_path.display()))?;
    let lock_raw = fs::read_to_string(&lock_path)
        .map_err(|e| format!("read lockfile `{}`: {e}", lock_path.display()))?;

    let manifest: DriftManifestFile = serde_json::from_str(&manifest_raw)
        .map_err(|e| format!("parse manifest `{}`: {e}", manifest_path.display()))?;
    let lock: DriftLockFile = serde_json::from_str(&lock_raw)
        .map_err(|e| format!("parse lockfile `{}`: {e}", lock_path.display()))?;

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

    let expected_dependencies = normalize_manifest_dependencies(&manifest.dependencies)?;
    let expected_roots = vec![DriftLockRoot {
        name: manifest.project.name.trim().to_string(),
        dependencies: expected_dependencies,
    }];
    let actual_roots = normalize_lock_roots(lock.roots)?;

    if actual_roots != expected_roots {
        let expected_json =
            serde_json::to_string(&expected_roots).map_err(|e| format!("serialize expected roots: {e}"))?;
        let actual_json =
            serde_json::to_string(&actual_roots).map_err(|e| format!("serialize lock roots: {e}"))?;
        return Err(format!(
            "manifest/lock inconsistency (`{}` vs `{}`): expected roots {}, found {}",
            manifest_path.display(),
            lock_path.display(),
            expected_json,
            actual_json
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
    let mut signature_bytes =
        serde_json::to_vec_pretty(&signature_payload).map_err(|e| format!("serialize signature sidecar: {e}"))?;
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
    let mut name = solver_bin.file_name().unwrap_or_else(|| OsStr::new("solver")).to_os_string();
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

fn decode_fixed_hex32(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 || value.bytes().any(|b| !b.is_ascii_hexdigit() || b.is_ascii_uppercase())
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

