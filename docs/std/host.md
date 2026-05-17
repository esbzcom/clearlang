# Package: `std::host`

## Purpose
Capability-gated host interfaces for runtime interaction in deterministic profiles.

## Key Types
- `Storage`
- `Log`
- `Env`
- `HostError`

## Class/Method Draft

### `Storage`
Methods:
- `contains(key: Bytes) -> Result<Bool, HostError>`
- `get(key: Bytes) -> Result<Option<Bytes>, HostError>`
- `set(key: Bytes, value: Bytes) -> Result<Bool, HostError>` (`true` if value changed, `false` if already equal)
- `delete(key: Bytes) -> Result<Bool, HostError>` (`true` if key existed and was removed, `false` if key absent)

### `Log`
Methods:
- `info(code: ErrorCode, message: String) -> Result<Bool, HostError>` (on `Ok`, value must be `true`)
- `warn(code: ErrorCode, message: String) -> Result<Bool, HostError>` (on `Ok`, value must be `true`)
- `error(code: ErrorCode, message: String) -> Result<Bool, HostError>` (on `Ok`, value must be `true`)

### `Env`
Methods:
- `chain_id() -> Result<U64, HostError>`
- `caller() -> Result<Bytes, HostError>`
- `block_height() -> Result<U64, HostError>`
- `timestamp() -> Result<U64, HostError>`
- `gas_left() -> Result<U64, HostError>`

### `HostError`
Methods:
- `code(self) -> ErrorCode`
- `equals(self, other: HostError) -> Bool`

## First-Production Cut (recommended)
- Keep `Storage`: `get`, `set`, `delete`, `contains`.
- Keep `Env`: `chain_id`, `caller`, `block_height`, `timestamp`.
- Keep deterministic `HostError`.
- Defer `Env::gas_left` plus richer telemetry/log levels and optional host metadata until after first launch.

## Notes
- Host functions are capability-gated and must fail closed when unavailable.
- Runtime profile must explicitly define available host capabilities.
- Return semantics are deterministic and must not depend on host-specific logging/storage side effects beyond the documented booleans.
- For `Log::*`, `Ok(false)` is invalid API behavior in production profiles and must fail conformance tests.

## Summary
- Defines explicit IO/mutation boundaries for stateful host operations.
- Requires host-profile conformance and fail-closed capability checks.
- Keeps host interaction surfaces auditable and policy-driven.
