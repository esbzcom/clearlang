# Phase 26.1.4.6 - `std::host` First-Production API Lock

## Scope
This lock defines the first-production `std::host` capability surface and conformance requirements for Gate B execution.

## First-Production API (Release-Enabled)
- `std::host::storage::{contains,get,set,delete}`
- `std::host::log::{info,warn,error}`
- `std::host::env::{chain_id,caller,block_height,timestamp}`
- `std::host::host_error::{code,equals}`

## Determinism and Fail-Closed Contracts
- Missing or denied host capabilities MUST fail closed with stable `HostError` codes.
- `storage::set` and `storage::delete` MUST return `Result<Bool, HostError>` with deterministic boolean semantics:
  - `set`: `true` when value changed, `false` when already equal.
  - `delete`: `true` when key existed, `false` when key was absent.
- `log::{info,warn,error}` successful delivery MUST be `Ok(true)`; `Ok(false)` is invalid behavior.
- Environment calls MUST be deterministic for a fixed execution context.

## Current-Surface Compatibility Notes
- Existing host-backed namespaces (`std::env`, `std::wasi`, and host-backed `std::crypto`) remain valid in this phase.
- `std::host` is the canonical capability ownership namespace; compatibility surfaces must not weaken policy gates.

## Deferred (Non-Release) Symbols
- `std::host::env::gas_left`
- Additional telemetry/logging verbosity surfaces beyond the locked set.

## Gate B Exit for `std::host`
`std::host` slice is complete when:
1. API/doc/metadata surface is consistent with this lock.
2. `typed|runtime|proved` statuses are recorded in `docs/std/coverage-matrix.md`.
3. Host-profile conformance tests cover allow and deny paths for each release-enabled capability.
4. Release profile fails closed for deferred host symbols.
