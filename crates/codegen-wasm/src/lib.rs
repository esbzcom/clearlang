use anyhow::{bail, Context, Result};
use lumi_ast::{Expr, Func, Program, Type};
use std::collections::HashMap;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection,
    Module, TypeSection, ValType, NameSection, NameMap,
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

    // Build function types, map names to indices
    let mut types = TypeSection::new();
    let mut fn_index_by_name = std::collections::HashMap::new();
    for (i, f) in ir.funcs.iter().enumerate() {
        fn_index_by_name.insert(f.name.as_str(), i as u32);
        let params: Vec<ValType> = f.params.iter().map(|_| ValType::I32).collect();
        let results: Vec<ValType> = match f.ret { Some(_) => vec![ValType::I32], None => vec![] };
        types.ty().function(params, results);
    }
    module.section(&types);

    // Function section
    let mut functions = FunctionSection::new();
    for i in 0..ir.funcs.len() {
        functions.function(i as u32);
    }
    module.section(&functions);

    // Export main if present
    if let Some((i, _)) = ir.funcs.iter().enumerate().find(|(_, f)| f.name == "main") {
        let mut exports = ExportSection::new();
        exports.export("main", ExportKind::Func, i as u32);
        module.section(&exports);
    }

    // Code section: encode each function body
    let mut codes = CodeSection::new();
    for f in &ir.funcs {
        let func = encode_ir_function(f, &fn_index_by_name)?;
        codes.function(&func);
    }
    module.section(&codes);

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

fn encode_ir_function<'a>(f: &IrFunction, fn_indices: &std::collections::HashMap<&'a str, u32>) -> Result<Function> {
    // Compute locals: values >= params are locals; params are indices 0..P-1
    let params_len = f.params.len() as u32;
    let mut max_id = params_len.saturating_sub(1);
    for ins in &f.body {
        match ins {
            IrInstr::IConst { dst, .. } => max_id = max_id.max(dst.0),
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
                let idx = *fn_indices
                    .get(callee.as_str())
                    .ok_or_else(|| anyhow::anyhow!(format!("unknown callee `{}`", callee)))?;
                insts.call(idx);
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
