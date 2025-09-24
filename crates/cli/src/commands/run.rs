use anyhow::{Context, Result};
use std::path::PathBuf;
use wasmtime as wt;

pub fn run(file: PathBuf, invoke: String) -> Result<()> {
    let engine = wt::Engine::default();
    let module = wt::Module::from_file(&engine, &file)
        .with_context(|| format!("loading {}", file.display()))?;
    let mut store = wt::Store::new(&engine, ());
    let instance = wt::Instance::new(&mut store, &module, &[]).context("instantiating module")?;
    let func = instance
        .get_typed_func::<(), i32>(&mut store, &invoke)
        .with_context(|| format!("export `{}` not found or wrong type", invoke))?;
    let result = func.call(&mut store, ()).context("invoking function")?;
    println!("{}", result);
    Ok(())
}
