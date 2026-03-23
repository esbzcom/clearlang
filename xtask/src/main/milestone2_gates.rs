use serde_json::{json, Value};
use std::cmp::Ordering;
use std::time::Instant;

const STARTUP_LATENCY_MAX_S_DEFAULT: f64 = 2.0;
const PKG_RESOLUTION_LATENCY_MAX_S_DEFAULT: f64 = 2.5;
const RUNTIME_LINK_STARTUP_LATENCY_MAX_S_DEFAULT: f64 = 3.0;
const MAX_RSS_KB_DEFAULT: u64 = 400_000;
const MAX_CPU_PERCENT_DEFAULT: f64 = 400.0;
const PERF_SAMPLE_RUNS_DEFAULT: usize = 3;

#[derive(Clone, Copy)]
enum PerfGateMode {
    Run,
    PortabilitySmoke,
    SelfTest,
}

#[derive(Clone, Copy)]
struct PerfGateConfig {
    startup_latency_max_s: f64,
    pkg_resolution_latency_max_s: f64,
    runtime_link_startup_latency_max_s: f64,
    max_rss_kb: u64,
    max_cpu_percent: f64,
    sample_runs: usize,
}

#[derive(Clone, Copy, Debug)]
struct PerfSample {
    latency_s: f64,
    rss_kb: u64,
    cpu_percent: f64,
}

#[derive(Clone, Debug)]
struct SupplyChainReport {
    report: Value,
    violations: Vec<Value>,
}

fn run_milestone2_perf_gate(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let mode = parse_perf_gate_mode(raw_args)?;
    match mode {
        PerfGateMode::PortabilitySmoke => {
            run_perf_portability_smoke()?;
            println!("milestone_2 perf portability smoke passed");
            return Ok(());
        }
        PerfGateMode::SelfTest => {
            run_perf_self_test(root)?;
            println!("milestone_2 perf self-test passed");
            return Ok(());
        }
        PerfGateMode::Run => {}
    }

    let cfg = perf_gate_config_from_env()?;
    let out_dir = root.join("tmp").join("perf");
    fs::create_dir_all(&out_dir)
        .map_err(|e| format!("create perf output dir `{}`: {e}", out_dir.display()))?;

    let clg_bin = find_clg_release_bin(root)?;
    let startup = measure_median_sample(cfg.sample_runs, || {
        let mut cmd = Command::new(&clg_bin);
        cmd.current_dir(root)
            .arg("run")
            .arg(root.join("clearlang-tests").join("16_namespaced_call.clear"));
        Ok(cmd)
    })?;
    let pkg_resolution = measure_median_sample(cfg.sample_runs, || {
        let mut cmd = Command::new(&clg_bin);
        cmd.current_dir(root)
            .arg("build")
            .arg(
                root.join("clearlang-tests")
                    .join("perf")
                    .join("pkg_resolution")
                    .join("main.clear"),
            )
            .arg("-o")
            .arg(out_dir.join("pkg_resolution.wasm"));
        Ok(cmd)
    })?;

    let runtime_fixture = setup_runtime_loader_fixture(root, &out_dir)?;
    let runtime_link_startup = measure_median_sample(cfg.sample_runs, || {
        let mut cmd = Command::new(&clg_bin);
        cmd.current_dir(root)
            .arg("run")
            .arg(runtime_fixture.join("app.wasm"));
        Ok(cmd)
    })?;

    assert_le_float(
        "startup_latency_s",
        startup.latency_s,
        cfg.startup_latency_max_s,
    )?;
    assert_le_float(
        "pkg_resolution_latency_s",
        pkg_resolution.latency_s,
        cfg.pkg_resolution_latency_max_s,
    )?;
    assert_le_float(
        "runtime_link_startup_latency_s",
        runtime_link_startup.latency_s,
        cfg.runtime_link_startup_latency_max_s,
    )?;
    assert_le_int("startup_rss_kb", startup.rss_kb, cfg.max_rss_kb)?;
    assert_le_int("pkg_resolution_rss_kb", pkg_resolution.rss_kb, cfg.max_rss_kb)?;
    assert_le_int(
        "runtime_link_rss_kb",
        runtime_link_startup.rss_kb,
        cfg.max_rss_kb,
    )?;
    assert_le_float(
        "startup_cpu_percent",
        startup.cpu_percent,
        cfg.max_cpu_percent,
    )?;
    assert_le_float(
        "pkg_resolution_cpu_percent",
        pkg_resolution.cpu_percent,
        cfg.max_cpu_percent,
    )?;
    assert_le_float(
        "runtime_link_cpu_percent",
        runtime_link_startup.cpu_percent,
        cfg.max_cpu_percent,
    )?;

    let artifact = json!({
      "schema_version": 1,
      "sample_runs": cfg.sample_runs,
      "aggregation_mode": "median",
      "thresholds": {
        "startup_latency_s_max": cfg.startup_latency_max_s,
        "pkg_resolution_latency_s_max": cfg.pkg_resolution_latency_max_s,
        "runtime_link_startup_latency_s_max": cfg.runtime_link_startup_latency_max_s,
        "max_rss_kb": cfg.max_rss_kb,
        "max_cpu_percent": cfg.max_cpu_percent
      },
      "measurements": {
        "startup": sample_to_json(startup),
        "pkg_resolution": sample_to_json(pkg_resolution),
        "runtime_link_startup": sample_to_json(runtime_link_startup)
      }
    });

    let perf_artifact_path = out_dir.join("milestone2-performance.json");
    write_json_pretty(&perf_artifact_path, &artifact)?;
    println!("milestone_2 performance gate passed");
    println!("artifact: {}", perf_artifact_path.display());
    Ok(())
}

