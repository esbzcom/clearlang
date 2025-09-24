use clg_codegen_wasm::emit_trivial_main;

#[test]
fn builds_hello_wasm_module() {
    let bytes = emit_trivial_main().expect("emit ok");
    // Check WASM binary magic header 0x00 0x61 0x73 0x6D
    assert!(bytes.len() > 8);
    assert_eq!(&bytes[0..4], b"\0asm");
}

