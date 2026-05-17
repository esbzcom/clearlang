# Package: `std::contract`

## Purpose
Contract-oriented domain types for deterministic smart-contract development.

## Key Types
- `Address`
- `Amount`
- `Event`
- `ContractError`

## Class/Method Draft

### `Address`
Methods:
- `from_bytes(input: Bytes) -> Result<Address, ContractError>`
- `to_bytes(self) -> Bytes`
- `equals(self, other: Address) -> Bool`
- `is_zero(self) -> Bool`

### `Amount`
Methods:
- `from_u64(value: U64) -> Amount`
- `value(self) -> U64`
- `add_checked(self, rhs: Amount) -> Result<Amount, ContractError>`
- `sub_checked(self, rhs: Amount) -> Result<Amount, ContractError>`
- `is_zero(self) -> Bool`

### `Event`
Methods:
- `new(topic: Bytes, payload: Bytes) -> Event`
- `topic(self) -> Bytes`
- `payload(self) -> Bytes`

### `ContractError`
Methods:
- `code(self) -> ErrorCode`
- `equals(self, other: ContractError) -> Bool`

## First-Production Cut (recommended)
- Keep `Address`: `from_bytes`, `to_bytes`, `equals`.
- Keep `Amount`: `value`, checked `add/sub`.
- Keep `Event::new` plus getters.
- Keep deterministic `ContractError`.

## Notes
- Contract types should stay chain-agnostic in `std::contract`.
- Chain-specific semantics belong in `std::chain::<target>`.

## Summary
- Provides common contract abstractions over core/runtime primitives.
- Targets audit-grade clarity for state transition and event emission logic.
- Maintains strict compatibility with release-proof and host capability policies.