fn parse_perf_gate_mode(raw_args: Vec<String>) -> Result<PerfGateMode, String> {
    if raw_args.is_empty() {
        return Ok(PerfGateMode::Run);
    }
    if raw_args.len() > 1 {
        return Err(
            "usage: cargo run -p xtask -- milestone2-perf-gate [--portability-smoke|--self-test]"
                .into(),
        );
    }
    match raw_args[0].as_str() {
        "--portability-smoke" => Ok(PerfGateMode::PortabilitySmoke),
        "--self-test" => Ok(PerfGateMode::SelfTest),
        other => Err(format!("unknown milestone2-perf-gate arg `{other}`")),
    }
}

fn perf_gate_config_from_env() -> Result<PerfGateConfig, String> {
    Ok(PerfGateConfig {
        startup_latency_max_s: env_f64("STARTUP_LATENCY_MAX_S", STARTUP_LATENCY_MAX_S_DEFAULT)?,
        pkg_resolution_latency_max_s: env_f64(
            "PKG_RESOLUTION_LATENCY_MAX_S",
            PKG_RESOLUTION_LATENCY_MAX_S_DEFAULT,
        )?,
        runtime_link_startup_latency_max_s: env_f64(
            "RUNTIME_LINK_STARTUP_LATENCY_MAX_S",
            RUNTIME_LINK_STARTUP_LATENCY_MAX_S_DEFAULT,
        )?,
        max_rss_kb: env_u64("MAX_RSS_KB", MAX_RSS_KB_DEFAULT)?,
        max_cpu_percent: env_f64("MAX_CPU_PERCENT", MAX_CPU_PERCENT_DEFAULT)?,
        sample_runs: env_usize("PERF_SAMPLE_RUNS", PERF_SAMPLE_RUNS_DEFAULT)?,
    })
}

fn env_f64(name: &str, default: f64) -> Result<f64, String> {
    match env::var(name) {
        Ok(raw) => raw
            .parse::<f64>()
            .map_err(|e| format!("invalid `{name}` value `{raw}`: {e}")),
        Err(_) => Ok(default),
    }
}

fn env_u64(name: &str, default: u64) -> Result<u64, String> {
    match env::var(name) {
        Ok(raw) => raw
            .parse::<u64>()
            .map_err(|e| format!("invalid `{name}` value `{raw}`: {e}")),
        Err(_) => Ok(default),
    }
}

fn env_usize(name: &str, default: usize) -> Result<usize, String> {
    match env::var(name) {
        Ok(raw) => {
            let parsed = raw
                .parse::<usize>()
                .map_err(|e| format!("invalid `{name}` value `{raw}`: {e}"))?;
            if parsed == 0 {
                return Err(format!("`{name}` must be > 0"));
            }
            Ok(parsed)
        }
        Err(_) => Ok(default),
    }
}

fn sample_to_json(sample: PerfSample) -> Value {
    json!({
      "latency_s": sample.latency_s,
      "rss_kb": sample.rss_kb,
      "cpu_percent": sample.cpu_percent
    })
}

fn measure_median_sample<F>(runs: usize, mut build_cmd: F) -> Result<PerfSample, String>
where
    F: FnMut() -> Result<Command, String>,
{
    let mut latencies = Vec::with_capacity(runs);
    let mut rss_values = Vec::with_capacity(runs);
    let mut cpu_values = Vec::with_capacity(runs);
    for _ in 0..runs {
        let sample = measure_once(build_cmd()?)?;
        latencies.push(sample.latency_s);
        rss_values.push(sample.rss_kb);
        cpu_values.push(sample.cpu_percent);
    }
    Ok(PerfSample {
        latency_s: median_f64(&mut latencies),
        rss_kb: median_u64(&mut rss_values),
        cpu_percent: median_f64(&mut cpu_values),
    })
}

fn measure_once(mut cmd: Command) -> Result<PerfSample, String> {
    let display = format_command(&cmd);
    let start = Instant::now();
    let status = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("failed to run `{display}` for performance measurement: {e}"))?;
    let latency_s = start.elapsed().as_secs_f64();
    if !status.success() {
        return Err(format!(
            "performance measurement command `{display}` failed with status {:?}",
            status.code()
        ));
    }
    Ok(PerfSample {
        latency_s,
        rss_kb: 0,
        cpu_percent: 0.0,
    })
}

