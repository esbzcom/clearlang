use anyhow::{anyhow, Result};
use clg_codegen_wasm::emit_from_ir;
use clg_parser::parse;
use clg_typer::check;
use wasmtime::{Instance, Module, Store};

mod common;

fn build_and_run(src: &str) -> Result<i32> {
    let ast = parse(src).map_err(|e| anyhow!(e))?;
    let ir = check(&ast)?;
    let wasm = emit_from_ir(&ir)?;
    let engine = common::engine();
    let module = Module::from_binary(engine, &wasm)?;
    let mut store = Store::new(engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let main = instance.get_typed_func::<(), i32>(&mut store, "main")?;
    let result = main.call(&mut store, ())?;
    Ok(result)
}

#[test]
fn while_false_returns_tail() -> Result<()> {
    let src = r#"
function main() -> Int {
    while false invariant { true } variant { 0 } {
        1;
    }
    99
}
"#;

    let out = build_and_run(src)?;
    assert_eq!(out, 99);
    Ok(())
}

#[test]
fn invariant_runs_even_without_iterations() {
    let src = r#"
function main() -> Int {
    while false invariant { false } variant { 0 } {
        1;
    }
    1
}
"#;

    assert!(
        build_and_run(src).is_err(),
        "loop invariant should trap even when cond is false"
    );
}

#[test]
fn variant_must_decrease_each_iteration() {
    let src = r#"
function main() -> Int {
    while true invariant { true } variant { 1 } {
        0;
    }
    0
}
"#;

    assert!(
        build_and_run(src).is_err(),
        "non-decreasing variant should trap"
    );
}
