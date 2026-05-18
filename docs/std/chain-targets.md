# Namespace Family: `std::chain::<target>`

## Purpose
Chain-specific wrappers and typed helpers layered on top of `std::host` capabilities.

## Sub-Namespaces (examples)
- `address` (target-specific)
- `io` (target-specific host IO wrappers)
- `env` (target-specific environment wrappers)

## Types (examples)
- `Address`
- Target-specific event/value helper types

## Type/Function Draft (example shape)

### `std::<target>::address`
Functions:
- `from_bytes(input: Bytes) -> Result<Address, ContractError>`
- `to_bytes(addr: Address) -> Bytes`
- `equals(lhs: Address, other: Address) -> Bool`

### `std::<target>::io`
Functions:
- `send(to: Address, amount: Amount) -> Result<Int, HostError>`
- `call(to: Address, payload: Bytes) -> Result<Bytes, HostError>`
- `emit(event: Event) -> Result<Int, HostError>`

### `std::<target>::env`
Functions:
- `network_id() -> Result<U64, HostError>`
- `tx_hash() -> Result<Bytes, HostError>`

## First-Production Cut (recommended)
- Defer this package family from first production release unless a target chain launch requires it.
- If enabled for a specific launch, keep minimal `Address` + `io.send`/`io.call`/`io.emit` only.

## Notes
- `std::chain::<target>` must remain additive and must not alter `std::core` semantics.
- Capability and trust policy checks MUST remain fail-closed.

## Contract Conformance Checklist
- Target wrappers MUST preserve core determinism guarantees.
- Host capability checks MUST fail closed with stable diagnostics.
- Chain-specific encoding/address semantics MUST be explicitly versioned before activation.

## Summary
- Keeps chain specializations outside core std to preserve portability.
- Allows chain ecosystems to evolve without destabilizing core language/runtime contracts.
- Treated as stretch/deferred relative to first production `must-have` std surfaces.


