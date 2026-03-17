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

fn check_std_surface_drift(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_std_surface_args(raw_args)?;
    let design_lock_path = root
        .join("docs")
        .join("design")
        .join("phase-21.0-std-core-package-surface.md");
    let std_metadata_path = root
        .join("crates")
        .join("cli")
        .join("assets")
        .join("std-metadata.json");
    let typer_builtins_path = root
        .join("crates")
        .join("typer")
        .join("src")
        .join("builtins.rs");
    let typer_collections_path = root
        .join("crates")
        .join("typer")
        .join("src")
        .join("check")
        .join("expr")
        .join("calls")
        .join("collections.rs");
    let codegen_ir_path = root
        .join("crates")
        .join("codegen-wasm")
        .join("src")
        .join("ir")
        .join("mod.rs");
    let binding_lock_path = root
        .join("docs")
        .join("design")
        .join("phase-21.0-std-binding-map.lock.json");

    let design_lock = fs::read_to_string(&design_lock_path)
        .map_err(|e| format!("read `{}`: {e}", design_lock_path.display()))?;
    let locked_symbols = parse_phase21_locked_symbols_from_design(&design_lock)?;
    let metadata_symbols = load_std_metadata_value_symbols(&std_metadata_path)?;

    let typer_builtins_source = fs::read_to_string(&typer_builtins_path)
        .map_err(|e| format!("read `{}`: {e}", typer_builtins_path.display()))?;
    let typer_collections_source = fs::read_to_string(&typer_collections_path)
        .map_err(|e| format!("read `{}`: {e}", typer_collections_path.display()))?;
    let typer_builtin_symbols = extract_std_symbols_from_source(&typer_builtins_source)?;
    let typer_collection_symbols = extract_std_symbols_from_source(&typer_collections_source)?;
    let typer_symbols = typer_builtin_symbols
        .union(&typer_collection_symbols)
        .filter(|symbol| is_callable_std_symbol(symbol))
        .cloned()
        .collect::<BTreeSet<_>>();

    let codegen_ir_source = fs::read_to_string(&codegen_ir_path)
        .map_err(|e| format!("read `{}`: {e}", codegen_ir_path.display()))?;
    let codegen_symbols_all = extract_std_symbols_from_source(&codegen_ir_source)?;
    let codegen_symbols = codegen_symbols_all
        .intersection(&locked_symbols)
        .cloned()
        .collect::<BTreeSet<_>>();

    let generated_binding_map = generate_std_binding_map(&locked_symbols, &codegen_symbols);

    if let Some(out_dir) = opts.emit_artifact.as_ref() {
        emit_std_binding_map_artifact(out_dir.as_path(), &generated_binding_map)?;
    }

    if opts.refresh_lock {
        let bytes = pretty_json_bytes(&generated_binding_map)?;
        fs::write(&binding_lock_path, bytes)
            .map_err(|e| format!("write `{}`: {e}", binding_lock_path.display()))?;
    }

    let mut drift_errors = Vec::new();
    collect_set_mismatch(
        "phase-21 design lock vs std metadata value surface",
        &locked_symbols,
        &metadata_symbols,
        &mut drift_errors,
    );
    collect_set_mismatch(
        "phase-21 design lock vs typer std call-check surface",
        &locked_symbols,
        &typer_symbols,
        &mut drift_errors,
    );
    collect_extras(
        "codegen intrinsic symbol surface",
        &locked_symbols,
        &codegen_symbols_all,
        &mut drift_errors,
    );

    let expected_binding_lock = fs::read_to_string(&binding_lock_path)
        .map_err(|e| format!("read `{}`: {e}", binding_lock_path.display()))?;
    let parsed_lock: StdBindingMapLockFile = serde_json::from_str(&expected_binding_lock)
        .map_err(|e| format!("parse `{}`: {e}", binding_lock_path.display()))?;
    let normalized_lock = normalize_binding_map_lock(parsed_lock)?;
    let lock_symbols_from_binding = normalized_lock
        .symbols
        .iter()
        .map(|entry| entry.symbol.clone())
        .collect::<BTreeSet<_>>();
    collect_set_mismatch(
        "phase-21 design lock vs std binding-map lock symbols",
        &locked_symbols,
        &lock_symbols_from_binding,
        &mut drift_errors,
    );

    let expected_routes = binding_routes_as_map(&normalized_lock);
    let generated_routes = binding_routes_as_map(&generated_binding_map);
    collect_route_mismatch(&expected_routes, &generated_routes, &mut drift_errors);

    if drift_errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "std surface drift detected:\n - {}",
            drift_errors.join("\n - ")
        ))
    }
}

