use anyhow::Result;
use clg_ir::{Instr as IrInstr, IrType, Module as IrModule};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use wasm_encoder::{
    CodeSection, ConstExpr, CustomSection, DataSection, EntityType, ExportKind, ExportSection,
    FunctionSection, GlobalSection, GlobalType, ImportSection, MemorySection, MemoryType, Module,
    NameMap, NameSection, TypeSection, ValType,
};

use crate::intrinsics::{
    crypto::{
        encode_intrinsic_crypto_hash, encode_intrinsic_crypto_hmac, encode_intrinsic_crypto_verify,
    },
    env::{encode_intrinsic_env_random, encode_intrinsic_env_time},
    runtime::encode_intrinsic_identity,
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

mod encode;
mod fuel;
mod infer;

use encode::encode_ir_function;

type StringPool<'a> = HashMap<&'a str, u32>;

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
    pub external_imports: Vec<ExternalImport>,
    pub std_core_link_mode: StdCoreLinkMode,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StdCoreLinkMode {
    #[default]
    Intrinsic,
    Precompiled,
}

#[derive(Clone, Debug)]
pub struct ExportAlias {
    pub export: String,
    pub target: String,
}

#[derive(Clone, Debug)]
pub struct ExternalImport {
    pub function: String,
    pub import_module: String,
    pub import_name: String,
}

#[derive(Default)]
struct IntrinsicPresence {
    has_wasi_print: bool,
    has_env_time: bool,
    has_env_random: bool,
    has_crypto_hash: bool,
    has_crypto_hmac: bool,
    has_crypto_verify: bool,
}

impl IntrinsicPresence {
    fn observe(&mut self, name: &str) {
        match name {
            "std::wasi::print" => self.has_wasi_print = true,
            "std::env::time" => self.has_env_time = true,
            "std::env::random" => self.has_env_random = true,
            "std::crypto::hash" => self.has_crypto_hash = true,
            "std::crypto::hmac" => self.has_crypto_hmac = true,
            "std::crypto::verify" => self.has_crypto_verify = true,
            _ => {}
        }
    }
}

pub fn emit_from_ir_with_opts(ir: &IrModule, opts: CodegenOpts) -> Result<Vec<u8>> {
    let mut module = Module::new();
    let mut external_by_function: HashMap<&str, &ExternalImport> =
        HashMap::with_capacity(opts.external_imports.len());
    for binding in &opts.external_imports {
        if external_by_function
            .insert(binding.function.as_str(), binding)
            .is_some()
        {
            anyhow::bail!(
                "duplicate external import binding for `{}`",
                binding.function
            );
        }
    }

    // Single pass over module-level metadata:
    // - collect unique string literals and assign memory offsets
    // - detect intrinsic imports required by this module
    let mut str_pool: StringPool<'_> = HashMap::new();
    let mut intrinsic_presence = IntrinsicPresence::default();
    let mut cur_off: u32 = 0;
    for f in &ir.funcs {
        intrinsic_presence.observe(f.name.as_str());
        for ins in &f.body {
            if let IrInstr::IStringConst { s, .. } = ins {
                if let Entry::Vacant(entry) = str_pool.entry(s.as_str()) {
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

    let has_wasi_print = intrinsic_presence.has_wasi_print;
    let has_env_time = intrinsic_presence.has_env_time;
    let has_env_random = intrinsic_presence.has_env_random;
    let has_crypto_hash = intrinsic_presence.has_crypto_hash;
    let has_crypto_hmac = intrinsic_presence.has_crypto_hmac;
    let has_crypto_verify = intrinsic_presence.has_crypto_verify;

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
        Some(type_index_for(
            vec![ValType::I32, ValType::I32],
            vec![ValType::I32],
        ))
    } else {
        None
    };
    let crypto_hmac_ty = if has_crypto_hmac {
        Some(type_index_for(
            vec![ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::I32],
        ))
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
    let mut used_external_imports: Vec<(&ExternalImport, u32)> = Vec::new();
    let mut seen_external_functions: HashSet<&str> = HashSet::new();
    for (i, f) in ir.funcs.iter().enumerate() {
        if let Some(binding) = external_by_function.get(f.name.as_str()) {
            if seen_external_functions.insert(f.name.as_str()) {
                used_external_imports.push((*binding, fn_type_indices[i]));
            }
        }
    }
    module.section(&types);

    let mut import_count = 0u32;
    let mut fd_write_index: Option<u32> = None;
    let mut env_time_index: Option<u32> = None;
    let mut env_random_index: Option<u32> = None;
    let mut crypto_hash_index: Option<u32> = None;
    let mut crypto_hmac_index: Option<u32> = None;
    let mut crypto_verify_index: Option<u32> = None;
    let mut external_import_indices: HashMap<String, u32> =
        HashMap::with_capacity(used_external_imports.len());
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
    for (binding, ty_idx) in &used_external_imports {
        imports.import(
            binding.import_module.as_str(),
            binding.import_name.as_str(),
            EntityType::Function(*ty_idx),
        );
        external_import_indices.insert(binding.function.clone(), import_count);
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
    let heap_start = ((cur_off + 15) & !15).max(16); // reserve 0 as invalid; 16-byte aligned
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
        if opts.std_core_link_mode == StdCoreLinkMode::Precompiled
            && is_precompiled_std_core_locked_symbol(f.name.as_str())
        {
            if let Some(idx) = external_import_indices.get(f.name.as_str()) {
                let func = encode_external_import_forwarder(f, *idx)?;
                codes.function(&func);
                continue;
            }
        }
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
            name if external_import_indices.contains_key(name) => {
                let idx = *external_import_indices
                    .get(name)
                    .expect("external import index expected");
                encode_external_import_forwarder(f, idx)?
            }
            _ => encode_ir_function(f, &ir.funcs, &str_pool, func_index_offset)?,
        };
        codes.function(&func);
    }
    module.section(&codes);

    // Data section for string literals (after Code as per section order)
    if !str_pool.is_empty() {
        let mut data = DataSection::new();
        // Sort by offset for deterministic emission
        let mut items: Vec<(u32, &str)> = str_pool.iter().map(|(s, off)| (*off, *s)).collect();
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

fn encode_external_import_forwarder(
    f: &clg_ir::Function,
    import_index: u32,
) -> Result<wasm_encoder::Function> {
    let mut fenc = wasm_encoder::Function::new(Vec::new());
    let mut insts = fenc.instructions();
    for idx in 0..f.params.len() {
        insts.local_get(idx as u32);
    }
    insts.call(import_index);
    insts.end();
    Ok(fenc)
}

fn is_precompiled_std_core_locked_symbol(name: &str) -> bool {
    matches!(name, "std::str::len" | "std::bytes::len")
}

// Backwards-compatible helper with default options
pub fn emit_from_ir(ir: &IrModule) -> Result<Vec<u8>> {
    emit_from_ir_with_opts(ir, CodegenOpts::default())
}
