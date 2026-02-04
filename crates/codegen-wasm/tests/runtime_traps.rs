use clg_codegen_wasm::emit_from_ir;
use clg_codegen_wasm::intrinsics::runtime::{emit_runtime_trap, TrapOperand};
use clg_ir::{
    Function as IrFunction, Instr as IrInstr, IrType, Module as IrModule, TrapCode, Value,
    VariantKind,
};
use clg_parser::parse;
use clg_typer::check;
use wasm_encoder::{CodeSection, ConstExpr, ExportKind, ExportSection, Function, FunctionSection,
                   GlobalSection, GlobalType, Module, TypeSection, ValType};

mod common;

fn get_global(instance: &wasmtime::Instance, store: &mut wasmtime::Store<()>, name: &str) -> i32 {
    let g = instance
        .get_global(&mut *store, name)
        .expect("global present");
    g.get(&mut *store).i32().expect("i32 global")
}

#[test]
fn trap_r002_invalid_utf8_from_len() {
    // Build IR that calls std::str::len with an invalid pointer (1)
    let len_fn = IrFunction {
        name: "std::str::len".to_string(),
        params: vec![IrType::Int],
        ret: Some(IrType::Int),
        body: vec![],
    };
    let main_fn = IrFunction {
        name: "main".to_string(),
        params: vec![],
        ret: Some(IrType::Int),
        body: vec![
            IrInstr::IConst {
                dst: Value(0),
                ty: IrType::Int,
                n: 1,
            },
            IrInstr::Call {
                dst: Some(Value(1)),
                callee: 0, // index of std::str::len in funcs below
                args: vec![Value(0)],
            },
            IrInstr::Ret { val: Value(1) },
        ],
    };
    let ir = IrModule {
        funcs: vec![len_fn, main_fn],
    };
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module from bytes");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, ());
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 3, "expected R002 (InvalidUtf8) trap code");
}

#[test]
fn trap_r001_concat_oom_and_heap_ptr_unchanged() {
    // Build a program with two large literals so concat would exceed memory and OOM before writing.
    let big_a = "a".repeat(40_000);
    let big_b = "b".repeat(40_000);
    let src = format!(
        "function main() -> Int {{ std::str::len(std::str::concat({a:?}, {b:?})) }}",
        a = big_a,
        b = big_b,
    );
    let ast = parse(&src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module from bytes");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");

    // heap ptr before call
    let heap_before = get_global(&instance, &mut store, "__clg_heap_ptr");

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, ());
    assert!(err.is_err(), "expected trap, got ok result");

    let code = get_global(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 2, "expected R001 (AllocatorOom) trap code");

    let heap_after = get_global(&instance, &mut store, "__clg_heap_ptr");
    assert_eq!(heap_before, heap_after, "heap_ptr must be unchanged on OOM");
}

#[test]
fn trap_r003_invalid_tag_from_variant_load() {
    let main_fn = IrFunction {
        name: "main".to_string(),
        params: vec![],
        ret: Some(IrType::Int),
        body: vec![
            IrInstr::IConst {
                dst: Value(0),
                ty: IrType::Int,
                n: 2,
            },
            IrInstr::IConst {
                dst: Value(1),
                ty: IrType::Int,
                n: 0,
            },
            IrInstr::IConst {
                dst: Value(2),
                ty: IrType::Int,
                n: 0,
            },
            IrInstr::VariantInit {
                dst: Value(3),
                tag: Value(0),
                payload_lo: Value(1),
                payload_hi: Value(2),
            },
            IrInstr::VariantLoadTag {
                dst: Value(4),
                variant: Value(3),
                kind: VariantKind::Option,
            },
            IrInstr::Ret { val: Value(4) },
        ],
    };
    let ir = IrModule {
        funcs: vec![main_fn],
    };
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, ());
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 4, "expected R003 (InvalidVariantTag) trap code");
}

#[test]
fn trap_r009_collection_bounds() {
    let mut module = Module::new();
    let mut types = TypeSection::new();
    types.ty().function([], []);
    module.section(&types);

    let mut funcs = FunctionSection::new();
    funcs.function(0);
    module.section(&funcs);

    let mut globals = GlobalSection::new();
    let gty = GlobalType {
        val_type: ValType::I32,
        mutable: true,
        shared: false,
    };
    for _ in 0..5 {
        globals.global(gty, &ConstExpr::i32_const(0));
    }
    module.section(&globals);

    let mut exports = ExportSection::new();
    exports.export("__clg_runtime_error_code", ExportKind::Global, 1);
    exports.export("__clg_runtime_error_start", ExportKind::Global, 2);
    exports.export("__clg_runtime_error_end", ExportKind::Global, 3);
    exports.export("__clg_runtime_error_detail", ExportKind::Global, 4);
    exports.export("main", ExportKind::Func, 0);
    module.section(&exports);

    let mut code = CodeSection::new();
    let mut f = Function::new(Vec::new());
    {
        let mut insts = f.instructions();
        emit_runtime_trap(
            &mut insts,
            TrapCode::CollectionBounds,
            TrapOperand::zero(),
            TrapOperand::zero(),
            0,
        );
        insts.end();
    }
    code.function(&f);
    module.section(&code);

    let wasm = module.finish();

    let engine = common::engine();
    let module = wasmtime::Module::from_binary(engine, &wasm).expect("module");
    let mut store = common::store(engine);
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let main = instance
        .get_typed_func::<(), ()>(&mut store, "main")
        .expect("get main");
    let err = main.call(&mut store, ());
    assert!(err.is_err(), "expected trap, got ok result");
    let code = get_global(&instance, &mut store, "__clg_runtime_error_code");
    assert_eq!(code, 10, "expected R009 (CollectionBounds) trap code");
}
