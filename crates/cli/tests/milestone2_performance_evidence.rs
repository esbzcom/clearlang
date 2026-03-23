use std::fs;
use std::path::Path;

fn extract_xtask_const(xtask_source: &str, name: &str) -> String {
    let prefix = format!("const {name}:");
    for line in xtask_source.lines() {
        let line = line.trim();
        if line.starts_with(&prefix) {
            let rhs = line
                .split_once('=')
                .map(|(_, rhs)| rhs.trim().trim_end_matches(';'))
                .unwrap_or_else(|| panic!("xtask const `{name}` should have initializer"));
            return rhs.replace('_', "");
        }
    }
    panic!("xtask source should define const `{name}`");
}

#[test]
fn milestone2_performance_evidence_doc_references_gate_script_and_artifact() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let evidence_path = root
        .join("docs")
        .join("evidence")
        .join("milestone_2-performance.md");
    let xtask_path = root
        .join("xtask")
        .join("src")
        .join("main")
        .join("milestone2_gates.rs");
    let content =
        fs::read_to_string(&evidence_path).expect("read milestone_2 performance evidence doc");
    let xtask_source = fs::read_to_string(&xtask_path).expect("read xtask milestone2 gate source");

    assert!(
        content.contains("cargo run -p xtask -- milestone2-perf-gate"),
        "performance evidence doc should reference xtask perf gate command"
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
    let startup_max = extract_xtask_const(&xtask_source, "STARTUP_LATENCY_MAX_S_DEFAULT");
    let pkg_max = extract_xtask_const(&xtask_source, "PKG_RESOLUTION_LATENCY_MAX_S_DEFAULT");
    let runtime_max =
        extract_xtask_const(&xtask_source, "RUNTIME_LINK_STARTUP_LATENCY_MAX_S_DEFAULT");
    let rss_max = extract_xtask_const(&xtask_source, "MAX_RSS_KB_DEFAULT");
    let cpu_max = extract_xtask_const(&xtask_source, "MAX_CPU_PERCENT_DEFAULT");
    let sample_runs = extract_xtask_const(&xtask_source, "PERF_SAMPLE_RUNS_DEFAULT");
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
        content.contains(&format!("<= `{cpu_max}%`"))
            || content.contains(&format!("<= `{}%`", cpu_max.trim_end_matches(".0"))),
        "performance evidence CPU threshold should match script default"
    );
    assert!(
        content.contains(&format!("across `{sample_runs}` runs")),
        "performance evidence should match script sample-run default"
    );
    assert!(
        xtask_source.contains("\"sample_runs\""),
        "perf gate artifact should include sample run metadata"
    );
    assert!(
        xtask_source.contains(r#""aggregation_mode": "median""#),
        "perf gate artifact should declare median aggregation mode"
    );
}
