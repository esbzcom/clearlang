# Namespace: `std::contract`

## Purpose
Contract-oriented domain types for deterministic smart-contract development.

## Sub-Namespaces
- `address`
- `amount`
- `event`
- `contract_error`

## Types
- `Address`
- `Amount`
- `Event`
- `ContractError`

## Type/Function Draft

### `address`
Functions:
- `from_bytes(input: Bytes) -> Result<Address, ContractError>`
- `to_bytes(addr: Address) -> Bytes`
- `equals(lhs: Address, other: Address) -> Bool`
- `is_zero(addr: Address) -> Bool`

### `amount`
Functions:
- `from_u64(value: U64) -> Amount`
- `value(amount: Amount) -> U64`
- `add_checked(lhs: Amount, rhs: Amount) -> Result<Amount, ContractError>`
- `sub_checked(lhs: Amount, rhs: Amount) -> Result<Amount, ContractError>`
- `is_zero(amount: Amount) -> Bool`

### `event`
Functions:
- `new(topic: Bytes, payload: Bytes) -> Event`
- `topic(event: Event) -> Bytes`
- `payload(event: Event) -> Bytes`

### `contract_error`
Functions:
- `code(err: ContractError) -> ErrorCode`
- `equals(a: ContractError, other: ContractError) -> Bool`

## First-Production Cut (recommended)
- Keep `Address`: `from_bytes`, `to_bytes`, `equals`.
- Keep `Amount`: `value`, checked `add/sub`.
- Keep `Event::new` plus getters.
- Keep deterministic `ContractError`.

## Notes
- Contract types MUST stay chain-agnostic in `std::contract`.
- Chain-specific semantics MUST belong in `std::chain::<target>`.
- `Address::from_bytes` MUST enforce canonical address format constraints defined by the active contract profile.
- `Amount` arithmetic MUST preserve deterministic overflow/underflow behavior via `ContractError`.

## Security Considerations
- Address parsing/serialization MUST be canonical to avoid replay or aliasing ambiguity.
- Amount operations in settlement paths SHOULD use checked APIs only.
- Event topic/payload encoding SHOULD be canonicalized via `std::codec` policies when used for hashing/signing.

## Contract Conformance Checklist
- `Address` equality and byte round-trip behavior MUST be deterministic.
- `Amount` checked math failures MUST map to stable `ContractError` codes.
- `Event` construction/accessors MUST preserve payload bytes exactly.

## Summary
- Provides common contract abstractions over core/runtime primitives.
- Targets audit-grade clarity for state transition and event emission logic.
- Maintains strict compatibility with release-proof and host capability policies.


