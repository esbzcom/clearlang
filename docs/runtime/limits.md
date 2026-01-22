# Runtime Limits

ClearLang inserts an internal step meter for loops/recursion, and hosts can add
Wasmtime limits as a second line of defense.

## Internal step meter
- A mutable global `__clg_fuel_remaining` is initialized at module load.
- Function entry decrements by 1000; loop headers decrement by 1.
- When the counter reaches 0, the runtime traps with `R004`.
- Default fuel budget: `1_000_000` steps per invocation.

## Wasmtime fuel (host instruction budget)
- Enable with `Config::consume_fuel(true)`.
- Add fuel per execution with `Store::set_fuel(...)`.
- Recommended default for tests/CI: `1_000_000_000_000` fuel units per run.
- Out-of-fuel traps are surfaced as Wasmtime errors; treat them as bounded-execution failures.

## Wasmtime epoch deadlines (host time budget)
- Enable with `Config::epoch_interruption(true)`.
- Set per-store deadlines with `Store::set_epoch_deadline(...)`.
- Hosts advance time via `Engine::increment_epoch()` (e.g., once per tick on a timer thread).
- Recommended default for tests/CI: deadline `200` with a coarse host tick (5ms).

## Memory limits
- Prefer a `StoreLimiter` to cap linear memory and table growth.
- Recommended starting point: 64 MiB for CI, 128 MiB for local development.
- Keep limits consistent with the string allocator's bump model in `docs/runtime/strings.md`.

## Notes
- The internal meter is always present; host limits are optional.
- Production hosts should tighten limits based on expected workloads and resource budgets.
- `clg run` relies on the internal meter and reports `R004` when limits are exceeded.
