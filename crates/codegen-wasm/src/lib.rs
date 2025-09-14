use anyhow::{bail, Context, Result};
use lumi_ast::{Expr, Func, Program, Type};
use std::collections::HashMap;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection,
    Module, TypeSection, ValType, NameSection, NameMap, MemorySection, MemoryType, MemArg,
    DataSection, ConstExpr, GlobalSection, GlobalType, BlockType,
};
use lumi_ir::{BinOpIR, Function as IrFunction, Instr as IrInstr, Module as IrModule};

/// Emit a minimal Wasm module exporting `main() -> i32` that returns 42.
pub fn emit_trivial_main() -> Result<Vec<u8>> {
    let mut module = Module::new();

    // Type section: (func (result i32)) -> type index 0
    let mut types = TypeSection::new();
    let params: Vec<ValType> = vec![];           // no params
    let results: Vec<ValType> = vec![ValType::I32];
    types.ty().function(params, results);
    module.section(&types);

    // Function section: one function of type 0
    let mut functions = FunctionSection::new();
    let type_index = 0;
    functions.function(type_index);
    module.section(&functions);

    // Export section: export func 0 as "main"
    let mut exports = ExportSection::new();
    exports.export("main", ExportKind::Func, 0);
    module.section(&exports);

    // Code section: body of func 0 → i32.const 42; end
    let mut codes = CodeSection::new();
    let locals: Vec<(u32, ValType)> = vec![];    // no locals
    let mut f = Function::new(locals);
    f.instructions()
        .i32_const(42)
        .end();
    codes.function(&f);
    module.section(&codes);

    Ok(module.finish())
}

/// Emit a Wasm module from a minimal subset of the Lumi AST.
/// Currently supports: a `main() -> Int` function whose body is an integer literal.
pub fn emit_from_ast(ast: &Program) -> Result<Vec<u8>> {
    // Constant-evaluate the program to an i32 result for main().
    let value_i32 = const_eval_main_int(ast)?;

    let mut module = Module::new();

    let mut types = TypeSection::new();
    types.ty().function(Vec::<ValType>::new(), vec![ValType::I32]);
    module.section(&types);

    let mut functions = FunctionSection::new();
    functions.function(0);
    module.section(&functions);

    let mut exports = ExportSection::new();
    exports.export("main", ExportKind::Func, 0);
    module.section(&exports);

    let mut codes = CodeSection::new();
    let mut f = Function::new(Vec::<(u32, ValType)>::new());
    f.instructions().i32_const(value_i32).end();
    codes.function(&f);
    module.section(&codes);

    Ok(module.finish())
}

fn const_eval_main_int(ast: &Program) -> Result<i32> {
    // Index functions by name
    let mut funs: HashMap<&str, &Func> = HashMap::new();
    for f in &ast.funcs {
        funs.insert(&f.name, f);
    }

    let main = funs.get("main").copied().context("missing `main` function")?;
    if !main.params.is_empty() {
        bail!("only zero-arg main supported in this phase");
    }
    if main.ret != Type::Int {
        bail!("only `main() -> Int` is supported in this phase");
    }

    let val = eval_expr_int(&main.body, &funs, &HashMap::new(), 0)?;
    let out = i32::try_from(val).context("result out of i32 range")?;
    Ok(out)
}

fn eval_expr_int(
    e: &Expr,
    funs: &HashMap<&str, &Func>,
    env: &HashMap<&str, i64>,
    depth: usize,
) -> Result<i64> {
    if depth > 256 {
        bail!("call depth limit exceeded");
    }
    match e {
        Expr::Int(n, _) => Ok(*n),
        Expr::Bool(_, _) => bail!("bool not supported in arithmetic evaluator"),
        Expr::String(_, _) => bail!("string not supported in arithmetic evaluator"),
        Expr::Match { .. } => bail!("match not supported in arithmetic evaluator"),
        Expr::If { .. } => bail!("if not supported in arithmetic evaluator"),
        Expr::Return { expr, .. } => eval_expr_int(expr, funs, env, depth + 1),
        Expr::Var(name, _) => env
            .get(name.as_str())
            .copied()
            .ok_or_else(|| anyhow::anyhow!("unbound variable `{}`", name)),
        Expr::Bin { op, lhs, rhs, .. } => {
            let l = eval_expr_int(lhs, funs, env, depth + 1)?;
            let r = eval_expr_int(rhs, funs, env, depth + 1)?;
            use lumi_ast::BinOp::*;
            let v = match op {
                Add => l + r,
                Sub => l - r,
                Mul => l * r,
                Div => {
                    if r == 0 {
                        bail!("division by zero")
                    } else {
                        l / r
                    }
                }
            };
            Ok(v)
        }
        Expr::Call { callee, args, .. } => {
            let f = funs
                .get(callee.as_str())
                .copied()
                .ok_or_else(|| anyhow::anyhow!("unknown function `{}`", callee))?;
            if f.ret != Type::Int {
                bail!("only Int-returning functions are supported")
            }
            if f.params.len() != args.len() {
                bail!("arity mismatch calling `{}`", callee)
            }
            // Evaluate arguments
            let mut new_env: HashMap<&str, i64> = HashMap::with_capacity(args.len());
            for (param, arg_e) in f.params.iter().zip(args.iter()) {
                if param.ty != Type::Int {
                    bail!("only Int parameters supported")
                }
                let v = eval_expr_int(arg_e, funs, env, depth + 1)?;
                new_env.insert(param.name.as_str(), v);
            }
            eval_expr_int(&f.body, funs, &new_env, depth + 1)
        }
    }
}

