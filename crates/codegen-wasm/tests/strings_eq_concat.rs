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

