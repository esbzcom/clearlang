use anyhow::Result;
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