// ---------------- IR → Wasm (Phase 3.5 input, Phase 4 encoding) ----------------

/// Emit Wasm from IR (supports IConst, IBin, Call, Ret; Int/Bool as i32)
pub struct CodegenOpts {
    pub debug_names: bool,
}

impl Default for CodegenOpts {
    fn default() -> Self { CodegenOpts { debug_names: false } }
}

pub fn emit_from_ir_with_opts(ir: &IrModule, opts: CodegenOpts) -> Result<Vec<u8>> {
    let mut module = Module::new();

    // String pool: collect unique string literals and assign memory offsets
    let mut str_pool: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut cur_off: u32 = 0;
    for f in &ir.funcs {
        for ins in &f.body {
            if let IrInstr::IStringConst { s, .. } = ins {
                if !str_pool.contains_key(s) {
                    let len = s.as_bytes().len() as u32;
                    let off = cur_off;
                    // size = 4(header) + len; align next to 4 bytes
                    let size = 4 + len;
                    let next = (off + size + 3) & !3;
                    str_pool.insert(s.clone(), off);
                    cur_off = next;
                }
            }
        }
    }

    // Build function types with deduplication, and record each function's type index
    #[derive(Hash, Eq, PartialEq, Clone)]
    struct SigKey { params: usize, has_ret: bool }
    let mut types = TypeSection::new();
    let mut sig_to_tyidx = std::collections::HashMap::<SigKey, u32>::new();
    let mut fn_type_indices: Vec<u32> = Vec::with_capacity(ir.funcs.len());
    for f in &ir.funcs {
        let key = SigKey { params: f.params.len(), has_ret: f.ret.is_some() };
        let ty_idx = if let Some(idx) = sig_to_tyidx.get(&key) { *idx } else {
            let params: Vec<ValType> = f.params.iter().map(|_| ValType::I32).collect();
            let results: Vec<ValType> = match f.ret { Some(_) => vec![ValType::I32], None => vec![] };
            let idx = sig_to_tyidx.len() as u32; // next index
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
    memories.memory(MemoryType {
        minimum: 1,
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
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

    // (Data section for string literals will be appended after Code)

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

fn encode_ir_function(f: &IrFunction, strs: &std::collections::HashMap<String, u32>) -> Result<Function> {
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
                    .ok_or_else(|| anyhow::anyhow!(format!("missing string offset for literal")))?;
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

fn encode_intrinsic_str_len(_f: &IrFunction) -> Result<Function> {
    // Expect exactly one param (i32 ptr) and i32 return
    let locals: Vec<(u32, ValType)> = Vec::new();
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();
    // load len := i32.load align=4 offset=0 from local 0 pointer
    insts.local_get(0);
    insts.i32_load(MemArg { align: 2, offset: 0, memory_index: 0 });
    insts.end();
    Ok(fenc)
}

fn encode_intrinsic_str_eq(_f: &IrFunction) -> Result<Function> {
    // Params: a: i32, b: i32; Return: i32 (0/1)
    // Locals: len_a (2), len_b (3), pa (4), pb (5), i (6), res (7)
    let locals: Vec<(u32, ValType)> = vec![(6, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();
    // len_a = load32(a); len_b = load32(b)
    insts.local_get(0); insts.i32_load(MemArg { align: 2, offset: 0, memory_index: 0 }); insts.local_set(2);
    insts.local_get(1); insts.i32_load(MemArg { align: 2, offset: 0, memory_index: 0 }); insts.local_set(3);
    // res = 1 by default
    insts.i32_const(1); insts.local_set(7);
    // block { if (len_a != len_b) { res = 0; br 0 } ...compare... }
    insts.block(BlockType::Empty);
      insts.local_get(2); insts.local_get(3); insts.i32_ne();
      insts.if_(BlockType::Empty);
        insts.i32_const(0); insts.local_set(7);
        insts.br(0);
      insts.end();
      // pa = a + 4; pb = b + 4; i = 0
      insts.local_get(0); insts.i32_const(4); insts.i32_add(); insts.local_set(4);
      insts.local_get(1); insts.i32_const(4); insts.i32_add(); insts.local_set(5);
      insts.i32_const(0); insts.local_set(6);
      // loop { if (i >= len_a) break; if (pa[i] != pb[i]) { res=0; break; } i++; }
      insts.loop_(BlockType::Empty);
        insts.local_get(6); insts.local_get(2); insts.i32_ge_u(); insts.br_if(1);
        insts.local_get(4); insts.local_get(6); insts.i32_add();
        insts.i32_load8_u(MemArg { align: 0, offset: 0, memory_index: 0 });
        insts.local_get(5); insts.local_get(6); insts.i32_add();
        insts.i32_load8_u(MemArg { align: 0, offset: 0, memory_index: 0 });
        insts.i32_ne();
        insts.if_(BlockType::Empty);
          insts.i32_const(0); insts.local_set(7);
          insts.br(1);
        insts.end();
        insts.local_get(6); insts.i32_const(1); insts.i32_add(); insts.local_set(6);
        insts.br(0);
      insts.end();
    insts.end(); // end block
    // return res
    insts.local_get(7);
    insts.end();
    Ok(fenc)
}

fn encode_intrinsic_str_concat(_f: &IrFunction) -> Result<Function> {
    // Params: a: i32, b: i32; Return: i32 (ptr)
    // Locals (indices 2..8): len_a(2), len_b(3), total(4), pa(5), pb(6), dest(7), i(8)
    let locals: Vec<(u32, ValType)> = vec![(7, ValType::I32)];
    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();
    // len_a = load32(a); len_b = load32(b)
    insts.local_get(0); insts.i32_load(MemArg { align: 2, offset: 0, memory_index: 0 }); insts.local_set(2);
    insts.local_get(1); insts.i32_load(MemArg { align: 2, offset: 0, memory_index: 0 }); insts.local_set(3);
    // total = len_a + len_b
    insts.local_get(2); insts.local_get(3); insts.i32_add(); insts.local_set(4);
    // dest = heap_ptr (global 0)
    insts.global_get(0); insts.local_set(7);
    // store header: *(dest) = total
    insts.local_get(7); insts.local_get(4); insts.i32_store(MemArg { align: 2, offset: 0, memory_index: 0 });
    // pa = a + 4; pb = b + 4
    insts.local_get(0); insts.i32_const(4); insts.i32_add(); insts.local_set(5);
    insts.local_get(1); insts.i32_const(4); insts.i32_add(); insts.local_set(6);
    // copy first: for i in 0..len_a: *(dest+4+i) = *(pa+i)
    insts.i32_const(0); insts.local_set(8);
    insts.block(BlockType::Empty);
    insts.loop_(BlockType::Empty);
      insts.local_get(8); insts.local_get(2); insts.i32_ge_u();
      insts.br_if(1);
      // store byte
      insts.local_get(7); insts.i32_const(4); insts.i32_add(); insts.local_get(8); insts.i32_add();
      insts.local_get(5); insts.local_get(8); insts.i32_add();
      insts.i32_load8_u(MemArg { align: 0, offset: 0, memory_index: 0 });
      insts.i32_store8(MemArg { align: 0, offset: 0, memory_index: 0 });
      // i++
      insts.local_get(8); insts.i32_const(1); insts.i32_add(); insts.local_set(8);
      insts.br(0);
    insts.end(); // loop
    insts.end(); // block
    // copy second: for i in 0..len_b: *(dest+4+len_a+i) = *(pb+i)
    insts.i32_const(0); insts.local_set(8);
    insts.block(BlockType::Empty);
    insts.loop_(BlockType::Empty);
      insts.local_get(8); insts.local_get(3); insts.i32_ge_u();
      insts.br_if(1);
      // store byte
      insts.local_get(7); insts.i32_const(4); insts.i32_add(); insts.local_get(2); insts.i32_add(); insts.local_get(8); insts.i32_add();
      insts.local_get(6); insts.local_get(8); insts.i32_add();
      insts.i32_load8_u(MemArg { align: 0, offset: 0, memory_index: 0 });
      insts.i32_store8(MemArg { align: 0, offset: 0, memory_index: 0 });
      // i++
      insts.local_get(8); insts.i32_const(1); insts.i32_add(); insts.local_set(8);
      insts.br(0);
    insts.end(); // loop
    insts.end(); // block
    // heap_ptr = align4(dest + 4 + total)
    insts.local_get(7); insts.i32_const(4); insts.i32_add(); insts.local_get(4); insts.i32_add();
    insts.i32_const(3); insts.i32_add();
    insts.i32_const(-4); insts.i32_and();
    insts.global_set(0);
    // return dest
    insts.local_get(7);
    insts.end();
    Ok(fenc)
}
