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

