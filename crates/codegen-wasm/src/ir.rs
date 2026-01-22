use anyhow::Result;
use clg_ir::{BinOpIR, Function as IrFunction, Instr as IrInstr, Module as IrModule, TrapCode};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, CustomSection, DataSection, EntityType, ExportKind,
    ExportSection, Function, FunctionSection, GlobalSection, GlobalType, ImportSection,
    InstructionSink, MemArg, MemorySection, MemoryType, Module, NameMap, NameSection, TypeSection,
    ValType,
};

use crate::intrinsics::{
    runtime::{emit_guard_trap, emit_runtime_trap, encode_intrinsic_identity, TrapOperand},
    strings::{encode_intrinsic_str_concat, encode_intrinsic_str_eq, encode_intrinsic_str_len},
    wasi::encode_intrinsic_wasi_print,
};

pub(crate) const HEAP_PTR_GLOBAL: u32 = 0;
pub(crate) const ERROR_CODE_GLOBAL: u32 = 1;
pub(crate) const ERROR_START_GLOBAL: u32 = 2;
pub(crate) const ERROR_END_GLOBAL: u32 = 3;
pub(crate) const ERROR_DETAIL_GLOBAL: u32 = 4;
pub(crate) const FUEL_GLOBAL: u32 = 5;
const DEFAULT_FUEL_LIMIT: i32 = 1_000_000;
const FUNCTION_FUEL_COST: i32 = 1_000;
const LOOP_FUEL_COST: i32 = 1;

#[derive(Default)]
pub struct CodegenOpts {
    pub debug_names: bool,
    pub proof_section: Option<Vec<u8>>,
    pub export_aliases: Vec<ExportAlias>,
}

#[derive(Clone, Debug)]
pub struct ExportAlias {
    pub export: String,
    pub target: String,
}