fn format_command(cmd: &Command) -> String {
    let mut out = cmd.get_program().to_string_lossy().to_string();
    for arg in cmd.get_args() {
        out.push(' ');
        out.push_str(&arg.to_string_lossy());
    }
    out
}

fn median_f64(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    values[values.len() / 2]
}

fn median_u64(values: &mut [u64]) -> u64 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn assert_le_float(name: &str, value: f64, max: f64) -> Result<(), String> {
    if value <= max {
        Ok(())
    } else {
        Err(format!(
            "{name} budget exceeded: value={value:.6}, max={max:.6}"
        ))
    }
}

fn assert_le_int(name: &str, value: u64, max: u64) -> Result<(), String> {
    if value <= max {
        Ok(())
    } else {
        Err(format!("{name} budget exceeded: value={value}, max={max}"))
    }
}

fn find_clg_release_bin(root: &Path) -> Result<PathBuf, String> {
    let base = root.join("target").join("release");
    let primary = base.join("clg");
    if primary.exists() {
        return Ok(primary);
    }
    let windows = base.join("clg.exe");
    if windows.exists() {
        return Ok(windows);
    }
    Err(format!(
        "missing release binary (`{}` or `{}`); run `cargo build --release -p clg-cli` first",
        primary.display(),
        windows.display()
    ))
}

fn find_tool_bin(name: &str) -> Result<String, String> {
    if command_exists(name) {
        return Ok(name.to_string());
    }
    let exe = format!("{name}.exe");
    if command_exists(&exe) {
        return Ok(exe);
    }
    Err(format!("missing `{name}` in PATH"))
}

