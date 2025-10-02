use anyhow::Result;
use clg_ir::{BinOpIR, Function as IrFunction, Instr as IrInstr, Module as IrModule, TrapCode};
use std::collections::HashMap;
use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, CustomSection, DataSection, ExportKind, ExportSection,
    Function, FunctionSection, GlobalSection, GlobalType, MemArg, MemorySection, MemoryType,
    Module, NameMap, NameSection, TypeSection, ValType,
};

use crate::intrinsics::{
    runtime::{emit_guard_trap, emit_runtime_trap, TrapOperand},
    strings::{encode_intrinsic_str_concat, encode_intrinsic_str_eq, encode_intrinsic_str_len},
};

pub(crate) const HEAP_PTR_GLOBAL: u32 = 0;
pub(crate) const ERROR_CODE_GLOBAL: u32 = 1;
pub(crate) const ERROR_START_GLOBAL: u32 = 2;
pub(crate) const ERROR_END_GLOBAL: u32 = 3;
pub(crate) const ERROR_DETAIL_GLOBAL: u32 = 4;

#[derive(Default)]
pub struct CodegenOpts {
    pub debug_names: bool,
    pub proof_section: Option<Vec<u8>>,
}

pub fn emit_from_ir_with_opts(ir: &IrModule, opts: CodegenOpts) -> Result<Vec<u8>> {
    let mut module = Module::new();

    // String pool: collect unique string literals and assign memory offsets
    let mut str_pool: HashMap<String, u32> = HashMap::new();
    let mut cur_off: u32 = 0;
    for f in &ir.funcs {
        for ins in &f.body {
            if let IrInstr::IStringConst { s, .. } = ins {
                if !str_pool.contains_key(s) {
                    let len = s.len() as u32;
                    let off = cur_off;
                    let size = 4 + len; // header + bytes
                    let next = (off + size + 3) & !3; // 4-byte align
                    str_pool.insert(s.clone(), off);
                    cur_off = next;
                }
            }
        }
    }

    // Build function types with deduplication, and record each function's type index
    #[derive(Hash, Eq, PartialEq, Clone)]
    struct SigKey {
        params: usize,
        has_ret: bool,
    }
    let mut types = TypeSection::new();
    let mut sig_to_tyidx = HashMap::<SigKey, u32>::new();
    let mut fn_type_indices: Vec<u32> = Vec::with_capacity(ir.funcs.len());
    for f in &ir.funcs {
        let key = SigKey {
            params: f.params.len(),
            has_ret: f.ret.is_some(),
        };
        let ty_idx = if let Some(idx) = sig_to_tyidx.get(&key) {
            *idx
        } else {
            let params: Vec<ValType> = f.params.iter().map(|_| ValType::I32).collect();
            let results: Vec<ValType> = match f.ret {
                Some(_) => vec![ValType::I32],
                None => vec![],
            };
            let idx = sig_to_tyidx.len() as u32;
            types.ty().function(params, results);
            sig_to_tyidx.insert(key, idx);
            idx
        };
        fn_type_indices.push(ty_idx);
    }
    module.section(&types);

    // Function section references the deduplicated type indices per function
    let mut functions = FunctionSection::new();
    for ty_idx in &fn_type_indices {
        functions.function(*ty_idx);
    }
    module.section(&functions);

    // Memory section (prepare for string runtime)
    // Ensure the initial memory is large enough to hold all string data segments.
    let heap_start = (cur_off + 3) & !3; // first free address after literals, 4-byte aligned
    let min_pages = u64::from(heap_start).div_ceil(65536).max(1);
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: min_pages,
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    module.section(&memories);

    // Global bump allocator pointer initialized after string data
    let mut globals = GlobalSection::new();
    // global 0: (mut i32) heap_ptr
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(heap_start as i32),
    );
    // global 1: last runtime error code (0 = none)
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(0),
    );
    // global 2: last runtime error span start
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(0),
    );
    // global 3: last runtime error span end
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(0),
    );
    // global 4: last runtime error detail (e.g., guard kind)
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(0),
    );
    module.section(&globals);

    // Export main + runtime bookkeeping globals
    let mut exports = ExportSection::new();
    if let Some((i, _)) = ir.funcs.iter().enumerate().find(|(_, f)| f.name == "main") {
        exports.export("main", ExportKind::Func, i as u32);
    }
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("__clg_heap_ptr", ExportKind::Global, HEAP_PTR_GLOBAL);
    exports.export(
        "__clg_runtime_error_code",
        ExportKind::Global,
        ERROR_CODE_GLOBAL,
    );
    exports.export(
        "__clg_runtime_error_start",
        ExportKind::Global,
        ERROR_START_GLOBAL,
    );
    exports.export(
        "__clg_runtime_error_end",
        ExportKind::Global,
        ERROR_END_GLOBAL,
    );
    exports.export(
        "__clg_runtime_error_detail",
        ExportKind::Global,
        ERROR_DETAIL_GLOBAL,
    );
    module.section(&exports);

    // Code section: encode each function body
    let mut codes = CodeSection::new();
    for f in &ir.funcs {
        // Encode intrinsics with custom bodies; other functions from IR
        let func = match f.name.as_str() {
            "std::str::len" => encode_intrinsic_str_len(f)?,
            "std::str::eq" => encode_intrinsic_str_eq(f)?,
            "std::str::concat" => encode_intrinsic_str_concat(f)?,
            _ => encode_ir_function(f, &str_pool)?,
        };
        codes.function(&func);
    }
    module.section(&codes);

    // Data section for string literals (after Code as per section order)
    if !str_pool.is_empty() {
        let mut data = DataSection::new();
        // Sort by offset for deterministic emission
        let mut items: Vec<(u32, &String)> = str_pool.iter().map(|(s, off)| (*off, s)).collect();
        items.sort_by_key(|(off, _)| *off);
        for (off, s) in items {
            let bytes = s.as_bytes();
            let mut init = Vec::with_capacity(4 + bytes.len());
            init.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            init.extend_from_slice(bytes);
            data.active(0, &ConstExpr::i32_const(off as i32), init);
        }
        module.section(&data);
    }

    // Optional debug name section
    if opts.debug_names {
        let mut names = NameSection::new();
        let mut fn_names = NameMap::new();
        for (i, f) in ir.funcs.iter().enumerate() {
            fn_names.append(i as u32, &f.name);
        }
        names.functions(&fn_names);
        module.section(&names);
    }

    if let Some(proof_bytes) = opts.proof_section {
        let custom = CustomSection {
            name: "clearlang.proof".into(),
            data: proof_bytes.into(),
        };
        module.section(&custom);
    }

    Ok(module.finish())
}

