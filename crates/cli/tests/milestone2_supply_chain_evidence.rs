use std::fs;
use std::path::Path;

#[test]
fn milestone2_supply_chain_evidence_doc_references_gate_and_artifacts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root
        .join("docs")
        .join("evidence")
        .join("milestone_2-supply-chain.md");
    let content = fs::read_to_string(&path).expect("read milestone_2 supply-chain evidence");

    assert!(
        content.contains("milestone2_supply_chain_gate.py"),
        "supply-chain evidence should reference the CI gate script"
    );
    assert!(
        content.contains("milestone2-sbom.json")
            && content.contains("milestone2-package-artifact-report.json")
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
}
