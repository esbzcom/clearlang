use clg_codegen_wasm::emit_from_ir;
use clg_ir::{Instr, IrType, Value};
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

fn get_global_i32(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<()>,
    name: &str,
) -> i32 {
    let global = instance
        .get_global(&mut *store, name)
        .expect("global export present");
    global.get(&mut *store).i32().expect("global should be i32")
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

#[test]
fn dispatcher_unknown_code_id_traps_deterministically() {
    let src = r#"
function make() -> function(Int) -> Int {
    (x: Int) => x + 1
}

io function apply(f: function(Int) -> Int, x: Int) -> Int {
    f(x)
}

io function main() -> Int {
    apply(make(), 5)
}
"#;

    let ast = parse(src).expect("parse ok");
    let mut ir = check(&ast).expect("type-check+lower ok");
    let dispatcher_idx = ir
        .funcs
        .iter()
        .position(|f| f.name.starts_with("__clg_dispatch_"))
        .expect("dispatcher should be generated") as u32;
    let main_fn_idx = ir
        .funcs
        .iter()
        .position(|f| f.name == "main")
        .expect("main should exist");
    ir.funcs[main_fn_idx].body = vec![
        Instr::IConst {
            dst: Value(0),
            ty: IrType::Int,
            n: 9_999,
        },
        Instr::IConst {
            dst: Value(1),
            ty: IrType::Int,
            n: 0,
        },
        Instr::IConst {
            dst: Value(2),
            ty: IrType::Int,
            n: 5,
        },
        Instr::Call {
            dst: Some(Value(3)),
            callee: dispatcher_idx,
            args: vec![Value(0), Value(1), Value(2)],
        },
        Instr::Ret { val: Value(3) },
    ];

    let wasm = emit_from_ir(&ir).expect("codegen ok");
    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, ());
    assert!(err.is_err(), "expected dispatcher unknown code_id trap");
    let code = get_global_i32(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(
        code, 12,
        "expected R011 (ClosureDispatchUnknownCode) trap code"
    );
}
