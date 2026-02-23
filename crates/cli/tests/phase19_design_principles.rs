use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn phase19_design_docs_include_design_principles_gate() {
    let design_dir = repo_root().join("docs").join("design");
    let mut phase19_docs = Vec::new();
    for entry in fs::read_dir(&design_dir).expect("read docs/design directory") {
        let entry = entry.expect("read docs/design entry");
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.starts_with("phase-19") && name.ends_with(".md") && path.is_file() {
            phase19_docs.push(path);
        }
    }

    phase19_docs.sort();
    assert!(
        !phase19_docs.is_empty(),
        "expected at least one phase-19 design doc in docs/design"
    );

    let required_labels = [
        "Simple for users",
        "AI-friendly",
        "Provably correct",
        "Crypto-focused",
    ];

    for doc in phase19_docs {
        let content = fs::read_to_string(&doc).expect("read phase-19 design doc");
        assert!(
            content.contains("## Design Principles Check"),
            "phase-19 design doc missing '## Design Principles Check': {}",
            doc.display()
        );
        for label in required_labels {
            assert!(
                content.contains(label),
                "phase-19 design doc missing required design-principle label `{}`: {}",
                label,
                doc.display()
            );
        }
    }
}
