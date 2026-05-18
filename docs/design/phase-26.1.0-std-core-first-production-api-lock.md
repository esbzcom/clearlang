# Phase 26.1.0 - Std Core First-Production API Lock

## Status
Design lock for `26.1.0` (`std::core` first-production API cut).

## Goal
Freeze the first-production `std::core` method surface and lock deterministic failure behavior before implementation expands broader std packages.

## Locked First-Production API Surface

### `Option<T>` (locked baseline)
- `is_some`
- `is_none`
- `map`
- `and_then`
- `filter`
- `or_else`
- `unwrap_or`
- `unwrap_or_else`
- `expect`
- `to_result`

### `Result<T,E>` (locked baseline)
- `is_ok`
- `is_err`
- `map`
- `map_err`
- `and_then`
- `or_else`
- `unwrap_or`
- `unwrap_or_else`
- `expect`
- `expect_err`
- `to_option`

### `ErrorCode` (locked baseline)
- `new`
- `value`
- `equals`

### `CoreError` + `Panic` (locked baseline)
- `CoreError::{new,with_message,code,message,equals}`
- `Panic::fail`

## Deferred from First Production
Deferred unless a concrete release blocker is proven:
- `Option/Result::{flatten,contains*,map_or*}`
- `Option::to_result_else`
- `ErrorCode::{from_parts,domain}`
- `CoreError::{with_cause,cause}`

## Deterministic Failure Contract Lock
1. `Option::expect` failure
   - Must fail closed with deterministic error code payload.
   - Must not leak host/runtime-specific extra fields in stable machine output.
2. `Result::expect`/`expect_err` failure
   - Must fail closed with deterministic error code payload.
   - Must preserve stable diagnostics shape across replay.
3. `Panic::fail`
   - Terminal failure path; execution does not continue.
   - Failure diagnostics are deterministic and machine-readable.

## Contract-Test Matrix (required)
1. `expect_success_path`
   - `Option::expect(Some)` and `Result::expect(Ok)` succeed with stable outputs.
2. `expect_failure_path`
   - `Option::expect(None)` and `Result::expect(Err)` fail closed with deterministic diagnostics.
3. `expect_err_failure_path`
   - `Result::expect_err(Ok)` fails closed with deterministic diagnostics.
4. `panic_terminal_path`
   - `Panic::fail` terminates execution and emits deterministic failure payload.
5. `diagnostics_replay_parity`
   - Repeated identical runs produce identical failure code + payload shape.

## Scope Boundary
- This lock freezes API and diagnostics/test contracts only.
- Implementation details for lowering/runtime wiring remain in `26.1.1+`.

## References
- `docs/TODO.md`
- `docs/std/core.md`
- `docs/design/phase-26.0.1-std-scope-and-governance-lock.md`
