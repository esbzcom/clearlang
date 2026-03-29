use std::fs;
use std::path::Path;

#[test]
fn phase25_reserved_solver_build_codes_are_registered_in_diagnostics_doc() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let diagnostics = fs::read_to_string(root.join("docs").join("diagnostics.md"))
        .expect("read docs/diagnostics.md");
    for code in ["C124", "C125", "C126", "C127"] {
        let needle = format!("| {code} |");
        assert!(
            diagnostics.contains(needle.as_str()),
            "missing reserved phase 25 build diagnostic code in docs: {code}"
        );
    }
}

#[test]
fn phase25_reserved_solver_verify_codes_are_registered_in_diagnostics_doc() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let diagnostics = fs::read_to_string(root.join("docs").join("diagnostics.md"))
        .expect("read docs/diagnostics.md");
    for code in ["V006"] {
        let needle = format!("| {code} |");
        assert!(
            diagnostics.contains(needle.as_str()),
            "missing reserved phase 25 verify diagnostic code in docs: {code}"
        );
    }
}
