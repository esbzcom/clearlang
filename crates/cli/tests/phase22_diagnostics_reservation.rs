use std::fs;
use std::path::Path;

#[test]
fn phase22_reserved_build_codes_are_registered_in_diagnostics_doc() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let diagnostics = fs::read_to_string(root.join("docs").join("diagnostics.md"))
        .expect("read docs/diagnostics.md");
    for code in [
        "C109", "C110", "C111", "C112", "C113", "C114", "C115", "C116", "C117", "C118", "C119",
    ] {
        let needle = format!("| {code} |");
        assert!(
            diagnostics.contains(needle.as_str()),
            "missing reserved phase 22 diagnostic code in docs: {code}"
        );
    }
}
