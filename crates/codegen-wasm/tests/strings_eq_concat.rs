use clg_codegen_wasm::emit_from_ir;
use clg_parser::parse;
use clg_typer::check;
mod common;

fn run_main_i32(wasm: &[u8]) -> i32 {
    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, wasm).expect("module from bytes");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    main.call(&mut store, ()).expect("invoke main")
}

#[test]
fn str_eq_true_and_false() {
    // main returns Bool (as i32) to avoid needing if-lowering
    let src_true = r#" function main() -> Bool { std::str::eq("aa", "aa") } "#;
    let ast_t = parse(src_true).expect("parse ok");
    let ir_t = check(&ast_t).expect("type-check+lower ok");
    let wasm_t = emit_from_ir(&ir_t).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_t), 1);

    let src_false = r#" function main() -> Bool { std::str::eq("a", "b") } "#;
    let ast_f = parse(src_false).expect("parse ok");
    let ir_f = check(&ast_f).expect("type-check+lower ok");
    let wasm_f = emit_from_ir(&ir_f).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_f), 0);
}

#[test]
fn str_eq_concat_vs_literal() {
    let src_true = r#"
        function main() -> Bool { std::str::eq(std::str::concat("a", "b"), "ab") }
    "#;
    let ast_t = parse(src_true).expect("parse ok");
    let ir_t = check(&ast_t).expect("type-check+lower ok");
    let wasm_t = emit_from_ir(&ir_t).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_t), 1);

    let src_false = r#"
        function main() -> Bool { std::str::eq(std::str::concat("a", "c"), "ab") }
    "#;
    let ast_f = parse(src_false).expect("parse ok");
    let ir_f = check(&ast_f).expect("type-check+lower ok");
    let wasm_f = emit_from_ir(&ir_f).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_f), 0);
}

#[test]
fn str_concat_then_len() {
    let src = r#"
        function main() -> Int { std::str::len(std::str::concat("ab", "c")) }
    "#;
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm), 3);
}

#[test]
fn str_eq_empty_and_multibyte() {
    // Empty strings
    let src1 = r#" function main() -> Bool { std::str::eq("", "") } "#;
    let ast1 = parse(src1).expect("parse ok");
    let ir1 = check(&ast1).expect("type-check+lower ok");
    let wasm1 = emit_from_ir(&ir1).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm1), 1);

    let src2 = r#" function main() -> Bool { std::str::eq("", "a") } "#;
    let ast2 = parse(src2).expect("parse ok");
    let ir2 = check(&ast2).expect("type-check+lower ok");
    let wasm2 = emit_from_ir(&ir2).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm2), 0);

    // Multibyte: "é" is two bytes in UTF-8
    let src3 = r#" function main() -> Bool { std::str::eq("é", "é") } "#;
    let ast3 = parse(src3).expect("parse ok");
    let ir3 = check(&ast3).expect("type-check+lower ok");
    let wasm3 = emit_from_ir(&ir3).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm3), 1);

    let src4 = r#" function main() -> Bool { std::str::eq("é", "e") } "#;
    let ast4 = parse(src4).expect("parse ok");
    let ir4 = check(&ast4).expect("type-check+lower ok");
    let wasm4 = emit_from_ir(&ir4).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm4), 0);
}

#[test]
fn str_concat_edge_cases_len() {
    let src1 = r#" function main() -> Int { std::str::len(std::str::concat("", "x")) } "#;
    let ast1 = parse(src1).expect("parse ok");
    let ir1 = check(&ast1).expect("type-check+lower ok");
    let wasm1 = emit_from_ir(&ir1).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm1), 1);

    let src2 = r#" function main() -> Int { std::str::len(std::str::concat("é", "")) } "#;
    let ast2 = parse(src2).expect("parse ok");
    let ir2 = check(&ast2).expect("type-check+lower ok");
    let wasm2 = emit_from_ir(&ir2).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm2), 2);

    let src3 = r#" function main() -> Int { std::str::len(std::str::concat("é", "é")) } "#;
    let ast3 = parse(src3).expect("parse ok");
    let ir3 = check(&ast3).expect("type-check+lower ok");
    let wasm3 = emit_from_ir(&ir3).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm3), 4);
}

