use anyhow::Result;
use clg_ir::{
    BinOpIR, Function as IrFunction, Instr as IrInstr, IrType, Module as IrModule, TrapCode, Value,
};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, CustomSection, DataSection, EntityType, ExportKind,
    ExportSection, Function, FunctionSection, GlobalSection, GlobalType, ImportSection,
    InstructionSink, MemArg, MemorySection, MemoryType, Module, NameMap, NameSection, TypeSection,
    ValType,
};

use crate::intrinsics::{
    crypto::{
        encode_intrinsic_crypto_hash, encode_intrinsic_crypto_hmac, encode_intrinsic_crypto_verify,
    },
    env::{encode_intrinsic_env_random, encode_intrinsic_env_time},
    runtime::{emit_guard_trap, emit_runtime_trap, encode_intrinsic_identity, TrapOperand},
    strings::{
        encode_intrinsic_bytes_eq_ct, encode_intrinsic_str_concat, encode_intrinsic_str_eq,
        encode_intrinsic_str_len,
    },
    u64::{
        encode_intrinsic_u64_from_bytes_be, encode_intrinsic_u64_from_bytes_le,
        encode_intrinsic_u64_rotl, encode_intrinsic_u64_rotr, encode_intrinsic_u64_to_bytes_be,
        encode_intrinsic_u64_to_bytes_le,
    },
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

fn val_type_for_ir(ty: IrType) -> ValType {
    match ty {
        IrType::U8 => ValType::I32,
        IrType::U64 => ValType::I64,
        IrType::U128 | IrType::U256 => ValType::I32,
        IrType::Int | IrType::Bool => ValType::I32,
    }
}

fn val_type_key(ty: ValType) -> u8 {
    match ty {
        ValType::I32 => 0,
        ValType::I64 => 1,
        _ => 2,
    }
}

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
    let has_env_time = ir.funcs.iter().any(|f| f.name == "std::env::time");
    let has_env_random = ir.funcs.iter().any(|f| f.name == "std::env::random");
    let has_crypto_hash = ir.funcs.iter().any(|f| f.name == "std::crypto::hash");
    let has_crypto_hmac = ir.funcs.iter().any(|f| f.name == "std::crypto::hmac");
    let has_crypto_verify = ir.funcs.iter().any(|f| f.name == "std::crypto::verify");

    // Build function types with deduplication, and record each function's type index
    #[derive(Hash, Eq, PartialEq, Clone)]
    struct SigKey {
        params: Vec<u8>,
        results: Vec<u8>,
    }
    let mut types = TypeSection::new();
    let mut sig_to_tyidx = HashMap::<SigKey, u32>::with_capacity(ir.funcs.len());
    let mut fn_type_indices: Vec<u32> = Vec::with_capacity(ir.funcs.len());
    let mut type_index_for = |params: Vec<ValType>, results: Vec<ValType>| -> u32 {
        let key = SigKey {
            params: params.iter().map(|ty| val_type_key(*ty)).collect(),
            results: results.iter().map(|ty| val_type_key(*ty)).collect(),
        };
        if let Some(idx) = sig_to_tyidx.get(&key) {
            *idx
        } else {
            let idx = sig_to_tyidx.len() as u32;
            types.ty().function(params, results);
            sig_to_tyidx.insert(key, idx);
            idx
        }
    };
    for f in &ir.funcs {
        let params: Vec<ValType> = f.params.iter().copied().map(val_type_for_ir).collect();
        let results: Vec<ValType> = f
            .ret
            .map(|ty| vec![val_type_for_ir(ty)])
            .unwrap_or_default();
        let ty_idx = type_index_for(params, results);
        fn_type_indices.push(ty_idx);
    }
    let fd_write_ty = if has_wasi_print {
        Some(type_index_for(vec![ValType::I32; 4], vec![ValType::I32]))
    } else {
        None
    };
    let env_time_ty = if has_env_time {
        Some(type_index_for(Vec::new(), vec![ValType::I32]))
    } else {
        None
    };
    let env_random_ty = if has_env_random {
        Some(type_index_for(vec![ValType::I32], vec![ValType::I32]))
    } else {
        None
    };
    let crypto_hash_ty = if has_crypto_hash {
        Some(type_index_for(vec![ValType::I32, ValType::I32], vec![ValType::I32]))
    } else {
        None
    };
    let crypto_hmac_ty = if has_crypto_hmac {
        Some(type_index_for(vec![ValType::I32, ValType::I32, ValType::I32], vec![ValType::I32]))
    } else {
        None
    };
    let crypto_verify_ty = if has_crypto_verify {
        Some(type_index_for(
            vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::I32],
        ))
    } else {
        None
    };
    module.section(&types);

    let mut import_count = 0u32;
    let mut fd_write_index: Option<u32> = None;
    let mut env_time_index: Option<u32> = None;
    let mut env_random_index: Option<u32> = None;
    let mut crypto_hash_index: Option<u32> = None;
    let mut crypto_hmac_index: Option<u32> = None;
    let mut crypto_verify_index: Option<u32> = None;
    let mut imports = ImportSection::new();
    if let Some(fd_write_ty) = fd_write_ty {
        imports.import(
            "wasi_snapshot_preview1",
            "fd_write",
            EntityType::Function(fd_write_ty),
        );
        fd_write_index = Some(import_count);
        import_count += 1;
    }
    if let Some(env_time_ty) = env_time_ty {
        imports.import(
            "clearlang_env",
            "env_time",
            EntityType::Function(env_time_ty),
        );
        env_time_index = Some(import_count);
        import_count += 1;
    }
    if let Some(env_random_ty) = env_random_ty {
        imports.import(
            "clearlang_env",
            "env_random",
            EntityType::Function(env_random_ty),
        );
        env_random_index = Some(import_count);
        import_count += 1;
    }
    if let Some(crypto_hash_ty) = crypto_hash_ty {
        imports.import(
            "clearlang_crypto",
            "crypto_hash",
            EntityType::Function(crypto_hash_ty),
        );
        crypto_hash_index = Some(import_count);
        import_count += 1;
    }
    if let Some(crypto_hmac_ty) = crypto_hmac_ty {
        imports.import(
            "clearlang_crypto",
            "crypto_hmac",
            EntityType::Function(crypto_hmac_ty),
        );
        crypto_hmac_index = Some(import_count);
        import_count += 1;
    }
    if let Some(crypto_verify_ty) = crypto_verify_ty {
        imports.import(
            "clearlang_crypto",
            "crypto_verify",
            EntityType::Function(crypto_verify_ty),
        );
        crypto_verify_index = Some(import_count);
        import_count += 1;
    }
    if import_count > 0 {
        module.section(&imports);
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
        exports.export(
            alias.export.as_str(),
            ExportKind::Func,
            *idx + func_index_offset,
        );
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
            "std::bytes::eq_ct" => encode_intrinsic_bytes_eq_ct(f)?,
            "std::bytes::concat" => encode_intrinsic_str_concat(f)?,
            "std::bytes::from_string" => encode_intrinsic_identity(f)?,
            "std::bytes::to_string" => encode_intrinsic_identity(f)?,
            "std::wasi::print" => {
                let fd_write = fd_write_index.expect("fd_write import expected");
                encode_intrinsic_wasi_print(f, fd_write)?
            }
            "std::env::time" => {
                let idx = env_time_index.expect("env_time import expected");
                encode_intrinsic_env_time(f, idx)?
            }
            "std::env::random" => {
                let idx = env_random_index.expect("env_random import expected");
                encode_intrinsic_env_random(f, idx)?
            }
            "std::crypto::hash" => {
                let idx = crypto_hash_index.expect("crypto_hash import expected");
                encode_intrinsic_crypto_hash(f, idx)?
            }
            "std::crypto::hmac" => {
                let idx = crypto_hmac_index.expect("crypto_hmac import expected");
                encode_intrinsic_crypto_hmac(f, idx)?
            }
            "std::crypto::verify" => {
                let idx = crypto_verify_index.expect("crypto_verify import expected");
                encode_intrinsic_crypto_verify(f, idx)?
            }
            "std::str::len" => encode_intrinsic_str_len(f)?,
            "std::str::eq" => encode_intrinsic_str_eq(f)?,
            "std::str::concat" => encode_intrinsic_str_concat(f)?,
            "std::u64::rotl" => encode_intrinsic_u64_rotl(f)?,
            "std::u64::rotr" => encode_intrinsic_u64_rotr(f)?,
            "std::u64::to_bytes_le" => encode_intrinsic_u64_to_bytes_le(f)?,
            "std::u64::to_bytes_be" => encode_intrinsic_u64_to_bytes_be(f)?,
            "std::u64::from_bytes_le" => encode_intrinsic_u64_from_bytes_le(f)?,
            "std::u64::from_bytes_be" => encode_intrinsic_u64_from_bytes_be(f)?,
            _ => encode_ir_function(f, &ir.funcs, &str_pool, func_index_offset)?,
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

fn infer_value_types(f: &IrFunction, funcs: &[IrFunction], max_id: u32) -> Result<Vec<ValType>> {
    let mut types: Vec<Option<ValType>> = vec![None; max_id as usize + 1];

    for (i, ty) in f.params.iter().copied().enumerate() {
        types[i] = Some(val_type_for_ir(ty));
    }

    fn set_type(types: &mut [Option<ValType>], value: Value, ty: ValType) -> Result<()> {
        let slot = types
            .get_mut(value.0 as usize)
            .ok_or_else(|| anyhow::anyhow!("value {} out of range", value.0))?;
        if let Some(existing) = *slot {
            if existing != ty {
                return Err(anyhow::anyhow!(
                    "value {} has conflicting types {:?} vs {:?}",
                    value.0,
                    existing,
                    ty
                ));
            }
        } else {
            *slot = Some(ty);
        }
        Ok(())
    }

    for ins in &f.body {
        match ins {
            IrInstr::IConst { dst, ty, .. } => {
                set_type(&mut types, *dst, val_type_for_ir(*ty))?;
            }
            IrInstr::IStringConst { dst, .. } => {
                set_type(&mut types, *dst, ValType::I32)?;
            }
            IrInstr::Alloc { dst, .. } => {
                set_type(&mut types, *dst, ValType::I32)?;
            }
            IrInstr::Load { dst, ty, .. } => {
                set_type(&mut types, *dst, val_type_for_ir(*ty))?;
            }
            IrInstr::IBin { dst, op, ty, .. } => {
                let res_ty = match op {
                    BinOpIR::Add
                    | BinOpIR::Sub
                    | BinOpIR::Mul
                    | BinOpIR::Div
                    | BinOpIR::And
                    | BinOpIR::Or
                    | BinOpIR::Xor
                    | BinOpIR::Shl
                    | BinOpIR::Shr => val_type_for_ir(*ty),
                    BinOpIR::Lt
                    | BinOpIR::Le
                    | BinOpIR::Gt
                    | BinOpIR::Ge
                    | BinOpIR::Eq
                    | BinOpIR::Neq => ValType::I32,
                };
                set_type(&mut types, *dst, res_ty)?;
            }
            IrInstr::ISelect {
                dst,
                then_v,
                else_v,
                ..
            } => {
                let then_ty = types
                    .get(then_v.0 as usize)
                    .and_then(|ty| *ty)
                    .ok_or_else(|| anyhow::anyhow!("missing type for value {}", then_v.0))?;
                let else_ty = types
                    .get(else_v.0 as usize)
                    .and_then(|ty| *ty)
                    .ok_or_else(|| anyhow::anyhow!("missing type for value {}", else_v.0))?;
                if then_ty != else_ty {
                    return Err(anyhow::anyhow!(
                        "select type mismatch for value {} ({:?} vs {:?})",
                        dst.0,
                        then_ty,
                        else_ty
                    ));
                }
                set_type(&mut types, *dst, then_ty)?;
            }
            IrInstr::VariantInit { dst, .. } => {
                set_type(&mut types, *dst, ValType::I32)?;
            }
            IrInstr::U128Init { dst, .. } | IrInstr::U256Init { dst, .. } => {
                set_type(&mut types, *dst, ValType::I32)?;
            }
            IrInstr::VariantLoadTag { dst, .. }
            | IrInstr::VariantLoadPayloadLo { dst, .. }
            | IrInstr::VariantLoadPayloadHi { dst, .. } => {
                set_type(&mut types, *dst, ValType::I32)?;
            }
            IrInstr::U128LoadLimb { dst, .. } | IrInstr::U256LoadLimb { dst, .. } => {
                set_type(&mut types, *dst, ValType::I64)?;
            }
            IrInstr::Call { dst, callee, .. } => {
                if let Some(dst) = dst {
                    let ret_ty =
                        funcs
                            .get(*callee as usize)
                            .and_then(|f| f.ret)
                            .ok_or_else(|| {
                                anyhow::anyhow!("missing return type for call {}", callee)
                            })?;
                    set_type(&mut types, *dst, val_type_for_ir(ret_ty))?;
                }
            }
            IrInstr::Guard { .. }
            | IrInstr::ReturnIf { .. }
            | IrInstr::Store { .. }
            | IrInstr::BrIf { .. }
            | IrInstr::BrIfEqz { .. }
            | IrInstr::BlockBegin
            | IrInstr::BlockEnd
            | IrInstr::LoopBegin
            | IrInstr::LoopEnd
            | IrInstr::Br { .. }
            | IrInstr::Ret { .. } => {}
        }
    }

    let mut resolved = Vec::with_capacity(types.len());
    for (idx, ty) in types.into_iter().enumerate() {
        let ty = ty.ok_or_else(|| anyhow::anyhow!("missing type for value {}", idx))?;
        resolved.push(ty);
    }
    Ok(resolved)
}

fn encode_ir_function(
    f: &IrFunction,
    funcs: &[IrFunction],
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
            IrInstr::Alloc { dst, .. } => max_id = max_id.max(dst.0),
            IrInstr::Load { dst, ptr, .. } => max_id = max_id.max(dst.0).max(ptr.0),
            IrInstr::Store { ptr, src, .. } => max_id = max_id.max(ptr.0).max(src.0),
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
            IrInstr::U128Init {
                dst,
                limb_lo,
                limb_hi,
            } => {
                max_id = max_id.max(dst.0).max(limb_lo.0).max(limb_hi.0);
            }
            IrInstr::U128LoadLimb { dst, value, .. } => {
                max_id = max_id.max(dst.0).max(value.0);
            }
            IrInstr::U256Init {
                dst,
                limb0,
                limb1,
                limb2,
                limb3,
            } => {
                max_id = max_id
                    .max(dst.0)
                    .max(limb0.0)
                    .max(limb1.0)
                    .max(limb2.0)
                    .max(limb3.0);
            }
            IrInstr::U256LoadLimb { dst, value, .. } => {
                max_id = max_id.max(dst.0).max(value.0);
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
    let value_types = infer_value_types(f, funcs, max_id)?;
    let mut locals: Vec<(u32, ValType)> = Vec::new();
    if max_id.saturating_add(1) > params_len {
        for idx in params_len..=max_id {
            let ty = value_types[idx as usize];
            if let Some((count, last_ty)) = locals.last_mut() {
                if *last_ty == ty {
                    *count += 1;
                    continue;
                }
            }
            locals.push((1, ty));
        }
    }
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    emit_fuel_tick(&mut insts, FUNCTION_FUEL_COST);

    for ins in &f.body {
        match ins {
            IrInstr::IConst { dst, n, ty } => {
                match ty {
                    IrType::U64 => insts.i64_const(*n),
                    IrType::U8 | IrType::U128 | IrType::U256 | IrType::Int | IrType::Bool => {
                        insts.i32_const(*n as i32)
                    }
                };
                insts.local_set(dst.0);
            }
            IrInstr::IStringConst { dst, s } => {
                let off = strs
                    .get(s)
                    .ok_or_else(|| anyhow::anyhow!("missing string offset for literal"))?;
                insts.i32_const(*off as i32);
                insts.local_set(dst.0);
            }
            IrInstr::Alloc { dst, size, align } => {
                insts.global_get(HEAP_PTR_GLOBAL);
                insts.local_set(dst.0);

                if *align > 1 {
                    insts.local_get(dst.0);
                    insts.i32_const((*align as i32) - 1);
                    insts.i32_add();
                    insts.i32_const(-(*align as i32));
                    insts.i32_and();
                    insts.local_set(dst.0);
                }

                insts.local_get(dst.0);
                insts.i32_const(*size as i32);
                insts.i32_add();
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

                insts.local_get(dst.0);
                insts.i32_const(*size as i32);
                insts.i32_add();
                insts.global_set(HEAP_PTR_GLOBAL);
            }
            IrInstr::Load {
                dst,
                ptr,
                offset,
                ty,
            } => {
                insts.local_get(ptr.0);
                match ty {
                    IrType::U64 => insts.i64_load(MemArg {
                        align: 3,
                        offset: (*offset).into(),
                        memory_index: 0,
                    }),
                    IrType::U8 => insts.i32_load8_u(MemArg {
                        align: 0,
                        offset: (*offset).into(),
                        memory_index: 0,
                    }),
                    IrType::U128 | IrType::U256 | IrType::Int | IrType::Bool => {
                        insts.i32_load(MemArg {
                            align: 2,
                            offset: (*offset).into(),
                            memory_index: 0,
                        })
                    }
                };
                insts.local_set(dst.0);
            }
            IrInstr::Store {
                ptr,
                src,
                offset,
                ty,
            } => {
                insts.local_get(ptr.0);
                insts.local_get(src.0);
                match ty {
                    IrType::U64 => insts.i64_store(MemArg {
                        align: 3,
                        offset: (*offset).into(),
                        memory_index: 0,
                    }),
                    IrType::U8 => insts.i32_store8(MemArg {
                        align: 0,
                        offset: (*offset).into(),
                        memory_index: 0,
                    }),
                    IrType::U128 | IrType::U256 | IrType::Int | IrType::Bool => {
                        insts.i32_store(MemArg {
                            align: 2,
                            offset: (*offset).into(),
                            memory_index: 0,
                        })
                    }
                };
            }
            IrInstr::IBin {
                dst,
                op,
                lhs,
                rhs,
                ty,
            } => {
                if matches!(ty, IrType::U128 | IrType::U256) {
                    return Err(anyhow::anyhow!(
                        "U128/U256 operations are not supported in codegen"
                    ));
                }
                insts.local_get(lhs.0);
                insts.local_get(rhs.0);
                match op {
                    BinOpIR::Add => match ty {
                        IrType::U64 => insts.i64_add(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_add()
                        }
                    },
                    BinOpIR::Sub => match ty {
                        IrType::U64 => insts.i64_sub(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_sub()
                        }
                    },
                    BinOpIR::Mul => match ty {
                        IrType::U64 => insts.i64_mul(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_mul()
                        }
                    },
                    BinOpIR::Div => match ty {
                        IrType::U64 => insts.i64_div_u(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_div_s()
                        }
                    },
                    BinOpIR::Shl => match ty {
                        IrType::U64 => insts.i64_shl(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_shl()
                        }
                    },
                    BinOpIR::Shr => match ty {
                        IrType::U64 => insts.i64_shr_u(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_shr_s()
                        }
                    },
                    BinOpIR::Lt => match ty {
                        IrType::U64 => insts.i64_lt_u(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_lt_s()
                        }
                    },
                    BinOpIR::Le => match ty {
                        IrType::U64 => insts.i64_le_u(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_le_s()
                        }
                    },
                    BinOpIR::Gt => match ty {
                        IrType::U64 => insts.i64_gt_u(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_gt_s()
                        }
                    },
                    BinOpIR::Ge => match ty {
                        IrType::U64 => insts.i64_ge_u(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_ge_s()
                        }
                    },
                    BinOpIR::Eq => match ty {
                        IrType::U64 => insts.i64_eq(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_eq()
                        }
                    },
                    BinOpIR::Neq => match ty {
                        IrType::U64 => insts.i64_ne(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_ne()
                        }
                    },
                    BinOpIR::And => match ty {
                        IrType::U64 => insts.i64_and(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_and()
                        }
                    },
                    BinOpIR::Or => match ty {
                        IrType::U64 => insts.i64_or(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_or()
                        }
                    },
                    BinOpIR::Xor => match ty {
                        IrType::U64 => insts.i64_xor(),
                        IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                            insts.i32_xor()
                        }
                    },
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
                let dst_ty = value_types[dst.0 as usize];
                insts.local_get(cond.0);
                // if (result ty) then_val else else_val
                insts.if_(wasm_encoder::BlockType::Result(dst_ty));
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
            IrInstr::U128Init {
                dst,
                limb_lo,
                limb_hi,
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

                insts.local_get(dst.0);
                insts.local_get(limb_lo.0);
                insts.i64_store(MemArg {
                    align: 3,
                    offset: 0,
                    memory_index: 0,
                });
                insts.local_get(dst.0);
                insts.local_get(limb_hi.0);
                insts.i64_store(MemArg {
                    align: 3,
                    offset: 8,
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
            IrInstr::U128LoadLimb { dst, value, limb } => {
                let offset = match limb {
                    0 => 0,
                    1 => 8,
                    _ => return Err(anyhow::anyhow!("invalid U128 limb {}", limb)),
                };
                insts.local_get(value.0);
                insts.i64_load(MemArg {
                    align: 3,
                    offset,
                    memory_index: 0,
                });
                insts.local_set(dst.0);
            }
            IrInstr::U256Init {
                dst,
                limb0,
                limb1,
                limb2,
                limb3,
            } => {
                // pointer = heap_ptr
                insts.global_get(HEAP_PTR_GLOBAL);
                insts.local_set(dst.0);

                // ensure allocation stays within memory before writing
                insts.local_get(dst.0);
                insts.i32_const(32);
                insts.i32_add();
                insts.i32_const(31);
                insts.i32_add();
                insts.i32_const(-32);
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

                insts.local_get(dst.0);
                insts.local_get(limb0.0);
                insts.i64_store(MemArg {
                    align: 3,
                    offset: 0,
                    memory_index: 0,
                });
                insts.local_get(dst.0);
                insts.local_get(limb1.0);
                insts.i64_store(MemArg {
                    align: 3,
                    offset: 8,
                    memory_index: 0,
                });
                insts.local_get(dst.0);
                insts.local_get(limb2.0);
                insts.i64_store(MemArg {
                    align: 3,
                    offset: 16,
                    memory_index: 0,
                });
                insts.local_get(dst.0);
                insts.local_get(limb3.0);
                insts.i64_store(MemArg {
                    align: 3,
                    offset: 24,
                    memory_index: 0,
                });

                // heap_ptr = align32(ptr + size)
                insts.local_get(dst.0);
                insts.i32_const(32);
                insts.i32_add();
                insts.i32_const(31);
                insts.i32_add();
                insts.i32_const(-32);
                insts.i32_and();
                insts.global_set(HEAP_PTR_GLOBAL);
            }
            IrInstr::U256LoadLimb { dst, value, limb } => {
                let offset = match limb {
                    0 => 0,
                    1 => 8,
                    2 => 16,
                    3 => 24,
                    _ => return Err(anyhow::anyhow!("invalid U256 limb {}", limb)),
                };
                insts.local_get(value.0);
                insts.i64_load(MemArg {
                    align: 3,
                    offset,
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