pub fn emit_from_ir_with_opts(ir: &IrModule, opts: CodegenOpts) -> Result<Vec<u8>> {
    let mut module = Module::new();

    // String pool: collect unique string literals and assign memory offsets.
    // Avoid a pre-scan pass to keep codegen hot paths to a single walk.
    let mut str_pool: HashMap<String, u32> = HashMap::new();
    let mut cur_off: u32 = 0;
    for f in &ir.funcs {
        for ins in &f.body {
            if let IrInstr::IStringConst { s, .. } = ins {
                if let Entry::Vacant(entry) = str_pool.entry(s.clone()) {
                    let len = s.len() as u32;
                    let off = cur_off;
                    let size = 4 + len; // header + bytes
                    let next = (off + size + 3) & !3; // 4-byte align
                    entry.insert(off);
                    cur_off = next;
                }
            }
        }
    }

    let has_wasi_print = ir.funcs.iter().any(|f| f.name == "std::wasi::print");

    // Build function types with deduplication, and record each function's type index
    #[derive(Hash, Eq, PartialEq, Clone)]
    struct SigKey {
        params: usize,
        has_ret: bool,
    }
    let mut types = TypeSection::new();
    let mut sig_to_tyidx = HashMap::<SigKey, u32>::with_capacity(ir.funcs.len());
    let mut fn_type_indices: Vec<u32> = Vec::with_capacity(ir.funcs.len());
    let mut type_index_for = |params: usize, has_ret: bool| -> u32 {
        let key = SigKey { params, has_ret };
        if let Some(idx) = sig_to_tyidx.get(&key) {
            *idx
        } else {
            let params: Vec<ValType> = (0..params).map(|_| ValType::I32).collect();
            let results: Vec<ValType> = if has_ret { vec![ValType::I32] } else { vec![] };
            let idx = sig_to_tyidx.len() as u32;
            types.ty().function(params, results);
            sig_to_tyidx.insert(key, idx);
            idx
        }
    };
    for f in &ir.funcs {
        let ty_idx = type_index_for(f.params.len(), f.ret.is_some());
        fn_type_indices.push(ty_idx);
    }
    let fd_write_ty = if has_wasi_print {
        Some(type_index_for(4, true))
    } else {
        None
    };
    module.section(&types);

    let mut import_count = 0u32;
    let mut fd_write_index: Option<u32> = None;
    if let Some(fd_write_ty) = fd_write_ty {
        let mut imports = ImportSection::new();
        imports.import(
            "wasi_snapshot_preview1",
            "fd_write",
            EntityType::Function(fd_write_ty),
        );
        module.section(&imports);
        fd_write_index = Some(0);
        import_count = 1;
    }

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
    // global 5: remaining fuel for loop/recursion metering
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(DEFAULT_FUEL_LIMIT),
    );
    module.section(&globals);

    let func_index_offset = import_count;

    // Export entrypoints + runtime bookkeeping globals
    let mut exports = ExportSection::new();
    let mut export_names: HashMap<&str, ()> = HashMap::new();
    let mut fn_indices: HashMap<&str, u32> = HashMap::with_capacity(ir.funcs.len());
    for (i, f) in ir.funcs.iter().enumerate() {
        fn_indices.insert(f.name.as_str(), i as u32);
    }
    if let Some(idx) = fn_indices.get("main") {
        exports.export("main", ExportKind::Func, *idx + func_index_offset);
        export_names.insert("main", ());
    }
    for alias in &opts.export_aliases {
        if export_names.contains_key(alias.export.as_str()) {
            continue;
        }
        let Some(idx) = fn_indices.get(alias.target.as_str()) else {
            return Err(anyhow::anyhow!(
                "missing function `{}` for export `{}`",
                alias.target,
                alias.export
            ));
        };
        exports.export(alias.export.as_str(), ExportKind::Func, *idx + func_index_offset);
        export_names.insert(alias.export.as_str(), ());
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
    exports.export("__clg_fuel_remaining", ExportKind::Global, FUEL_GLOBAL);
    module.section(&exports);

    // Code section: encode each function body
    let mut codes = CodeSection::new();
    for f in &ir.funcs {
        // Encode intrinsics with custom bodies; other functions from IR
        let func = match f.name.as_str() {
            "std::bytes::len" => encode_intrinsic_str_len(f)?,
            "std::bytes::eq" => encode_intrinsic_str_eq(f)?,
            "std::bytes::concat" => encode_intrinsic_str_concat(f)?,
            "std::bytes::from_string" => encode_intrinsic_identity(f)?,
            "std::bytes::to_string" => encode_intrinsic_identity(f)?,
            "std::wasi::print" => {
                let fd_write = fd_write_index.expect("fd_write import expected");
                encode_intrinsic_wasi_print(f, fd_write)?
            }
            "std::str::len" => encode_intrinsic_str_len(f)?,
            "std::str::eq" => encode_intrinsic_str_eq(f)?,
            "std::str::concat" => encode_intrinsic_str_concat(f)?,
            _ => encode_ir_function(f, &str_pool, func_index_offset)?,
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
            fn_names.append(i as u32 + func_index_offset, &f.name);
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

fn emit_fuel_tick(insts: &mut InstructionSink<'_>, cost: i32) {
    insts.global_get(FUEL_GLOBAL);
    insts.i32_const(cost);
    insts.i32_sub();
    insts.global_set(FUEL_GLOBAL);

    insts.global_get(FUEL_GLOBAL);
    insts.i32_const(0);
    insts.i32_le_s();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        insts,
        TrapCode::LimitsExceeded,
        TrapOperand::zero(),
        TrapOperand::zero(),
        0,
    );
    insts.end();
}

fn encode_ir_function(
    f: &IrFunction,
    strs: &HashMap<String, u32>,
    func_index_offset: u32,
) -> Result<Function> {
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
            IrInstr::VariantLoadTag { dst, variant, .. }
            | IrInstr::VariantLoadPayloadLo { dst, variant }
            | IrInstr::VariantLoadPayloadHi { dst, variant } => {
                max_id = max_id.max(dst.0).max(variant.0);
            }
            IrInstr::ReturnIf { cond, ret } => {
                max_id = max_id.max(cond.0).max(ret.0);
            }
            IrInstr::Call { dst, args, .. } => {
                if let Some(d) = dst {
                    max_id = max_id.max(d.0);
                }
                for a in args {
                    max_id = max_id.max(a.0);
                }
            }
            IrInstr::BrIf { cond, .. } | IrInstr::BrIfEqz { cond, .. } => {
                max_id = max_id.max(cond.0);
            }
            IrInstr::BlockBegin
            | IrInstr::BlockEnd
            | IrInstr::LoopBegin
            | IrInstr::LoopEnd
            | IrInstr::Br { .. } => {}
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

    emit_fuel_tick(&mut insts, FUNCTION_FUEL_COST);

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
            IrInstr::VariantLoadTag { dst, variant, kind } => {
                insts.local_get(variant.0);
                insts.i32_load(MemArg {
                    align: 2,
                    offset: 0,
                    memory_index: 0,
                });
                insts.local_set(dst.0);

                insts.local_get(dst.0);
                insts.i32_const(2);
                insts.i32_ge_u();
                insts.if_(BlockType::Empty);
                emit_runtime_trap(
                    &mut insts,
                    TrapCode::InvalidVariantTag,
                    TrapOperand::local(dst.0),
                    TrapOperand::zero(),
                    kind.as_i32(),
                );
                insts.end();
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
            IrInstr::ReturnIf { cond, ret } => {
                insts.local_get(cond.0);
                insts.if_(BlockType::Empty);
                insts.local_get(ret.0);
                insts.return_();
                insts.end();
            }
            IrInstr::Call { dst, callee, args } => {
                for a in args {
                    insts.local_get(a.0);
                }
                insts.call(*callee + func_index_offset);
                if let Some(d) = dst {
                    insts.local_set(d.0);
                }
            }
            IrInstr::BlockBegin => {
                insts.block(BlockType::Empty);
            }
            IrInstr::BlockEnd => {
                insts.end();
            }
            IrInstr::LoopBegin => {
                insts.loop_(BlockType::Empty);
                emit_fuel_tick(&mut insts, LOOP_FUEL_COST);
            }
            IrInstr::LoopEnd => {
                insts.end();
            }
            IrInstr::Br { depth } => {
                insts.br(*depth);
            }
            IrInstr::BrIf { cond, depth } => {
                insts.local_get(cond.0);
                insts.br_if(*depth);
            }
            IrInstr::BrIfEqz { cond, depth } => {
                insts.local_get(cond.0);
                insts.i32_eqz();
                insts.br_if(*depth);
            }
            IrInstr::Ret { val } => {
                insts.local_get(val.0);
            }
        }
    }
    insts.end();
    Ok(fenc)
}