fn parse_std_surface_args(raw_args: Vec<String>) -> Result<StdSurfaceDriftOpts, String> {
    let mut emit_artifact: Option<PathBuf> = None;
    let mut refresh_lock = false;
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--emit-artifact" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--emit-artifact`".to_string())?;
                emit_artifact = Some(PathBuf::from(value));
            }
            "--refresh-lock" => {
                refresh_lock = true;
            }
            other => {
                return Err(format!(
                    "unknown std-surface-drift-check arg `{other}` (supported: --emit-artifact, --refresh-lock)"
                ));
            }
        }
        idx += 1;
    }
    Ok(StdSurfaceDriftOpts {
        emit_artifact,
        refresh_lock,
    })
}

fn parse_phase21_locked_symbols_from_design(markdown: &str) -> Result<BTreeSet<String>, String> {
    let pattern = Regex::new(r"std::([a-z0-9_]+)::\{([^}]*)\}")
        .map_err(|e| format!("compile phase-21 lock regex: {e}"))?;
    let mut symbols = BTreeSet::new();
    for capture in pattern.captures_iter(markdown) {
        let module = capture
            .get(1)
            .map(|m| m.as_str())
            .ok_or_else(|| "phase-21 lock parse missing module capture".to_string())?;
        let entries = capture
            .get(2)
            .map(|m| m.as_str())
            .ok_or_else(|| "phase-21 lock parse missing export capture".to_string())?;
        for item in entries.split(',') {
            let name = item.trim();
            if name.is_empty() {
                continue;
            }
            symbols.insert(format!("std::{module}::{name}"));
        }
    }
    if symbols.is_empty() {
        Err("phase-21 design lock yielded zero std symbols".to_string())
    } else {
        Ok(symbols)
    }
}

fn load_std_metadata_value_symbols(path: &Path) -> Result<BTreeSet<String>, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read `{}`: {e}", path.display()))?;
    let parsed: StdMetadataRoot =
        serde_json::from_str(&raw).map_err(|e| format!("parse `{}`: {e}", path.display()))?;
    let mut out = BTreeSet::new();
    for module in parsed.modules {
        for export in module.exports {
            if export.kind == "value" {
                out.insert(format!("{}::{}", module.path, export.name));
            }
        }
    }
    Ok(out)
}

