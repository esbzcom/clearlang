use clg_codegen_wasm::emit_from_ir;
use clg_ir::{Function as IrFunction, Instr, IrType, Module as IrModule, Value};
use wasmtime::{Engine, Instance, Module, Store, Val};

fn simple_return_wasm() -> Vec<u8> {
    let main_fn = IrFunction {
        name: "main".to_string(),
        params: vec![],
        ret: Some(IrType::Int),
        body: vec![
            Instr::IConst {
                dst: Value(0),
                ty: IrType::Int,
                n: 0,
            },
            Instr::Ret { val: Value(0) },
        ],
    };
    let module = IrModule {
        funcs: vec![main_fn],
    };
    emit_from_ir(&module).expect("codegen ok")
}

fn two_empty_loops_wasm() -> Vec<u8> {
    let main_fn = IrFunction {
        name: "main".to_string(),
        params: vec![],
        ret: Some(IrType::Int),
        body: vec![
            Instr::IConst {
                dst: Value(0),
                ty: IrType::Int,
                n: 0,
            },
            Instr::BlockBegin,
            Instr::LoopBegin,
            Instr::BrIfEqz {
                cond: Value(0),
                depth: 1,
            },
            Instr::Br { depth: 0 },
            Instr::LoopEnd,
            Instr::BlockEnd,
            Instr::BlockBegin,
            Instr::LoopBegin,
            Instr::BrIfEqz {
                cond: Value(0),
                depth: 1,
            },
            Instr::Br { depth: 0 },
            Instr::LoopEnd,
            Instr::BlockEnd,
            Instr::IConst {
                dst: Value(1),
                ty: IrType::Int,
                n: 0,
            },
            Instr::Ret { val: Value(1) },
        ],
    };
    let module = IrModule {
        funcs: vec![main_fn],
    };
    emit_from_ir(&module).expect("codegen ok")
}

fn nested_loops_wasm() -> Vec<u8> {
    let main_fn = IrFunction {
        name: "main".to_string(),
        params: vec![],
        ret: Some(IrType::Int),
        body: vec![
            Instr::IConst {
                dst: Value(0),
                ty: IrType::Int,
                n: 1,
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
            Instr::BlockBegin,
            Instr::LoopBegin,
            Instr::BrIfEqz {
                cond: Value(1),
                depth: 1,
            },
            Instr::IConst {
                dst: Value(2),
                ty: IrType::Int,
                n: 1,
            },
            Instr::IBin {
                dst: Value(1),
                op: clg_ir::BinOpIR::Sub,
                lhs: Value(1),
                rhs: Value(2),
                ty: IrType::Int,
            },
            Instr::Br { depth: 0 },
            Instr::LoopEnd,
            Instr::BlockEnd,
            Instr::IConst {
                dst: Value(3),
                ty: IrType::Int,
                n: 1,
            },
            Instr::IBin {
                dst: Value(0),
                op: clg_ir::BinOpIR::Sub,
                lhs: Value(0),
                rhs: Value(3),
                ty: IrType::Int,
            },
            Instr::Br { depth: 0 },
            Instr::LoopEnd,
            Instr::BlockEnd,
            Instr::IConst {
                dst: Value(4),
                ty: IrType::Int,
                n: 0,
            },
            Instr::Ret { val: Value(4) },
        ],
    };
    let module = IrModule {
        funcs: vec![main_fn],
    };
    emit_from_ir(&module).expect("codegen ok")
}

fn get_i32_global(instance: &Instance, store: &mut Store<()>, name: &str) -> i32 {
    let global = instance.get_global(&mut *store, name).expect("global");
    global.get(&mut *store).i32().expect("i32 global")
}

fn set_i32_global(instance: &Instance, store: &mut Store<()>, name: &str, value: i32) {
    let global = instance.get_global(&mut *store, name).expect("global");
    global
        .set(&mut *store, Val::I32(value))
        .expect("set global");
}

#[test]
fn meter_traps_on_exact_entry_budget() {
    let wasm = simple_return_wasm();
    let engine = Engine::default();
    let module = Module::from_binary(&engine, &wasm).expect("module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("instantiate");

    set_i32_global(&instance, &mut store, "__clg_fuel_remaining", 1_000);
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("main");
    assert!(main.call(&mut store, ()).is_err());
    let code = get_i32_global(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 5);
}

#[test]
fn meter_allows_entry_with_margin() {
    let wasm = simple_return_wasm();
    let engine = Engine::default();
    let module = Module::from_binary(&engine, &wasm).expect("module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("instantiate");

    set_i32_global(&instance, &mut store, "__clg_fuel_remaining", 1_001);
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("main");
    let result = main.call(&mut store, ());
    assert!(result.is_ok());
    let remaining = get_i32_global(&instance, &mut store, "__clg_fuel_remaining");
    assert_eq!(remaining, 1);
}

#[test]
fn meter_counts_multiple_calls() {
    let wasm = simple_return_wasm();
    let engine = Engine::default();
    let module = Module::from_binary(&engine, &wasm).expect("module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("instantiate");

    set_i32_global(&instance, &mut store, "__clg_fuel_remaining", 2_001);
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("main");
    assert!(main.call(&mut store, ()).is_ok());
    assert!(main.call(&mut store, ()).is_ok());
    let remaining = get_i32_global(&instance, &mut store, "__clg_fuel_remaining");
    assert_eq!(remaining, 1);
}

#[test]
fn meter_charges_multiple_loop_headers() {
    let wasm = two_empty_loops_wasm();
    let engine = Engine::default();
    let module = Module::from_binary(&engine, &wasm).expect("module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("instantiate");

    set_i32_global(&instance, &mut store, "__clg_fuel_remaining", 1_002);
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("main");
    assert!(main.call(&mut store, ()).is_err());
    let code = get_i32_global(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 5);
}

#[test]
fn meter_charges_nested_loop_headers() {
    let wasm = nested_loops_wasm();
    let engine = Engine::default();
    let module = Module::from_binary(&engine, &wasm).expect("module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("instantiate");

    set_i32_global(&instance, &mut store, "__clg_fuel_remaining", 1_004);
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("main");
    assert!(main.call(&mut store, ()).is_err());
    let code = get_i32_global(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 5);
}
