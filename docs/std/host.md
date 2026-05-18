# Namespace: `std::host`

## Purpose
Capability-gated host interfaces for runtime interaction in deterministic profiles.

## Sub-Namespaces
- `storage`
- `log`
- `env`
- `host_error`

## Types
- `HostError`

## Type/Function Draft

### `storage`
Functions:
- `contains(key: Bytes) -> Result<Bool, HostError>`
- `get(key: Bytes) -> Result<Option<Bytes>, HostError>`
- `set(key: Bytes, value: Bytes) -> Result<Bool, HostError>` (`true` if value changed, `false` if already equal)
- `delete(key: Bytes) -> Result<Bool, HostError>` (`true` if key existed and was removed, `false` if key absent)

### `log`
Functions:
- `info(code: ErrorCode, message: String) -> Result<Bool, HostError>` (on `Ok`, value must be `true`)
- `warn(code: ErrorCode, message: String) -> Result<Bool, HostError>` (on `Ok`, value must be `true`)
- `error(code: ErrorCode, message: String) -> Result<Bool, HostError>` (on `Ok`, value must be `true`)

### `env`
Functions:
- `chain_id() -> Result<U64, HostError>`
- `caller() -> Result<Bytes, HostError>`
- `block_height() -> Result<U64, HostError>`
- `timestamp() -> Result<U64, HostError>`
- `gas_left() -> Result<U64, HostError>`

### `host_error`
Functions:
- `code(err: HostError) -> ErrorCode`
- `equals(a: HostError, other: HostError) -> Bool`

## First-Production Cut (recommended)
- Keep `storage`: `get`, `set`, `delete`, `contains`.
- Keep `env`: `chain_id`, `caller`, `block_height`, `timestamp`.
- Keep deterministic `HostError`.
- Defer `env::gas_left` plus richer telemetry/log levels and optional host metadata until after first launch.

## Notes
- Host functions are capability-gated and MUST fail closed when unavailable.
- Runtime profile MUST explicitly define available host capabilities.
- Return semantics MUST be deterministic and MUST NOT depend on host-specific logging/storage side effects beyond documented return values.
- `log::*` functions MUST return `Ok(true)` on successful delivery; `Ok(false)` is invalid API behavior and MUST fail conformance tests.

## Security Considerations
- Host boundary calls MUST be treated as untrusted integration points and validated by capability policy.
- Storage key/value handling SHOULD avoid secret-dependent logging or branching in host adapters.
- Environment values (`chain_id`, `timestamp`, etc.) MUST be treated as external inputs and validated by contract logic where required.

## Contract Conformance Checklist
- Missing/disabled capability access MUST fail deterministically with stable `HostError` codes.
- `storage::set/delete` boolean semantics MUST be consistent across host implementations.
- `log::*` success semantics MUST be strictly `Ok(true)` only.
- Host profile conformance tests MUST cover both allow and deny paths for each capability.

## Summary
- Defines explicit IO/mutation boundaries for stateful host operations.
- Requires host-profile conformance and fail-closed capability checks.
- Keeps host interaction surfaces auditable and policy-driven.