fn extract_std_symbols_from_source(source: &str) -> Result<BTreeSet<String>, String> {
    let pattern = Regex::new(r#""(std::[a-z0-9_]+::[A-Za-z0-9_]+)""#)
        .map_err(|e| format!("compile std symbol regex: {e}"))?;
    let mut out = BTreeSet::new();
    for capture in pattern.captures_iter(source) {
        if let Some(symbol) = capture.get(1) {
            out.insert(symbol.as_str().to_string());
        }
    }
    Ok(out)
}

fn is_callable_std_symbol(symbol: &str) -> bool {
    let Some((_, leaf)) = symbol.rsplit_once("::") else {
        return false;
    };
    leaf.chars()
        .next()
        .map(|c| c.is_ascii_lowercase())
        .unwrap_or(false)
}

fn generate_std_binding_map(
    locked_symbols: &BTreeSet<String>,
    codegen_intrinsic_symbols: &BTreeSet<String>,
) -> StdBindingMapLockFile {
    let symbols = locked_symbols
        .iter()
        .map(|symbol| StdBindingRouteEntry {
            symbol: symbol.clone(),
            route: if codegen_intrinsic_symbols.contains(symbol) {
                "intrinsic".to_string()
            } else {
                "package_import".to_string()
            },
        })
        .collect();
    StdBindingMapLockFile {
        schema_version: 1,
        symbols,
    }
}

fn normalize_binding_map_lock(
    lock: StdBindingMapLockFile,
) -> Result<StdBindingMapLockFile, String> {
    if lock.schema_version != 1 {
        return Err(format!(
            "unsupported std binding-map lock schema_version {} (expected 1)",
            lock.schema_version
        ));
    }
    let mut normalized = BTreeMap::<String, String>::new();
    for entry in lock.symbols {
        if entry.route != "intrinsic" && entry.route != "package_import" {
            return Err(format!(
                "invalid std binding-map route `{}` for symbol `{}`",
                entry.route, entry.symbol
            ));
        }
        if normalized
            .insert(entry.symbol.clone(), entry.route.clone())
            .is_some()
        {
            return Err(format!(
                "duplicate std binding-map lock symbol `{}`",
                entry.symbol
            ));
        }
    }
    Ok(StdBindingMapLockFile {
        schema_version: 1,
        symbols: normalized
            .into_iter()
            .map(|(symbol, route)| StdBindingRouteEntry { symbol, route })
            .collect(),
    })
}

fn collect_set_mismatch(
    label: &str,
    expected: &BTreeSet<String>,
    actual: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let missing = expected
        .difference(actual)
        .cloned()
        .collect::<Vec<String>>();
    let extra = actual
        .difference(expected)
        .cloned()
        .collect::<Vec<String>>();
    if !missing.is_empty() || !extra.is_empty() {
        errors.push(format!(
            "{label}: missing [{}], extra [{}]",
            preview_symbols(&missing),
            preview_symbols(&extra)
        ));
    }
}

fn collect_extras(
    label: &str,
    expected_superset: &BTreeSet<String>,
    actual: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let extra = actual
        .difference(expected_superset)
        .cloned()
        .collect::<Vec<String>>();
    if !extra.is_empty() {
        errors.push(format!("{label}: unexpected [{}]", preview_symbols(&extra)));
    }
}

fn collect_route_mismatch(
    expected: &BTreeMap<String, String>,
    actual: &BTreeMap<String, String>,
    errors: &mut Vec<String>,
) {
    let mut mismatches = Vec::new();
    for symbol in expected.keys() {
        if let (Some(exp), Some(found)) = (expected.get(symbol), actual.get(symbol)) {
            if exp != found {
                mismatches.push(format!("{symbol}: expected `{exp}`, found `{found}`"));
            }
        }
    }
    if !mismatches.is_empty() {
        errors.push(format!(
            "std binding-map route drift [{}]",
            preview_symbols(&mismatches)
        ));
    }
}

fn preview_symbols(items: &[String]) -> String {
    if items.is_empty() {
        return "none".to_string();
    }
    const MAX_ITEMS: usize = 8;
    let mut shown = items
        .iter()
        .take(MAX_ITEMS)
        .cloned()
        .collect::<Vec<String>>();
    if items.len() > MAX_ITEMS {
        shown.push(format!("... +{}", items.len() - MAX_ITEMS));
    }
    shown.join(", ")
}

fn binding_routes_as_map(lock: &StdBindingMapLockFile) -> BTreeMap<String, String> {
    lock.symbols
        .iter()
        .map(|entry| (entry.symbol.clone(), entry.route.clone()))
        .collect()
}

fn emit_std_binding_map_artifact(
    out_dir: &Path,
    binding_map: &StdBindingMapLockFile,
) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|e| format!("create `{}`: {e}", out_dir.display()))?;
    let json_path = out_dir.join("std-binding-map-v1.json");
    let sha_path = out_dir.join("std-binding-map-v1.sha256");
    let bytes = pretty_json_bytes(binding_map)?;
    fs::write(&json_path, &bytes).map_err(|e| format!("write `{}`: {e}", json_path.display()))?;
    let digest = format!("sha256:{}", hex::encode(Sha256::digest(&bytes)));
    fs::write(&sha_path, format!("{digest}\n"))
        .map_err(|e| format!("write `{}`: {e}", sha_path.display()))?;
    Ok(())
}

fn pretty_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| format!("serialize json: {e}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn minimal_wasm_module_bytes() -> Vec<u8> {
    vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]
}

fn select_core_modules(mut modules: Vec<StdMetadataModule>) -> Vec<StdMetadataModule> {
    const CORE_MODULES: [&str; 9] = [
        "std::str",
        "std::bytes",
        "std::u64",
        "std::u128",
        "std::u256",
        "std::array",
        "std::slice",
        "std::list",
        "std::set",
        // `std::map` included through filter below.
    ];
    modules
        .retain(|module| module.path == "std::map" || CORE_MODULES.contains(&module.path.as_str()));
    modules.sort_by(|a, b| a.path.cmp(&b.path));
    for module in &mut modules {
        module
            .exports
            .sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.kind.cmp(&b.kind)));
    }
    modules
}

