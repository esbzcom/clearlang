use std::fs;
use std::path::Path;

#[test]
fn phase23_reserved_runtime_loader_codes_are_registered_in_diagnostics_doc() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let diagnostics = fs::read_to_string(root.join("docs").join("diagnostics.md"))
        .expect("read docs/diagnostics.md");
    for code in ["R012", "R013", "R014", "R015", "R016", "R017"] {
        let needle = format!("| {code} | runtime |");
        assert!(
            diagnostics.contains(needle.as_str()),
            "missing or wrong-stage reserved phase 23 diagnostic row in docs: {code}"
        );
    }
}
