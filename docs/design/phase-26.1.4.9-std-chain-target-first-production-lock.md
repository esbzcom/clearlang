# Phase 26.1.4.9 - `std::chain::<target>` Deferred Activation Lock

## Scope
This lock keeps `std::chain::<target>` deferred by default and defines activation requirements when a launch needs target-specific namespaces.

## Default Policy (First Production)
- `std::chain::<target>` remains deferred and is not release-enabled by default.
- Chain-specific behavior must not alter core semantics in `std::core`, `std::bytes`, `std::int`, or `std::contract`.

## Current-Surface Compatibility Notes
- Existing target namespaces (`std::eth`, `std::solana`, `std::cosmos`) are compatibility surfaces and remain non-proof-grade until explicitly activated per target checklist.

## Activation Checklist (Per Target)
1. Canonical address/type encoding is locked and versioned.
2. Capability mapping to `std::host` is explicit, deterministic, and fail-closed.
3. Typed/runtime conformance tests pass for positive and negative cases.
4. Coverage matrix rows are updated for the target namespace (`typed|runtime|proved`).
5. Release-profile policy explicitly lists the enabled target namespace.

## Gate B Exit for `std::chain::<target>`
This slice is complete for Gate B when:
1. Deferred-by-default policy is documented and enforced.
2. Activation checklist is published and referenced by release workflows.
3. Coverage matrix marks target namespaces as `deferred` until activated.
