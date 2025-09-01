use anyhow::{bail, Context, Result};
use lumi_ast::{Expr, Func, Program, Type};
use std::collections::HashMap;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection,
    Module, TypeSection, ValType,
};

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
        Expr::Int(n) => Ok(*n),
        Expr::Bool(_) => bail!("bool not supported in arithmetic evaluator"),
        Expr::Var(name) => env
            .get(name.as_str())
            .copied()
            .ok_or_else(|| anyhow::anyhow!("unbound variable `{}`", name)),
        Expr::Bin { op, lhs, rhs } => {
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
        Expr::Call { callee, args } => {
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
