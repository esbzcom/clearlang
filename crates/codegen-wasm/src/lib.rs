use anyhow::{bail, Context, Result};
use lumi_ast::{Expr, Func, Program, Type};
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
    let main: &Func = ast
        .funcs
        .iter()
        .find(|f| f.name == "main")
        .context("missing `main` function")?;

    if !main.params.is_empty() {
        bail!("only zero-arg main supported in this phase");
    }
    if main.ret != Type::Int {
        bail!("only `main() -> Int` is supported in this phase");
    }

    let value_i32: i32 = match main.body {
        Expr::Int(n) => i32::try_from(n).context("main Int literal out of i32 range")?,
        _ => bail!("only Int literal bodies are supported for now"),
    };

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
