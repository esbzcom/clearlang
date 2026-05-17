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
- `set(key: Bytes, value: Bytes) -> Result<Int, HostError>`
- `delete(key: Bytes) -> Result<Int, HostError>`

### `Log`
Methods:
- `info(code: ErrorCode, message: String) -> Result<Int, HostError>`
- `warn(code: ErrorCode, message: String) -> Result<Int, HostError>`
- `error(code: ErrorCode, message: String) -> Result<Int, HostError>`

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
- Defer richer telemetry/log levels and optional host metadata until after first launch.

## Notes
- Host functions are capability-gated and must fail closed when unavailable.
- Runtime profile must explicitly define available host capabilities.

## Summary
- Defines explicit IO/mutation boundaries for stateful host operations.
- Requires host-profile conformance and fail-closed capability checks.
- Keeps host interaction surfaces auditable and policy-driven.
