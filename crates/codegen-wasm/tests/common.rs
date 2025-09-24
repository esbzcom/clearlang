use once_cell::sync::Lazy;

pub static ENGINE: Lazy<wasmtime::Engine> = Lazy::new(|| wasmtime::Engine::default());

#[inline]
pub fn engine() -> &'static wasmtime::Engine {
    &*ENGINE
}
