use std::fs;
use std::path::Path;

fn extract_env_default(script: &str, name: &str) -> String {
    let prefix = format!(r#"{name}="${{{name}:-"#);
    let start = script
        .find(&prefix)
        .unwrap_or_else(|| panic!("perf gate script should define `{name}` default"));
    let rest = &script[start + prefix.len()..];
    let end = rest
        .find("}")
        .unwrap_or_else(|| panic!("perf gate script default for `{name}` should terminate"));
    rest[..end].to_string()
}

#[test]
fn milestone2_performance_evidence_doc_references_gate_script_and_artifact() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let evidence_path = root
        .join("docs")
        .join("evidence")
        .join("milestone_2-performance.md");
    let script_path = root
        .join("scripts")
        .join("ci")
        .join("milestone2_perf_gate.sh");
    let content =
        fs::read_to_string(&evidence_path).expect("read milestone_2 performance evidence doc");
    let script = fs::read_to_string(&script_path).expect("read milestone2 perf gate script");

    assert!(
        content.contains("milestone2_perf_gate.sh"),
        "performance evidence doc should reference CI gate script"
    );
    assert!(
        content.contains("milestone2-performance.json"),
        "performance evidence doc should reference machine-readable performance artifact"
    );
    assert!(
        content.contains("startup overhead") && content.contains("package resolution/link"),
        "performance evidence doc should include startup and package-resolution metrics"
    );
    assert!(
        content.contains("runtime-link startup latency"),
        "performance evidence doc should include runtime-link startup metric"
    );
    assert!(
        content.contains("median"),
        "performance evidence doc should describe median aggregation semantics"
    );
    for env_name in [
        "STARTUP_LATENCY_MAX_S",
        "PKG_RESOLUTION_LATENCY_MAX_S",
        "RUNTIME_LINK_STARTUP_LATENCY_MAX_S",
        "MAX_RSS_KB",
        "MAX_CPU_PERCENT",
        "PERF_SAMPLE_RUNS",
    ] {
        assert!(
            content.contains(env_name),
            "performance evidence doc should list `{env_name}` override"
        );
    }
    let startup_max = extract_env_default(&script, "STARTUP_LATENCY_MAX_S");
    let pkg_max = extract_env_default(&script, "PKG_RESOLUTION_LATENCY_MAX_S");
    let runtime_max = extract_env_default(&script, "RUNTIME_LINK_STARTUP_LATENCY_MAX_S");
    let rss_max = extract_env_default(&script, "MAX_RSS_KB");
    let cpu_max = extract_env_default(&script, "MAX_CPU_PERCENT");
    let sample_runs = extract_env_default(&script, "PERF_SAMPLE_RUNS");
    assert!(
        content.contains(&format!("<= `{startup_max}s`")),
        "performance evidence startup threshold should match script default"
    );
    assert!(
        content.contains(&format!("<= `{pkg_max}s`")),
        "performance evidence package-resolution threshold should match script default"
    );
    assert!(
        content.contains(&format!("<= `{runtime_max}s`")),
        "performance evidence runtime-link threshold should match script default"
    );
    assert!(
        content.contains(&format!("<= `{rss_max} KB`")),
        "performance evidence RSS threshold should match script default"
    );
    assert!(
        content.contains(&format!("<= `{cpu_max}%`")),
        "performance evidence CPU threshold should match script default"
    );
    assert!(
        content.contains(&format!("across `{sample_runs}` runs")),
        "performance evidence should match script sample-run default"
    );
    assert!(
        script.contains(r#""sample_runs":"#) || script.contains(r#""sample_runs": "#),
        "perf gate artifact should include sample run metadata"
    );
    assert!(
        script.contains(r#""aggregation_mode": "median""#),
        "perf gate artifact should declare median aggregation mode"
    );
}
