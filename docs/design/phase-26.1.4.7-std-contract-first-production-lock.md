# Phase 26.1.4.7 - `std::contract` First-Production API Lock

## Scope
This lock defines the first-production chain-agnostic `std::contract` API slice for Gate B execution.

## First-Production API (Release-Enabled Target)
- `std::contract::address::{from_bytes,to_bytes,equals}`
- `std::contract::amount::{from_u64,value,add_checked,sub_checked,is_zero}`
- `std::contract::event::{new,topic,payload}`
- `std::contract::contract_error::{code,equals}`

## Determinism Contracts
- `address::from_bytes` MUST enforce canonical address bytes for the active contract profile.
- `address::to_bytes` followed by `from_bytes` MUST round-trip deterministically for valid addresses.
- `amount::{add_checked,sub_checked}` MUST fail closed on overflow/underflow with stable `ContractError` mapping.
- Event topic/payload accessors MUST preserve bytes exactly.

## Evolution Contracts
- Chain-specific address or event semantics MUST stay in `std::chain::<target>` and remain additive.
- Future numeric width expansion for `Amount` MUST be additive and must not change `from_u64/value` semantics.

## Gate B Exit for `std::contract`
`std::contract` slice is complete when:
1. API/doc/metadata surface is consistent with this lock.
2. `typed|runtime|proved` statuses are recorded in `docs/std/coverage-matrix.md`.
3. Release profile fails closed for any unimplemented/deferred contract-domain symbols.
4. CI has deterministic conformance tests for address round-trip, amount checked-math, and event byte preservation.
