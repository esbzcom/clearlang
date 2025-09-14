use anyhow::Result;
use lumi_ir::{BinOpIR, Function as IrFunction, Instr as IrInstr, Module as IrModule};
use std::collections::HashMap;
use wasm_encoder::{
    CodeSection, DataSection, ExportKind, ExportSection, Function, FunctionSection, GlobalSection, GlobalType, MemorySection, MemoryType, Module, NameMap, NameSection, TypeSection, ValType, ConstExpr,
};

use crate::intrinsics::strings::{encode_intrinsic_str_concat, encode_intrinsic_str_eq, encode_intrinsic_str_len};

pub struct CodegenOpts {
    pub debug_names: bool,
}

impl Default for CodegenOpts {
    fn default() -> Self {
        CodegenOpts { debug_names: false }
    }
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
                    let len = s.as_bytes().len() as u32;
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
        let key = SigKey { params: f.params.len(), has_ret: f.ret.is_some() };
        let ty_idx = if let Some(idx) = sig_to_tyidx.get(&key) {
            *idx
        } else {
            let params: Vec<ValType> = f.params.iter().map(|_| ValType::I32).collect();
            let results: Vec<ValType> = match f.ret { Some(_) => vec![ValType::I32], None => vec![] };
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

    // Memory section (prepare for string runtime); 1 page minimum
    let mut memories = MemorySection::new();
    memories.memory(MemoryType { minimum: 1, maximum: None, memory64: false, shared: false, page_size_log2: None });
    module.section(&memories);

    // Global bump allocator pointer initialized after string data
    let heap_start = (cur_off + 3) & !3;
    let mut globals = GlobalSection::new();
    // global 0: (mut i32) heap_ptr
    globals.global(GlobalType { val_type: ValType::I32, mutable: true, shared: false }, &ConstExpr::i32_const(heap_start as i32));
    module.section(&globals);

    // Export main if present
    if let Some((i, _)) = ir.funcs.iter().enumerate().find(|(_, f)| f.name == "main") {
        let mut exports = ExportSection::new();
        exports.export("main", ExportKind::Func, i as u32);
        module.section(&exports);
    }

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
            IrInstr::Call { dst, args, .. } => {
                if let Some(d) = dst { max_id = max_id.max(d.0); }
                for a in args { max_id = max_id.max(a.0); }
            }
            IrInstr::Ret { val } => max_id = max_id.max(val.0),
        }
    }
    let locals_count = max_id.saturating_add(1).saturating_sub(params_len);
    let locals = if locals_count > 0 { vec![(locals_count, ValType::I32)] } else { Vec::new() };
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();

    for ins in &f.body {
        match ins {
            IrInstr::IConst { dst, n, .. } => {
                insts.i32_const(*n as i32);
                insts.local_set(dst.0);
            }
            IrInstr::IStringConst { dst, s } => {
                let off = strs.get(s)
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
                };
                insts.local_set(dst.0);
            }
            IrInstr::Call { dst, callee, args } => {
                for a in args { insts.local_get(a.0); }
                insts.call(*callee);
                if let Some(d) = dst { insts.local_set(d.0); }
            }
            IrInstr::Ret { val } => {
                insts.local_get(val.0);
            }
        }
    }
    insts.end();
    Ok(fenc)
}
