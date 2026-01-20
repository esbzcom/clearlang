# Wasmtime Runtime Limits

ClearLang relies on Wasmtime host limits to keep execution bounded and deterministic when running compiled Wasm.

## Fuel (instruction budget)
- Enable with `Config::consume_fuel(true)`.
- Add fuel per execution with `Store::set_fuel(...)`.
- Recommended default for tests/CI: `1_000_000` fuel units per run.
- Out-of-fuel traps are surfaced as Wasmtime errors; treat them as bounded-execution failures.

## Epoch deadlines (time budget)
- Enable with `Config::epoch_interruption(true)`.
- Set per-store deadlines with `Store::set_epoch_deadline(...)`.
- Hosts advance time via `Engine::increment_epoch()` (e.g., once per tick on a timer thread).
- Recommended default for tests/CI: deadline `1_000_000` with a coarse host tick (1-10ms).

## Memory limits
- Prefer a `StoreLimiter` to cap linear memory and table growth.
- Recommended starting point: 64 MiB for CI, 128 MiB for local development.
- Keep limits consistent with the string allocator's bump model in `docs/runtime/strings.md`.

## Notes
- Fuel and epoch limits are enforced in integration tests to prevent unbounded execution.
- Production hosts should tighten limits based on expected workloads and resource budgets.
