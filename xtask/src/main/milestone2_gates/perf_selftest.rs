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
    if parsed.get("schema_version").and_then(Value::as_i64) != Some(1)
        || parsed
            .get("aggregation_mode")
            .and_then(Value::as_str)
            != Some("median")
    {
        return Err("perf self-test failed: artifact schema mismatch".into());
    }
    Ok(())
}

