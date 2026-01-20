use clg_codegen_wasm::emit_from_ir;
use clg_ir::{Function as IrFunction, Instr as IrInstr, IrType, Module as IrModule, Value};
use std::convert::TryInto;

mod common;

fn get_global(instance: &wasmtime::Instance, store: &mut wasmtime::Store<()>, name: &str) -> i32 {
    instance
        .get_global(&mut *store, name)
        .expect("global present")
        .get(&mut *store)
        .i32()
        .expect("i32 global")
}

#[test]
fn variant_init_allocates_and_writes_layout() {
    let main_fn = IrFunction {
        name: "main".to_string(),
        params: vec![],
        ret: Some(IrType::Int),
        body: vec![
            IrInstr::IConst {
                dst: Value(0),
                ty: IrType::Int,
                n: 42,
            },
            IrInstr::IConst {
                dst: Value(1),
                ty: IrType::Int,
                n: 1,
            },
            IrInstr::IConst {
                dst: Value(2),
                ty: IrType::Int,
                n: 0,
            },
            IrInstr::VariantInit {
                dst: Value(3),
                tag: Value(1),
                payload_lo: Value(0),
                payload_hi: Value(2),
            },
            IrInstr::Ret { val: Value(3) },
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

    let heap_before = get_global(&instance, &mut store, "__clg_heap_ptr");

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .expect("get main");
    let ptr = main.call(&mut store, ()).expect("call main");

    assert_eq!(
        ptr, heap_before,
        "allocation should return previous heap pointer"
    );
    assert_eq!(ptr & 0xF, 0, "variant pointer must be 16-byte aligned");

    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("memory export");
    let mut buf = [0u8; 16];
    memory
        .read(&mut store, ptr as usize, &mut buf)
        .expect("read variant bytes");

    let tag = i32::from_le_bytes(buf[0..4].try_into().unwrap());
    let payload_lo = i32::from_le_bytes(buf[4..8].try_into().unwrap());
    let payload_hi = i32::from_le_bytes(buf[8..12].try_into().unwrap());
    let reserved = i32::from_le_bytes(buf[12..16].try_into().unwrap());

    assert_eq!(tag, 1, "tag for Some/Ok must be 1");
    assert_eq!(payload_lo, 42, "payload_lo stores the constructor value");
    assert_eq!(payload_hi, 0, "payload_hi defaults to zero for scalars");
    assert_eq!(reserved, 0, "reserved slot must be zeroed");

    let heap_after = get_global(&instance, &mut store, "__clg_heap_ptr");
    let expected_end = (heap_before + 16 + 15) & !15;
    assert_eq!(
        heap_after, expected_end,
        "heap pointer must advance by aligned variant size"
    );
}