fn command_exists(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn run_capture_output(cmd: &mut Command) -> Result<Vec<u8>, String> {
    let display = format_command(cmd);
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to run `{display}`: {e}"))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "command `{display}` failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn sha256_hex(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("read `{}`: {e}", path.display()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn setup_runtime_loader_fixture(root: &Path, perf_dir: &Path) -> Result<PathBuf, String> {
    let wasm_tools = find_tool_bin("wasm-tools")?;
    let openssl = find_tool_bin("openssl")?;
    let fixture_dir = perf_dir.join("runtime_loader");
    if fixture_dir.exists() {
        fs::remove_dir_all(&fixture_dir)
            .map_err(|e| format!("cleanup runtime fixture dir `{}`: {e}", fixture_dir.display()))?;
    }
    let store_dir = fixture_dir.join("store");
    fs::create_dir_all(&store_dir)
        .map_err(|e| format!("create runtime fixture store dir `{}`: {e}", store_dir.display()))?;

    let provider_wat = fixture_dir.join("provider.wat");
    let app_wat = fixture_dir.join("app.wat");
    let provider_wasm = store_dir.join("pkg-a.wasm");
    let app_wasm = fixture_dir.join("app.wasm");

    fs::write(
        &provider_wat,
        r#"(module
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
)"#,
    )
    .map_err(|e| format!("write `{}`: {e}", provider_wat.display()))?;

    fs::write(
        &app_wat,
        r#"(module
  (import "pkg::a" "add" (func $add (param i32 i32) (result i32)))
  (func (export "main") (result i32)
    i32.const 40
    i32.const 2
    call $add)
)"#,
    )
    .map_err(|e| format!("write `{}`: {e}", app_wat.display()))?;

    run(Command::new(&wasm_tools)
        .arg("parse")
        .arg(&provider_wat)
        .arg("-o")
        .arg(&provider_wasm)
        .current_dir(root))?;
    run(Command::new(&wasm_tools)
        .arg("parse")
        .arg(&app_wat)
        .arg("-o")
        .arg(&app_wasm)
        .current_dir(root))?;

    let digest = format!("sha256:{}", sha256_hex(&provider_wasm)?);
    let runtime_link = json!({
      "schema_version": 0,
      "resolver_version": 1,
      "packages": [{
        "id": "pkg::a@1.0.0",
        "digest": digest,
        "artifact_path": "store/pkg-a.wasm",
        "abi_id": "abi:pkg::a:1.0.0"
      }],
      "bindings": [{
        "import_module": "pkg::a",
        "import_name": "add",
        "provider_package_id": "pkg::a@1.0.0",
        "provider_symbol": "add"
      }]
    });
    write_json_pretty(&fixture_dir.join("clg.runtime-link.json"), &runtime_link)?;
    let runtime_link_canonical =
        serde_json::to_vec(&runtime_link).map_err(|e| format!("serialize runtime-link: {e}"))?;
    fs::write(
        fixture_dir.join("clg.runtime-link.sha256"),
        format!(
            "sha256:{}\n",
            hex::encode(Sha256::digest(runtime_link_canonical))
        ),
    )
    .map_err(|e| format!("write runtime-link hash: {e}"))?;

    let lockfile = json!({
      "schema_version": 1,
      "resolver_version": 1,
      "roots": [],
      "packages": [{
        "id": "pkg::a@1.0.0",
        "name": "pkg::a",
        "version": "1.0.0",
        "digest": digest,
        "abi_id": "abi:pkg::a:1.0.0",
        "dependencies": []
      }]
    });
    write_json_pretty(&fixture_dir.join("clg.lock.json"), &lockfile)?;

    let store_index = json!({
      "schema_version": 0,
      "artifacts": [{
        "id": "pkg::a@1.0.0",
        "digest": digest,
        "path": "store/pkg-a.wasm"
      }]
    });
    write_json_pretty(&fixture_dir.join("clg.package-store-index.json"), &store_index)?;

    let signing_key_path = fixture_dir.join("signing.key");
    run(Command::new(&openssl)
        .arg("genpkey")
        .arg("-algorithm")
        .arg("ED25519")
        .arg("-out")
        .arg(&signing_key_path)
        .current_dir(root))?;
    let pub_der = run_capture_output(
        Command::new(&openssl)
            .arg("pkey")
            .arg("-in")
            .arg(&signing_key_path)
            .arg("-pubout")
            .arg("-outform")
            .arg("DER")
            .current_dir(root),
    )?;
    if pub_der.len() < 32 {
        return Err("openssl public key DER output too short".into());
    }
    let pub_hex = hex::encode(&pub_der[pub_der.len() - 32..]);

    let signed_at = "2026-06-01T00:00:00Z";
    let payload = format!("clg-package-signature-v0\npkg::a\n1.0.0\n{digest}\n{signed_at}\n");
    let payload_path = fixture_dir.join("payload.txt");
    let sig_bin_path = fixture_dir.join("sig.bin");
    fs::write(&payload_path, payload).map_err(|e| format!("write payload: {e}"))?;
    run(Command::new(&openssl)
        .arg("pkeyutl")
        .arg("-sign")
        .arg("-inkey")
        .arg(&signing_key_path)
        .arg("-rawin")
        .arg("-in")
        .arg(&payload_path)
        .arg("-out")
        .arg(&sig_bin_path)
        .current_dir(root))?;
    let sig_hex = hex::encode(
        fs::read(&sig_bin_path)
            .map_err(|e| format!("read signature `{}`: {e}", sig_bin_path.display()))?,
    );

    let trust_policy = json!({
      "schema_version": 0,
      "trusted_signers": [{
        "key_id": "k1",
        "scheme": "ed25519",
        "public_key": format!("hex:{pub_hex}"),
        "not_before": "2026-01-01T00:00:00Z",
        "not_after": "2027-01-01T00:00:00Z"
      }],
      "revoked_key_ids": []
    });
    write_json_pretty(&fixture_dir.join("clg.trust-policy.json"), &trust_policy)?;

    let signatures = json!({
      "schema_version": 0,
      "signatures": [{
        "name": "pkg::a",
        "version": "1.0.0",
        "digest": digest,
        "key_id": "k1",
        "signed_at": signed_at,
        "signature_format": "ed25519",
        "signature": sig_hex
      }]
    });
    write_json_pretty(
        &fixture_dir.join("clg.package-signatures.json"),
        &signatures,
    )?;
    Ok(fixture_dir)
}

fn run_perf_portability_smoke() -> Result<(), String> {
    let sample = PathBuf::from("/tmp/clearlang/perf/sample.wasm");
    let converted_for_exe = to_tool_path("tool.exe", &sample);
    let converted_for_native = to_tool_path("tool", &sample);
    if cfg!(windows) {
        if converted_for_exe == converted_for_native {
            return Err(
                "portability smoke failed: expected different conversion for `.exe` tool on windows"
                    .into(),
            );
        }
    } else if converted_for_exe != converted_for_native {
        return Err("portability smoke failed: non-windows conversion should be passthrough".into());
    }
    Ok(())
}

fn to_tool_path(exe: &str, path: &Path) -> PathBuf {
    if cfg!(windows) && exe.ends_with(".exe") {
        if let Some(s) = path.to_str() {
            if s.starts_with('/') {
                let trimmed = s.trim_start_matches('/');
                return PathBuf::from(format!(r"C:\{}", trimmed.replace('/', r"\")));
            }
        }
    }
    path.to_path_buf()
}

fn run_perf_self_test(root: &Path) -> Result<(), String> {
    assert_le_float("self_float_pass", 1.0, 1.0)?;
    if assert_le_float("self_float_fail", 2.0, 1.0).is_ok() {
        return Err("perf self-test failed: float assertion should fail when value > max".into());
    }
    assert_le_int("self_int_pass", 1, 2)?;
    if assert_le_int("self_int_fail", 3, 2).is_ok() {
        return Err("perf self-test failed: int assertion should fail when value > max".into());
    }
    let mut floats = vec![1.0, 3.0, 2.0];
    let mut ints = vec![100u64, 300, 200];
    if median_f64(&mut floats) != 2.0 || median_u64(&mut ints) != 200 {
        return Err("perf self-test failed: median calculation mismatch".into());
    }
    let out_dir = root.join("tmp").join("perf").join("self-test");
    fs::create_dir_all(&out_dir)
        .map_err(|e| format!("create perf self-test dir `{}`: {e}", out_dir.display()))?;
    let artifact = json!({
      "schema_version": 1,
      "sample_runs": PERF_SAMPLE_RUNS_DEFAULT,
      "aggregation_mode": "median",
      "thresholds": {
        "startup_latency_s_max": STARTUP_LATENCY_MAX_S_DEFAULT,
        "pkg_resolution_latency_s_max": PKG_RESOLUTION_LATENCY_MAX_S_DEFAULT,
        "runtime_link_startup_latency_s_max": RUNTIME_LINK_STARTUP_LATENCY_MAX_S_DEFAULT,
        "max_rss_kb": MAX_RSS_KB_DEFAULT,
        "max_cpu_percent": MAX_CPU_PERCENT_DEFAULT
      },
      "measurements": {
        "startup": { "latency_s": 0.1, "rss_kb": 1000, "cpu_percent": 10.0 },
        "pkg_resolution": { "latency_s": 0.2, "rss_kb": 1100, "cpu_percent": 20.0 },
        "runtime_link_startup": { "latency_s": 0.3, "rss_kb": 1200, "cpu_percent": 30.0 }
      }
    });
    let path = out_dir.join("milestone2-performance.json");
    write_json_pretty(&path, &artifact)?;
    let parsed: Value = serde_json::from_slice(
        &fs::read(&path).map_err(|e| format!("read perf self-test artifact: {e}"))?,
    )
    .map_err(|e| format!("parse perf self-test artifact: {e}"))?;
    if parsed["schema_version"] != Value::from(1)
        || parsed["aggregation_mode"] != Value::from("median")
    {
        return Err("perf self-test failed: artifact schema mismatch".into());
    }
    Ok(())
}

fn run_milestone2_supply_chain_gate(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    if raw_args.len() == 1 && raw_args[0] == "--self-test" {
        run_supply_chain_self_test(root)?;
        println!("milestone_2 supply-chain gate self-test passed");
        return Ok(());
    }
    if !raw_args.is_empty() {
        return Err("usage: cargo run -p xtask -- milestone2-supply-chain-gate [--self-test]".into());
    }

    let out_dir = env::var("CLG_SUPPLY_CHAIN_OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("tmp").join("sbom"));
    fs::create_dir_all(&out_dir)
        .map_err(|e| format!("create supply-chain output dir `{}`: {e}", out_dir.display()))?;

    let metadata = load_cargo_metadata(root)?;
    let dep_report = build_dependency_report(&metadata)?;
    let pkg_report = build_package_artifact_report(root)?;
    let runtime_report = build_runtime_dependency_report(root)?;

    write_json_pretty(&out_dir.join("milestone2-sbom.json"), &dep_report.report)?;
    write_json_pretty(
        &out_dir.join("milestone2-package-artifact-report.json"),
        &pkg_report.report,
    )?;
    write_json_pretty(
        &out_dir.join("milestone2-runtime-dependency-report.json"),
        &runtime_report.report,
    )?;
    let summary = json!({
      "schema_version": 1,
      "dependency_violations": dep_report.violations.len(),
      "package_artifact_violations": pkg_report.violations.len(),
      "runtime_dependency_violations": runtime_report.violations.len(),
      "ok": dep_report.violations.is_empty() && pkg_report.violations.is_empty() && runtime_report.violations.is_empty()
    });
    write_json_pretty(&out_dir.join("milestone2-supply-chain-summary.json"), &summary)?;

    if !dep_report.violations.is_empty() {
        eprintln!("dependency license compliance violations detected");
        for entry in dep_report.violations.iter().take(10) {
            eprintln!("- {}", entry);
        }
    }
    if !pkg_report.violations.is_empty() {
        eprintln!("package metadata artifact compliance violations detected");
        for entry in pkg_report.violations.iter().take(10) {
            eprintln!("- {}", entry);
        }
    }
    if !runtime_report.violations.is_empty() {
        eprintln!("runtime dependency compliance violations detected");
        for entry in runtime_report.violations.iter().take(10) {
            eprintln!("- {}", entry);
        }
    }

    if dep_report.violations.is_empty()
        && pkg_report.violations.is_empty()
        && runtime_report.violations.is_empty()
    {
        println!("milestone_2 supply-chain gate passed");
        println!("artifacts written to {}", out_dir.display());
        Ok(())
    } else {
        Err("milestone_2 supply-chain gate failed".into())
    }
}

fn load_cargo_metadata(root: &Path) -> Result<Value, String> {
    if let Ok(path) = env::var("CLG_SUPPLY_CHAIN_METADATA_JSON") {
        return serde_json::from_slice(
            &fs::read(&path).map_err(|e| format!("read metadata override `{path}`: {e}"))?,
        )
        .map_err(|e| format!("parse metadata override `{path}`: {e}"));
    }
    let output = run_capture_output(
        Command::new("cargo")
            .arg("metadata")
            .arg("--format-version")
            .arg("1")
            .arg("--locked")
            .current_dir(root),
    )?;
    serde_json::from_slice(&output).map_err(|e| format!("parse cargo metadata output: {e}"))
}

fn build_dependency_report(metadata: &Value) -> Result<SupplyChainReport, String> {
    let mut package_map = std::collections::BTreeMap::<String, &Value>::new();
    if let Some(packages) = metadata.get("packages").and_then(Value::as_array) {
        for pkg in packages {
            if let Some(id) = pkg.get("id").and_then(Value::as_str) {
                package_map.insert(id.to_string(), pkg);
            }
        }
    }
    let workspace_members: std::collections::BTreeSet<String> = metadata
        .get("workspace_members")
        .and_then(Value::as_array)
        .map(|members| {
            members
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();

    let mut node_ids = Vec::<String>::new();
    if let Some(nodes) = metadata
        .get("resolve")
        .and_then(|v| v.get("nodes"))
        .and_then(Value::as_array)
    {
        for node in nodes {
            if let Some(id) = node.get("id").and_then(Value::as_str) {
                node_ids.push(id.to_string());
            }
        }
    }
    if node_ids.is_empty() {
        node_ids.extend(package_map.keys().cloned());
    }
    node_ids.sort();
    node_ids.dedup();

    let mut entries = Vec::<Value>::new();
    let mut violations = Vec::<Value>::new();
    for pkg_id in node_ids {
        let Some(pkg) = package_map.get(&pkg_id) else {
            continue;
        };
        let license = pkg
            .get("license")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let license_file = pkg
            .get("license_file")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let compliant = !license.is_empty() || !license_file.is_empty();
        let row = json!({
          "id": pkg_id,
          "name": pkg.get("name").and_then(Value::as_str),
          "version": pkg.get("version").and_then(Value::as_str),
          "source": pkg.get("source").and_then(Value::as_str),
          "license": if license.is_empty() { Value::Null } else { Value::String(license) },
          "license_file": if license_file.is_empty() { Value::Null } else { Value::String(license_file) },
          "workspace_member": workspace_members.contains(pkg.get("id").and_then(Value::as_str).unwrap_or("")),
          "compliant": compliant
        });
        if !compliant {
            violations.push(row.clone());
        }
        entries.push(row);
    }
    Ok(SupplyChainReport {
        report: json!({
          "schema_version": 1,
          "kind": "cargo_dependency_license_sbom",
          "packages": entries
        }),
        violations,
    })
}

fn build_package_artifact_report(root: &Path) -> Result<SupplyChainReport, String> {
    let mut metadata_files = collect_tracked_package_metadata_paths(root)?;
    metadata_files.extend(find_files_by_name(
        &root.join("tmp").join("std-core"),
        "clg.package-metadata.json",
    ));
    metadata_files.sort();
    metadata_files.dedup();

    let mut entries = Vec::<Value>::new();
    let mut violations = Vec::<Value>::new();
    for metadata_path in metadata_files.into_iter().filter(|path| path.exists()) {
        let data: Value = serde_json::from_slice(
            &fs::read(&metadata_path)
                .map_err(|e| format!("read package metadata `{}`: {e}", metadata_path.display()))?,
        )
        .map_err(|e| format!("parse package metadata `{}`: {e}", metadata_path.display()))?;
        let schema_version = data.get("schema_version").and_then(Value::as_i64);
        if let Some(packages) = data.get("packages").and_then(Value::as_array) {
            let mut sorted_packages = packages.clone();
            sorted_packages.sort_by(|a, b| {
                let an = a.get("name").and_then(Value::as_str).unwrap_or("");
                let av = a.get("version").and_then(Value::as_str).unwrap_or("");
                let bn = b.get("name").and_then(Value::as_str).unwrap_or("");
                let bv = b.get("version").and_then(Value::as_str).unwrap_or("");
                (an, av).cmp(&(bn, bv))
            });
            for pkg in sorted_packages {
                let digest = pkg
                    .get("digest")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let artifact_path = pkg
                    .get("artifact")
                    .and_then(|v| v.get("path"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let artifact_exists = if artifact_path.is_empty() {
                    false
                } else {
                    metadata_path
                        .parent()
                        .unwrap_or(root)
                        .join(&artifact_path)
                        .exists()
                };
                let compliant = matches!(schema_version, Some(0) | Some(1))
                    && digest.starts_with("sha256:")
                    && !artifact_path.is_empty()
                    && artifact_exists;
                let row = json!({
                  "metadata_file": normalize_rel_path(root, &metadata_path),
                  "name": pkg.get("name").and_then(Value::as_str),
                  "version": pkg.get("version").and_then(Value::as_str),
                  "schema_version": schema_version,
                  "digest": if digest.is_empty() { Value::Null } else { Value::String(digest) },
                  "artifact_path": if artifact_path.is_empty() { Value::Null } else { Value::String(artifact_path) },
                  "artifact_exists": artifact_exists,
                  "compliant": compliant
                });
                if !compliant {
                    violations.push(row.clone());
                }
                entries.push(row);
            }
        }
    }
    Ok(SupplyChainReport {
        report: json!({
          "schema_version": 1,
          "kind": "package_metadata_artifact_compliance",
          "entries": entries
        }),
        violations,
    })
}

fn build_runtime_dependency_report(root: &Path) -> Result<SupplyChainReport, String> {
    let mut runtime_files = collect_tracked_runtime_link_paths(root)?;
    runtime_files.extend(find_files_by_name(
        &root.join("tmp").join("perf"),
        "clg.runtime-link.json",
    ));
    runtime_files.sort();
    runtime_files.dedup();

    let mut entries = Vec::<Value>::new();
    let mut violations = Vec::<Value>::new();
    if runtime_files.is_empty() {
        let row = json!({
          "entry_kind": "runtime_link_presence",
          "runtime_link_file": Value::Null,
          "compliant": false,
          "reason": "missing_runtime_link_artifacts"
        });
        violations.push(row.clone());
        entries.push(row);
    }

    for runtime_link_path in runtime_files.into_iter().filter(|path| path.exists()) {
        let data: Value = serde_json::from_slice(
            &fs::read(&runtime_link_path).map_err(|e| {
                format!("read runtime-link artifact `{}`: {e}", runtime_link_path.display())
            })?,
        )
        .map_err(|e| format!("parse runtime-link artifact `{}`: {e}", runtime_link_path.display()))?;
        let schema_version = data.get("schema_version").and_then(Value::as_i64);
        let lock_path = runtime_link_path.parent().unwrap_or(root).join("clg.lock.json");
        let store_index_path = runtime_link_path
            .parent()
            .unwrap_or(root)
            .join("clg.package-store-index.json");
        let lock_map = read_lock_digests(&lock_path)?;
        let store_map = read_store_digests(&store_index_path)?;

        let mut package_ids = std::collections::BTreeSet::<String>::new();
        let mut packages = data
            .get("packages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        packages.sort_by(|a, b| {
            a.get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .cmp(b.get("id").and_then(Value::as_str).unwrap_or(""))
        });
        for pkg in packages {
            let package_id = pkg
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            if !package_id.is_empty() {
                package_ids.insert(package_id.clone());
            }
            let digest = pkg
                .get("digest")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            let artifact_path = pkg
                .get("artifact_path")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            let artifact_full = runtime_link_path.parent().unwrap_or(root).join(&artifact_path);
            let artifact_exists = !artifact_path.is_empty() && artifact_full.exists();
            let digest_matches_artifact = if artifact_exists && digest.starts_with("sha256:") {
                sha256_hex(&artifact_full)
                    .map(|sum| digest == format!("sha256:{sum}"))
                    .unwrap_or(false)
            } else {
                false
            };
            let lock_digest_matches = !package_id.is_empty()
                && lock_map
                    .get(&package_id)
                    .map(|d| d == &digest)
                    .unwrap_or(false);
            let store_digest_matches = !package_id.is_empty()
                && store_map
                    .get(&package_id)
                    .map(|d| d == &digest)
                    .unwrap_or(false);
            let compliant = matches!(schema_version, Some(0) | Some(1))
                && !package_id.is_empty()
                && digest.starts_with("sha256:")
                && !artifact_path.is_empty()
                && artifact_exists
                && digest_matches_artifact
                && lock_digest_matches
                && store_digest_matches;
            let row = json!({
              "entry_kind": "runtime_package",
              "runtime_link_file": normalize_rel_path(root, &runtime_link_path),
              "lockfile_present": lock_path.exists(),
              "store_index_present": store_index_path.exists(),
              "schema_version": schema_version,
              "package_id": if package_id.is_empty() { Value::Null } else { Value::String(package_id.clone()) },
              "digest": if digest.is_empty() { Value::Null } else { Value::String(digest.clone()) },
              "artifact_path": if artifact_path.is_empty() { Value::Null } else { Value::String(artifact_path) },
              "artifact_exists": artifact_exists,
              "digest_matches_artifact": digest_matches_artifact,
              "lock_digest_matches": lock_digest_matches,
              "store_digest_matches": store_digest_matches,
              "compliant": compliant
            });
            if !compliant {
                violations.push(row.clone());
            }
            entries.push(row);
        }

        let mut bindings = data
            .get("bindings")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        bindings.sort_by(|a, b| {
            let ak = (
                a.get("import_module").and_then(Value::as_str).unwrap_or(""),
                a.get("import_name").and_then(Value::as_str).unwrap_or(""),
                a.get("provider_package_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            );
            let bk = (
                b.get("import_module").and_then(Value::as_str).unwrap_or(""),
                b.get("import_name").and_then(Value::as_str).unwrap_or(""),
                b.get("provider_package_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            );
            ak.cmp(&bk)
        });
        for binding in bindings {
            let provider_package_id = binding
                .get("provider_package_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            let compliant =
                !provider_package_id.is_empty() && package_ids.contains(&provider_package_id);
            let row = json!({
              "entry_kind": "runtime_binding",
              "runtime_link_file": normalize_rel_path(root, &runtime_link_path),
              "import_module": binding.get("import_module").and_then(Value::as_str),
              "import_name": binding.get("import_name").and_then(Value::as_str),
              "provider_package_id": if provider_package_id.is_empty() { Value::Null } else { Value::String(provider_package_id) },
              "compliant": compliant
            });
            if !compliant {
                violations.push(row.clone());
            }
            entries.push(row);
        }
    }

    Ok(SupplyChainReport {
        report: json!({
          "schema_version": 1,
          "kind": "runtime_dependency_compliance",
          "entries": entries
        }),
        violations,
    })
}

fn collect_tracked_package_metadata_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    if let Ok(raw) = env::var("CLG_SUPPLY_CHAIN_TRACKED_METADATA") {
        let mut out = Vec::new();
        for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
            out.push(root.join(line));
        }
        return Ok(out);
    }
    match run_capture_output(
        Command::new("git")
            .arg("ls-files")
            .arg("--")
            .arg("**/clg.package-metadata.json")
            .current_dir(root),
    ) {
        Ok(stdout) => Ok(String::from_utf8_lossy(&stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|rel| root.join(rel))
            .collect()),
        Err(_) => {
            let mut out = Vec::new();
            recurse_files(root, &mut out, "clg.package-metadata.json", &[".git", "target"]);
            Ok(out)
        }
    }
}

fn collect_tracked_runtime_link_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    if let Ok(raw) = env::var("CLG_SUPPLY_CHAIN_RUNTIME_LINKS") {
        let mut out = Vec::new();
        for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
            out.push(root.join(line));
        }
        return Ok(out);
    }
    match run_capture_output(
        Command::new("git")
            .arg("ls-files")
            .arg("--")
            .arg("**/clg.runtime-link.json")
            .current_dir(root),
    ) {
        Ok(stdout) => Ok(String::from_utf8_lossy(&stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|rel| root.join(rel))
            .collect()),
        Err(_) => Ok(Vec::new()),
    }
}

fn find_files_by_name(base: &Path, name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    recurse_files(base, &mut out, name, &[]);
    out
}

fn recurse_files(base: &Path, out: &mut Vec<PathBuf>, target_name: &str, skip_dirs: &[&str]) {
    if !base.exists() || !base.is_dir() {
        return;
    }
    let entries = match fs::read_dir(base) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if skip_dirs.iter().any(|skip| *skip == name) {
                continue;
            }
            recurse_files(&path, out, target_name, skip_dirs);
            continue;
        }
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name == target_name)
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
}

fn normalize_rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"))
}

fn read_lock_digests(path: &Path) -> Result<std::collections::BTreeMap<String, String>, String> {
    if !path.exists() {
        return Ok(std::collections::BTreeMap::new());
    }
    let data: Value = serde_json::from_slice(
        &fs::read(path).map_err(|e| format!("read lockfile `{}`: {e}", path.display()))?,
    )
    .map_err(|e| format!("parse lockfile `{}`: {e}", path.display()))?;
    let mut out = std::collections::BTreeMap::new();
    if let Some(packages) = data.get("packages").and_then(Value::as_array) {
        for pkg in packages {
            if let (Some(id), Some(digest)) = (
                pkg.get("id").and_then(Value::as_str),
                pkg.get("digest").and_then(Value::as_str),
            ) {
                out.insert(id.to_string(), digest.to_string());
            }
        }
    }
    Ok(out)
}

fn read_store_digests(path: &Path) -> Result<std::collections::BTreeMap<String, String>, String> {
    if !path.exists() {
        return Ok(std::collections::BTreeMap::new());
    }
    let data: Value = serde_json::from_slice(
        &fs::read(path).map_err(|e| format!("read store index `{}`: {e}", path.display()))?,
    )
    .map_err(|e| format!("parse store index `{}`: {e}", path.display()))?;
    let mut out = std::collections::BTreeMap::new();
    if let Some(artifacts) = data.get("artifacts").and_then(Value::as_array) {
        for artifact in artifacts {
            if let (Some(id), Some(digest)) = (
                artifact.get("id").and_then(Value::as_str),
                artifact.get("digest").and_then(Value::as_str),
            ) {
                out.insert(id.to_string(), digest.to_string());
            }
        }
    }
    Ok(out)
}

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