// Backwards-compatible helper with default options
pub fn emit_from_ir(ir: &IrModule) -> Result<Vec<u8>> {
    emit_from_ir_with_opts(ir, CodegenOpts::default())
}

fn encode_ir_function(f: &IrFunction, strs: &HashMap<String, u32>) -> Result<Function> {
    // Compute locals: values >= params are locals; params are indices 0..P-1
    let params_len = f.params.len() as u32;
    let mut max_id = params_len.saturating_sub(1);
    for ins in &f.body {
        match ins {
            IrInstr::IConst { dst, .. } => max_id = max_id.max(dst.0),
            IrInstr::IStringConst { dst, .. } => max_id = max_id.max(dst.0),
            IrInstr::IBin { dst, lhs, rhs, .. } => {
                max_id = max_id.max(dst.0).max(lhs.0).max(rhs.0);
            }
            IrInstr::Guard { cond, .. } => {
                max_id = max_id.max(cond.0);
            }
            IrInstr::ISelect {
                dst,
                cond,
                then_v,
                else_v,
            } => {
                max_id = max_id.max(dst.0).max(cond.0).max(then_v.0).max(else_v.0);
            }
            IrInstr::VariantInit {
                dst,
                tag,
                payload_lo,
                payload_hi,
            } => {
                max_id = max_id
                    .max(dst.0)
                    .max(tag.0)
                    .max(payload_lo.0)
                    .max(payload_hi.0);
            }
            IrInstr::VariantLoadTag { dst, variant }
            | IrInstr::VariantLoadPayloadLo { dst, variant }
            | IrInstr::VariantLoadPayloadHi { dst, variant } => {
                max_id = max_id.max(dst.0).max(variant.0);
            }
            IrInstr::Call { dst, args, .. } => {
                if let Some(d) = dst {
                    max_id = max_id.max(d.0);
                }
                for a in args {
                    max_id = max_id.max(a.0);
                }
            }
            IrInstr::Ret { val } => max_id = max_id.max(val.0),
        }
    }
    let locals_count = max_id.saturating_add(1).saturating_sub(params_len);
    let locals = if locals_count > 0 {
        vec![(locals_count, ValType::I32)]
    } else {
        Vec::new()
    };
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    for ins in &f.body {
        match ins {
            IrInstr::IConst { dst, n, .. } => {
                insts.i32_const(*n as i32);
                insts.local_set(dst.0);
            }
            IrInstr::IStringConst { dst, s } => {
                let off = strs
                    .get(s)
                    .ok_or_else(|| anyhow::anyhow!("missing string offset for literal"))?;
                insts.i32_const(*off as i32);
                insts.local_set(dst.0);
            }
            IrInstr::IBin { dst, op, lhs, rhs } => {
                insts.local_get(lhs.0);
                insts.local_get(rhs.0);
                match op {
                    BinOpIR::Add => insts.i32_add(),
                    BinOpIR::Sub => insts.i32_sub(),
                    BinOpIR::Mul => insts.i32_mul(),
                    BinOpIR::Div => insts.i32_div_s(),
                    BinOpIR::Lt => insts.i32_lt_s(),
                    BinOpIR::Le => insts.i32_le_s(),
                    BinOpIR::Gt => insts.i32_gt_s(),
                    BinOpIR::Ge => insts.i32_ge_s(),
                    BinOpIR::Eq => insts.i32_eq(),
                    BinOpIR::Neq => insts.i32_ne(),
                    BinOpIR::And => insts.i32_and(),
                    BinOpIR::Or => insts.i32_or(),
                };
                insts.local_set(dst.0);
            }
            IrInstr::Guard {
                cond,
                trap,
                span,
                detail,
            } => {
                insts.local_get(cond.0);
                insts.i32_eqz();
                insts.if_(BlockType::Empty);
                emit_guard_trap(&mut insts, *trap, *span, detail.as_i32());
                insts.end();
            }
            IrInstr::ISelect {
                dst,
                cond,
                then_v,
                else_v,
            } => {
                // Structured if/else expression: push result on stack, then set dst
                insts.local_get(cond.0);
                // if (result i32) then_val else else_val
                insts.if_(wasm_encoder::BlockType::Result(ValType::I32));
                insts.local_get(then_v.0);
                insts.else_();
                insts.local_get(else_v.0);
                insts.end();
                insts.local_set(dst.0);
            }
            IrInstr::VariantInit {
                dst,
                tag,
                payload_lo,
                payload_hi,
            } => {
                // pointer = heap_ptr
                insts.global_get(HEAP_PTR_GLOBAL);
                insts.local_set(dst.0);

                // ensure allocation stays within memory before writing
                insts.local_get(dst.0);
                insts.i32_const(16);
                insts.i32_add();
                insts.i32_const(15);
                insts.i32_add();
                insts.i32_const(-16);
                insts.i32_and();
                insts.memory_size(0);
                insts.i32_const(65536);
                insts.i32_mul();
                insts.i32_gt_u();
                insts.if_(BlockType::Empty);
                emit_runtime_trap(
                    &mut insts,
                    TrapCode::AllocatorOom,
                    TrapOperand::local(dst.0),
                    TrapOperand::zero(),
                    0,
                );
                insts.end();

                // store fields
                insts.local_get(dst.0);
                insts.local_get(tag.0);
                insts.i32_store(MemArg {
                    align: 2,
                    offset: 0,
                    memory_index: 0,
                });

                insts.local_get(dst.0);
                insts.local_get(payload_lo.0);
                insts.i32_store(MemArg {
                    align: 2,
                    offset: 4,
                    memory_index: 0,
                });

                insts.local_get(dst.0);
                insts.local_get(payload_hi.0);
                insts.i32_store(MemArg {
                    align: 2,
                    offset: 8,
                    memory_index: 0,
                });

                insts.local_get(dst.0);
                insts.i32_const(0);
                insts.i32_store(MemArg {
                    align: 2,
                    offset: 12,
                    memory_index: 0,
                });

                // heap_ptr = align16(ptr + size)
                insts.local_get(dst.0);
                insts.i32_const(16);
                insts.i32_add();
                insts.i32_const(15);
                insts.i32_add();
                insts.i32_const(-16);
                insts.i32_and();
                insts.global_set(HEAP_PTR_GLOBAL);
            }
            IrInstr::VariantLoadTag { dst, variant } => {
                insts.local_get(variant.0);
                insts.i32_load(MemArg {
                    align: 2,
                    offset: 0,
                    memory_index: 0,
                });
                insts.local_set(dst.0);
            }
            IrInstr::VariantLoadPayloadLo { dst, variant } => {
                insts.local_get(variant.0);
                insts.i32_load(MemArg {
                    align: 2,
                    offset: 4,
                    memory_index: 0,
                });
                insts.local_set(dst.0);
            }
            IrInstr::VariantLoadPayloadHi { dst, variant } => {
                insts.local_get(variant.0);
                insts.i32_load(MemArg {
                    align: 2,
                    offset: 8,
                    memory_index: 0,
                });
                insts.local_set(dst.0);
            }
            IrInstr::Call { dst, callee, args } => {
                for a in args {
                    insts.local_get(a.0);
                }
                insts.call(*callee);
                if let Some(d) = dst {
                    insts.local_set(d.0);
                }
            }
            IrInstr::Ret { val } => {
                insts.local_get(val.0);
            }
        }
    }
    insts.end();
    Ok(fenc)
}
