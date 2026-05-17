# Package Family: `std::chain::<target>`

## Purpose
Chain-specific wrappers and typed helpers layered on top of `std::host` capabilities.

## Key Types (examples)
- `std::eth::Address` (example)
- `std::eth::io::*` (example host IO wrappers)
- Target-specific event/value helper types

## Class/Method Draft (example shape)

### `std::<target>::Address`
Methods:
- `from_bytes(input: Bytes) -> Result<Address, ContractError>`
- `to_bytes(self) -> Bytes`
- `equals(self, other: Address) -> Bool`

### `std::<target>::io`
Methods:
- `send(to: Address, amount: Amount) -> Result<Int, HostError>`
- `call(to: Address, payload: Bytes) -> Result<Bytes, HostError>`
- `emit(event: Event) -> Result<Int, HostError>`

### `std::<target>::Env`
Methods:
- `network_id() -> Result<U64, HostError>`
- `tx_hash() -> Result<Bytes, HostError>`

## First-Production Cut (recommended)
- Defer this package family from first production release unless a target chain launch requires it.
- If enabled for a specific launch, keep minimal `Address` + `io.send`/`io.call`/`io.emit` only.

## Notes
- `std::chain::<target>` must remain additive and must not alter `std::core` semantics.
- Capability and trust policy checks remain fail-closed.

## Summary
- Keeps chain specializations outside core std to preserve portability.
- Allows chain ecosystems to evolve without destabilizing core language/runtime contracts.
- Treated as stretch/deferred relative to first production `must-have` std surfaces.
