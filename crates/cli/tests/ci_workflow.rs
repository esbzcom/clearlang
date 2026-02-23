use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn ci_workflow_enforces_validation_and_tests() {
    let ci_path = repo_root().join(".github").join("workflows").join("ci.yml");
    let contents = fs::read_to_string(&ci_path).expect("read ci workflow");
    assert!(
        contents.contains("wasm-tools validate"),
        "ci should validate emitted wasm with wasm-tools"
    );
    assert!(
        contents.contains("cargo test --workspace"),
        "ci should run workspace tests"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test cli_it sdk_usability_"),
        "ci should enforce sdk usability gate tests"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test cli_it migration_"),
        "ci should enforce migration-friction regression tests"
    );
    assert!(
        contents.contains("cargo test -p clg-cli --test cli_it imports::"),
        "ci should enforce import ergonomics gate tests"
    );
    assert!(
        contents.contains("cargo fmt --all -- --check"),
        "ci should enforce formatting"
    );
    assert!(
        contents.contains("cargo clippy --workspace --all-targets -- -D warnings"),
        "ci should enforce clippy warnings"
    );
}
