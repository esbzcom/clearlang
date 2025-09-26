use clg_codegen_wasm::{emit_from_ir_with_opts, CodegenOpts};
use clg_ir as ir;
use wasmparser::{Parser, Payload};

fn build_ir_module_for_types() -> ir::Module {
    // f1: (Int, Int) -> Int  returns first param
    let f1 = ir::Function {
        name: "f1".to_string(),
        params: vec![ir::IrType::Int, ir::IrType::Int],
        ret: Some(ir::IrType::Int),
        body: vec![ir::Instr::Ret { val: ir::Value(0) }],
    };
    // f2: (Int, Int) -> Int  identical signature
    let f2 = ir::Function {
        name: "f2".to_string(),
        params: vec![ir::IrType::Int, ir::IrType::Int],
        ret: Some(ir::IrType::Int),
        body: vec![ir::Instr::Ret { val: ir::Value(1) }],
    };
    // f3: () -> Int  returns const 7
    let f3 = ir::Function {
        name: "f3".to_string(),
        params: vec![],
        ret: Some(ir::IrType::Int),
        body: vec![
            ir::Instr::IConst {
                dst: ir::Value(0),
                ty: ir::IrType::Int,
                n: 7,
            },
            ir::Instr::Ret { val: ir::Value(0) },
        ],
    };
    // main: () -> Int just calls f3 or returns const
    let main = ir::Function {
        name: "main".to_string(),
        params: vec![],
        ret: Some(ir::IrType::Int),
        body: vec![
            ir::Instr::IConst {
                dst: ir::Value(0),
                ty: ir::IrType::Int,
                n: 42,
            },
            ir::Instr::Ret { val: ir::Value(0) },
        ],
    };
    ir::Module {
        funcs: vec![f1, f2, f3, main],
    }
}

#[test]
fn type_section_is_deduplicated_by_signature() {
    let m = build_ir_module_for_types();
    let wasm = emit_from_ir_with_opts(
        &m,
        CodegenOpts {
            debug_names: false,
            proof_section: None,
        },
    )
    .expect("codegen");
    // Count number of function types actually declared in the Type section
    let mut ty_count = 0usize;
    for payload in Parser::new(0).parse_all(&wasm) {
        if let Payload::TypeSection(rdr) = payload.expect("payload") {
            // In current wasmparser, the type section yields RecGroup entries.
            // Our encoder emits one type per group, so counting groups == types.
            for group in rdr {
                let _ = group.expect("group");
                ty_count += 1;
            }
        }
    }
    // Expect only two unique func types: (i32,i32)->i32 and ()->i32
    assert_eq!(
        ty_count, 2,
        "expected 2 deduped function types, got {}",
        ty_count
    );
}

#[test]
fn debug_names_emit_name_section() {
    let m = build_ir_module_for_types();
    let wasm = emit_from_ir_with_opts(
        &m,
        CodegenOpts {
            debug_names: true,
            proof_section: None,
        },
    )
    .expect("codegen");
    // Look for a custom section named "name"
    let mut has_name = false;
    for payload in Parser::new(0).parse_all(&wasm) {
        if let Payload::CustomSection(reader) = payload.expect("payload") {
            if reader.name() == "name" {
                has_name = true;
                break;
            }
        }
    }
    assert!(
        has_name,
        "expected custom name section when debug_names is enabled"
    );
}

#[test]
fn proof_section_is_embedded() {
    let m = build_ir_module_for_types();
    let wasm = emit_from_ir_with_opts(
        &m,
        CodegenOpts {
            debug_names: false,
            proof_section: Some(vec![0xAA, 0xBB, 0xCC]),
        },
    )
    .expect("codegen");
    let mut found = false;
    for payload in Parser::new(0).parse_all(&wasm) {
        if let Payload::CustomSection(reader) = payload.expect("payload") {
            if reader.name() == "clearlang.proof" {
                assert_eq!(reader.data(), &[0xAA, 0xBB, 0xCC]);
                found = true;
                break;
            }
        }
    }
    assert!(found, "expected clearlang.proof custom section");
}
