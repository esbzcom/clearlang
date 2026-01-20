use clg_codegen_wasm::emit_from_ir;
use clg_ir::{BinOpIR, Function as IrFunction, Instr, IrType, Module as IrModule, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use wasmtime::{Instance, Module, UpdateDeadline};

mod common;

fn bounded_loop_wasm(iterations: i64) -> Vec<u8> {
    let main_fn = IrFunction {
        name: "main".to_string(),
        params: vec![],
        ret: Some(IrType::Int),
        body: vec![
            Instr::IConst {
                dst: Value(0),
                ty: IrType::Int,
                n: iterations,
            },
            Instr::BlockBegin,
            Instr::LoopBegin,
            Instr::BrIfEqz {
                cond: Value(0),
                depth: 1,
            },
            Instr::IConst {
                dst: Value(1),
                ty: IrType::Int,
                n: 1,
            },
            Instr::IBin {
                dst: Value(0),
                op: BinOpIR::Sub,
                lhs: Value(0),
                rhs: Value(1),
            },
            Instr::Br { depth: 0 },
            Instr::LoopEnd,
            Instr::BlockEnd,
            Instr::Ret { val: Value(0) },
        ],
    };
    let module = IrModule {
        funcs: vec![main_fn],
    };
    emit_from_ir(&module).expect("codegen ok")
}

#[test]
fn fuel_consumption_decrements() {
    let wasm = bounded_loop_wasm(1_000);
    let engine = common::engine();
    let module = Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let initial_fuel = 50_000;
    store.set_fuel(initial_fuel).expect("set fuel");
    let instance = Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("main");
    let before = store.get_fuel().expect("get fuel");
    let _ = main.call(&mut store, ()).expect("call");
    let after = store.get_fuel().expect("get fuel");
    assert!(
        after < before,
        "expected fuel to be consumed (before {before}, after {after})"
    );
}

#[test]
fn epoch_deadline_callback_runs() {
    let wasm = bounded_loop_wasm(1_000);
    let engine = common::engine();
    let module = Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_for_cb = Arc::clone(&hits);
    store.epoch_deadline_callback(move |_store| {
        hits_for_cb.fetch_add(1, Ordering::Relaxed);
        Ok(UpdateDeadline::Continue(1))
    });
    store.set_epoch_deadline(1);
    engine.increment_epoch();
    let instance = Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("main");
    assert!(
        main.call(&mut store, ()).is_ok(),
        "expected epoch callback to allow execution"
    );
    assert!(
        hits.load(Ordering::Relaxed) > 0,
        "expected epoch callback to run"
    );
}
