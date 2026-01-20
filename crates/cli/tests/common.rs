use once_cell::sync::Lazy;
use wasmtime::{Config, Engine, Store};

const DEFAULT_FUEL: u64 = 1_000_000;
const DEFAULT_EPOCH_DEADLINE: u64 = 1_000_000;

pub static ENGINE: Lazy<Engine> = Lazy::new(|| {
    let mut config = Config::new();
    config.consume_fuel(true);
    config.epoch_interruption(true);
    Engine::new(&config).expect("engine")
});

#[inline]
pub fn engine() -> &'static Engine {
    &ENGINE
}

pub fn store(engine: &Engine) -> Store<()> {
    let mut store = Store::new(engine, ());
    store.set_fuel(DEFAULT_FUEL).expect("set fuel");
    store.set_epoch_deadline(DEFAULT_EPOCH_DEADLINE);
    store
}
