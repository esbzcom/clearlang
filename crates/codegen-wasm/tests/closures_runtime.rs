use clg_codegen_wasm::emit_from_ir;
use clg_parser::parse;
use clg_typer::check;

mod common;

fn run_main_i32(src: &str) -> i32 {
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    main.call(&mut store, ()).expect("call main")
}

#[test]
fn dynamic_closure_dispatch_runs_for_non_capturing_lambda() {
    let src = r#"
io function apply(f: function(Int) -> Int, x: Int) -> Int {
    f(x)
}

function make() -> function(Int) -> Int {
    (x: Int) => x + 1
}

io function main() -> Int {
    apply(make(), 5)
}
"#;

    assert_eq!(run_main_i32(src), 6);
}

#[test]
fn dynamic_closure_dispatch_runs_for_capturing_lambda() {
    let src = r#"
io function apply(f: function(Int) -> Int, x: Int) -> Int {
    f(x)
}

function make(base: Int) -> function(Int) -> Int {
    (x: Int) => x + base
}

io function main() -> Int {
    apply(make(2), 40)
}
"#;

    assert_eq!(run_main_i32(src), 42);
}
