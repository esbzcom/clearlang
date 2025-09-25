use once_cell::sync::Lazy;

#[allow(dead_code)]
pub static ENGINE: Lazy<wasmtime::Engine> = Lazy::new(wasmtime::Engine::default);

#[inline]
#[allow(dead_code)]
pub fn engine() -> &'static wasmtime::Engine {
    &ENGINE
}
