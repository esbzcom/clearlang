use clg_codegen_wasm::emit_trivial_main;

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|win| win == needle)
}

#[test]
fn hello_wasm_exports_main_and_returns_42() {
    let bytes = emit_trivial_main().expect("emit ok");

    // Magic header
    assert!(bytes.len() > 8);
    assert_eq!(&bytes[0..4], b"\0asm");

    // Export name contains "main"
    assert!(contains(&bytes, b"main"), "export name 'main' not found");

    // Code contains i32.const 42 (0x41 0x2A), followed by end (0x0B)
    assert!(
        contains(&bytes, &[0x41, 0x2A, 0x0B]),
        "expected instruction sequence i32.const 42; end not found"
    );
}

