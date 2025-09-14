use lumi_codegen_wasm::emit_from_ir;
use lumi_parser::parse;
use lumi_typer::check;

fn run_main_i32(wasm: &[u8]) -> i32 {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::from_binary(&engine, wasm).expect("module from bytes");
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance.get_typed_func::<(), i32>(&mut store, "main").expect("get main");
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