fn core_modules_to_abi_imports(modules: &[StdMetadataModule]) -> Vec<StrictAbiImport> {
    let mut imports = Vec::new();
    for module in modules {
        for export in &module.exports {
            if export.kind != "value" {
                continue;
            }
            imports.push(StrictAbiImport {
                symbol: format!("{}::{}", module.path, export.name),
                effect: if export.name.ends_with("_mut") {
                    "mut".to_string()
                } else {
                    "pure".to_string()
                },
                params: Vec::new(),
                ret: "Unknown".to_string(),
                capability: None,
            });
        }
    }
    imports.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    imports
}

fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| format!("serialize `{}`: {e}", path.display()))?;
    fs::write(path, bytes).map_err(|e| format!("write `{}`: {e}", path.display()))
}

fn run(cmd: &mut Command) -> Result<(), String> {
    let program = cmd.get_program().to_os_string();
    let args = cmd
        .get_args()
        .map(|arg| arg.to_os_string())
        .collect::<Vec<_>>();
    let display = display_cmd(&program, &args);
    let status = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("failed to run {display}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "command {display} failed with status {:?}",
            status.code()
        ))
    }
}

fn display_cmd(program: &OsStr, args: &[std::ffi::OsString]) -> String {
    let mut line = program.to_string_lossy().to_string();
    for arg in args {
        line.push(' ');
        line.push_str(&arg.to_string_lossy());
    }
    line
}

fn print_help() {
    println!("xtask commands:");
    println!("  fmt        - cargo fmt --all");
    println!("  clippy     - cargo clippy --workspace --all-targets -- -D warnings");
    println!("  test       - cargo test --workspace");
    println!("  validate   - build samples and run wasm-tools validate");
    println!("  emit-vcs   - build a small contract sample with --emit-vcs");
    println!("  std-core-artifact [--version X.Y.Z] [--out-dir DIR]");
    println!(
        "  std-surface-drift-check [--emit-artifact DIR] [--refresh-lock] (phase 21 drift gate)"
    );
    println!("  ci         - fmt + clippy + test + validate");
}

#[derive(Clone, Debug)]
struct StdCoreArtifactOpts {
    version: String,
    out_dir: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct StdSurfaceDriftOpts {
    emit_artifact: Option<PathBuf>,
    refresh_lock: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StdMetadataRoot {
    schema_version: u32,
    modules: Vec<StdMetadataModule>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StdMetadataModule {
    path: String,
    exports: Vec<StdMetadataExport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StdMetadataExport {
    name: String,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    layout: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize)]
struct StdCoreSurfaceMetadata {
    schema_version: u32,
    package: String,
    version: String,
    source_std_metadata_schema_version: u32,
    modules: Vec<StdMetadataModule>,
}

#[derive(Clone, Debug, Serialize)]
struct StrictPackageMetadataFile {
    schema_version: u32,
    packages: Vec<StrictPackageMetadataEntry>,
}

#[derive(Clone, Debug, Serialize)]
struct StrictPackageMetadataEntry {
    name: String,
    version: String,
    digest: String,
    artifact: StrictPackageArtifact,
    abi_id: String,
}

#[derive(Clone, Debug, Serialize)]
struct StrictPackageArtifact {
    format: String,
    path: String,
}

#[derive(Clone, Debug, Serialize)]
struct StrictAbiFile {
    schema_version: u32,
    contracts: Vec<StrictAbiContract>,
}

#[derive(Clone, Debug, Serialize)]
struct StrictAbiContract {
    abi_id: String,
    package: String,
    version: String,
    imports: Vec<StrictAbiImport>,
}

#[derive(Clone, Debug, Serialize)]
struct StrictAbiImport {
    symbol: String,
    effect: String,
    params: Vec<String>,
    ret: String,
    capability: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct StdCoreArtifactManifest {
    schema_version: u32,
    package: String,
    version: String,
    artifact: ArtifactEntry,
    metadata: MetadataEntry,
}

#[derive(Clone, Debug, Serialize)]
struct ArtifactEntry {
    file: String,
    digest: String,
}

#[derive(Clone, Debug, Serialize)]
struct MetadataEntry {
    surface: String,
    strict_package_metadata: String,
    strict_package_abi: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StdBindingMapLockFile {
    schema_version: u32,
    symbols: Vec<StdBindingRouteEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StdBindingRouteEntry {
    symbol: String,
    route: String,
}
