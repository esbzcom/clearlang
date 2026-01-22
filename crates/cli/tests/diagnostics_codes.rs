use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn is_code(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 4 {
        return false;
    }
    matches!(bytes[0], b'P' | b'T' | b'C' | b'V' | b'R')
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
}

fn extract_codes_from_table(doc: &str) -> Vec<String> {
    let mut codes = Vec::new();
    for line in doc.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let mut cells = trimmed.trim_matches('|').split('|').map(|c| c.trim());
        let code = cells.next().unwrap_or("");
        if is_code(code) {
            codes.push(code.to_string());
        }
    }
    codes
}

fn extract_codes_from_text(text: &str) -> HashSet<String> {
    let mut codes = HashSet::new();
    let bytes = text.as_bytes();
    if bytes.len() < 4 {
        return codes;
    }
    for i in 0..=(bytes.len() - 4) {
        let b0 = bytes[i];
        let b1 = bytes[i + 1];
        let b2 = bytes[i + 2];
        let b3 = bytes[i + 3];
        if !matches!(b0, b'P' | b'T' | b'C' | b'V' | b'R')
            || !b1.is_ascii_digit()
            || !b2.is_ascii_digit()
            || !b3.is_ascii_digit()
        {
            continue;
        }
        let prev_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
        let next_ok = i + 4 >= bytes.len() || !bytes[i + 4].is_ascii_alphanumeric();
        if prev_ok && next_ok {
            let code = std::str::from_utf8(&bytes[i..i + 4]).unwrap();
            codes.insert(code.to_string());
        }
    }
    codes
}

fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn collect_src_codes(root: &Path) -> HashSet<String> {
    let crates_dir = root.join("crates");
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(crates_dir) {
        for entry in entries.flatten() {
            let src = entry.path().join("src");
            if src.is_dir() {
                collect_rs_files(&src, &mut files);
            }
        }
    }
    let mut codes = HashSet::new();
    for file in files {
        if let Ok(text) = fs::read_to_string(&file) {
            for code in extract_codes_from_text(&text) {
                codes.insert(code);
            }
        }
    }
    codes
}

#[test]
fn error_codes_are_documented_and_unique() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let doc_path = root.join("docs").join("diagnostics.md");
    let doc = fs::read_to_string(&doc_path).expect("read diagnostics.md");
    let doc_codes = extract_codes_from_table(&doc);
    assert!(
        !doc_codes.is_empty(),
        "expected at least one code in diagnostics.md"
    );

    let mut seen = HashSet::new();
    let mut dupes = Vec::new();
    for code in &doc_codes {
        if !seen.insert(code.clone()) {
            dupes.push(code.clone());
        }
    }
    assert!(dupes.is_empty(), "duplicate codes in docs: {dupes:?}");

    let doc_set: HashSet<String> = doc_codes.into_iter().collect();
    let src_codes = collect_src_codes(&root);
    let mut missing: Vec<String> = src_codes
        .into_iter()
        .filter(|code| !doc_set.contains(code))
        .collect();
    missing.sort();
    assert!(missing.is_empty(), "undocumented error codes: {missing:?}");
}
