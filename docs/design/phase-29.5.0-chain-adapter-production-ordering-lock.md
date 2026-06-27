# Phase 29.5.0 - Chain-Adapter Production Ordering Lock

Date: 2026-06-27
Status: Locked
Owner: std-packages-owner

## Purpose

Lock the Wave 2 execution order for promoting chain-adapter packages into the supported shared-std
set.

This lock exists to prevent ad hoc chain-package activation after `std::contract` and to keep the
remaining breadth-completion work aligned with the README design principles:

- simple for users
- AI-friendly
- provably correct
- crypto-focused

## Scope

This lock covers the current Wave 2 chain-adapter package candidates only:

- `std::eth`
- `std::solana`
- `std::cosmos`

Each package currently remains a compatibility surface and is not yet a supported shared package.

## Execution Order

The locked production-ordering sequence is:

1. `std::eth`
2. `std::solana`
3. `std::cosmos`

No package may skip ahead of this order without a new design lock that explicitly revises the
prioritization rationale and acceptance criteria.

## Why This Order Is Correct

### 1. `std::eth` Comes First

`std::eth` is the clearest first chain-adapter promotion because:

1. it is already the most visible chain namespace in the public product framing and examples
2. it keeps the first chain-adapter slice focused on a narrow address-constructor surface
3. it exercises the chain-adapter packaging path without forcing broader runtime-IO or chain-event
   semantics into scope

This best fits the design principles:

- simple for users: the repo already uses `std::eth` as the canonical example chain package
- AI-friendly: small, stable namespace with deterministic constructor semantics
- provably correct: bounded constructor-only surface is easier to gate than broader chain behavior
- crypto-focused: highest immediate smart-contract relevance with minimal additional surface area

### 2. `std::solana` Comes Second

`std::solana` should land only after the first chain-adapter promotion path is routine.

It has the same bounded surface shape as `std::eth`:

- `from_bytes`
- `from_array`

but it still introduces a distinct chain identity and canonical address-type expectations. That
makes it the right second slice after the first packaging/trust/release/runtime pattern has been
proven on one chain adapter.

### 3. `std::cosmos` Comes Third

`std::cosmos` remains in the same bounded constructor class, but it should be last in the current
Wave 2 sequence because it is the least useful package to take first for product clarity and the
most likely to motivate broader target-specific naming and encoding questions if introduced too
early.

Promoting it last keeps the execution order conservative:

1. prove the first chain-adapter path on the most visible namespace
2. repeat it on a second major target namespace
3. close the remaining bounded Wave 2 adapter surface only after the prior two are routine

## Shared-Vs-Embedded Activation Expectations

Chain-adapter shared support must preserve the current product contract:

1. `embedded` remains the default supported production delivery mode
2. `shared` remains an explicit opt-in production mode only
3. no silent fallback from `shared` to `embedded` is allowed
4. no implicit activation of all chain adapters is allowed

That means each chain adapter must be activated independently through:

- explicit `std.delivery = "shared"`
- explicit `std.packages[]`
- explicit package-specific lock, release, verify-bundle, and runtime evidence

Embedded compatibility for existing chain namespaces remains valid and is not deprecated by this
lock.

## Full-Production Closure Criteria

Wave 2 chain-adapter work is complete only when all of the following are true:

1. each package in this order has a bounded slice lock naming its exact module and symbol set
2. each package is promoted under the same fail-closed trust, ABI, digest, provenance, and replay
   rules used by current supported shared packages
3. each package has deterministic manifest/lock/release/verify/runtime happy-path and tamper-path
   coverage
4. user/operator documentation names the exact supported chain-adapter package set
5. the final Milestone 3 closure lock re-runs the product decision and confirms breadth-complete
   production status

## What Stays Deferred

This ordering lock does not authorize:

1. chain-specific IO namespaces or broader chain runtime helpers
2. cross-chain bulk activation in one execution slice
3. proof-grade claims beyond the current bounded constructor surfaces
4. changing the default delivery mode
5. expanding beyond `std::eth`, `std::solana`, and `std::cosmos`

Those remain separate follow-on decisions or future-scope work.

## Next Task

After this ordering lock, the next task is `29.5.1`:

- lock one canonical bounded rollout shape for chain adapters so per-package promotions use the
  same package boundary, migration, trust, and runtime contract

## References

- `README.md`
- `docs/design/phase-26.1.4.9-std-chain-target-first-production-lock.md`
- `docs/design/phase-27.2-external-std-package-plan.v1.json`
- `docs/design/phase-28.9.2-shared-std-product-contract-lock.md`
- `docs/design/phase-29.3.0-std-contract-shared-std-slice-lock.md`