#[test]
fn str_starts_with_and_ends_with() {
    let src_starts_true = r#" function main() -> Bool { std::str::starts_with("abcdef", "abc") } "#;
    let ast_st = parse(src_starts_true).expect("parse ok");
    let ir_st = check(&ast_st).expect("type-check+lower ok");
    let wasm_st = emit_from_ir(&ir_st).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_st), 1);

    let src_starts_false =
        r#" function main() -> Bool { std::str::starts_with("abcdef", "abd") } "#;
    let ast_sf = parse(src_starts_false).expect("parse ok");
    let ir_sf = check(&ast_sf).expect("type-check+lower ok");
    let wasm_sf = emit_from_ir(&ir_sf).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_sf), 0);

    let src_ends_true = r#" function main() -> Bool { std::str::ends_with("abcdef", "def") } "#;
    let ast_et = parse(src_ends_true).expect("parse ok");
    let ir_et = check(&ast_et).expect("type-check+lower ok");
    let wasm_et = emit_from_ir(&ir_et).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_et), 1);

    let src_ends_false = r#" function main() -> Bool { std::str::ends_with("abcdef", "deg") } "#;
    let ast_ef = parse(src_ends_false).expect("parse ok");
    let ir_ef = check(&ast_ef).expect("type-check+lower ok");
    let wasm_ef = emit_from_ir(&ir_ef).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_ef), 0);
}

#[test]
fn str_contains_and_to_bytes() {
    let src_contains_true = r#" function main() -> Bool { std::str::contains("abcdef", "cde") } "#;
    let ast_ct = parse(src_contains_true).expect("parse ok");
    let ir_ct = check(&ast_ct).expect("type-check+lower ok");
    let wasm_ct = emit_from_ir(&ir_ct).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_ct), 1);

    let src_contains_false = r#" function main() -> Bool { std::str::contains("abcdef", "cdx") } "#;
    let ast_cf = parse(src_contains_false).expect("parse ok");
    let ir_cf = check(&ast_cf).expect("type-check+lower ok");
    let wasm_cf = emit_from_ir(&ir_cf).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_cf), 0);

    let src_empty_needle = r#" function main() -> Bool { std::str::contains("abcdef", "") } "#;
    let ast_en = parse(src_empty_needle).expect("parse ok");
    let ir_en = check(&ast_en).expect("type-check+lower ok");
    let wasm_en = emit_from_ir(&ir_en).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_en), 1);

    let src_to_bytes_len =
        r#" function main() -> Int { std::bytes::len(std::str::to_bytes("abc")) } "#;
    let ast_tb = parse(src_to_bytes_len).expect("parse ok");
    let ir_tb = check(&ast_tb).expect("type-check+lower ok");
    let wasm_tb = emit_from_ir(&ir_tb).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_tb), 3);
}

#[test]
fn str_pattern_matches_contract() {
    let src_true = r#" function main() -> Bool { std::str_pattern::matches("cde", "abcdef") } "#;
    let ast_t = parse(src_true).expect("parse ok");
    let ir_t = check(&ast_t).expect("type-check+lower ok");
    let wasm_t = emit_from_ir(&ir_t).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_t), 1);

    let src_false = r#" function main() -> Bool { std::str_pattern::matches("cdx", "abcdef") } "#;
    let ast_f = parse(src_false).expect("parse ok");
    let ir_f = check(&ast_f).expect("type-check+lower ok");
    let wasm_f = emit_from_ir(&ir_f).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_f), 0);

    let src_empty = r#" function main() -> Bool { std::str_pattern::matches("", "abcdef") } "#;
    let ast_e = parse(src_empty).expect("parse ok");
    let ir_e = check(&ast_e).expect("type-check+lower ok");
    let wasm_e = emit_from_ir(&ir_e).expect("codegen ok");
    assert_eq!(run_main_i32(&wasm_e), 1);
}
