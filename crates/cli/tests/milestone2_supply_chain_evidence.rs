use std::fs;
use std::path::Path;

#[test]
fn milestone2_supply_chain_evidence_doc_references_gate_and_artifacts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let evidence_path = root
        .join("docs")
        .join("evidence")
        .join("milestone_2-supply-chain.md");
    let xtask_path = root
        .join("xtask")
        .join("src")
        .join("main")
        .join("milestone2_gates.rs");
    let content =
        fs::read_to_string(&evidence_path).expect("read milestone_2 supply-chain evidence");
    let xtask_source =
        fs::read_to_string(&xtask_path).expect("read xtask supply-chain gate source");

    assert!(
        content.contains("cargo run -p xtask -- milestone2-supply-chain-gate"),
        "supply-chain evidence should reference the xtask gate command"
    );
    assert!(
        content.contains("milestone2-sbom.json")
            && content.contains("milestone2-package-artifact-report.json")
            && content.contains("milestone2-runtime-dependency-report.json")
            && content.contains("milestone2-supply-chain-summary.json"),
        "supply-chain evidence should reference emitted compliance artifacts"
    );
    assert!(
        content.contains("including workspace crates"),
        "supply-chain evidence should lock workspace crate license coverage"
    );
    assert!(
        content.contains("after std-core artifact reproducibility"),
        "supply-chain evidence should lock CI execution order for generated-artifact coverage"
    );
    assert!(
        content.contains("tracked `clg.package-metadata.json` files")
            && content.contains("tmp/std-core/**/clg.package-metadata.json"),
        "supply-chain evidence should lock deterministic package-metadata scan scope"
    );
    assert!(
        content.contains("runtime-link artifacts")
            && content.contains("clg.runtime-link.json")
            && content.contains("tmp/perf/**/clg.runtime-link.json"),
        "supply-chain evidence should explicitly lock runtime dependency validation scope"
    );
    for artifact_name in [
        "milestone2-sbom.json",
        "milestone2-package-artifact-report.json",
        "milestone2-runtime-dependency-report.json",
        "milestone2-supply-chain-summary.json",
    ] {
        assert!(
            xtask_source.contains(artifact_name),
            "supply-chain gate should emit `{artifact_name}`"
        );
    }
    for summary_key in [
        "\"dependency_violations\"",
        "\"package_artifact_violations\"",
        "\"runtime_dependency_violations\"",
    ] {
        assert!(
            xtask_source.contains(summary_key),
            "supply-chain gate summary should include `{summary_key}`"
        );
    }
    assert!(
        xtask_source.contains("missing_runtime_link_artifacts"),
        "supply-chain gate should fail closed when runtime-link artifacts are absent"
    );
}
