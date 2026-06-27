# Phase 29.8.1 - Milestone 3 Closure Lock

Date: 2026-06-27
Status: Locked
Owner: std-packages-owner

## Purpose

Close Milestone 3 with one final product lock that confirms the supported production surface after
the breadth-complete decision in `29.8.0`.

This lock exists to remove any remaining ambiguity about:

- the exact shared-package set that is supported in production
- which items are intentionally deferred and therefore not release blockers
- the steady-state maintenance rules that apply after Milestone 3 ships

## Final Supported Production Contract

Milestone 3 closes with two supported production delivery modes:

1. `embedded` - the default supported production mode
2. `shared` - an explicit opt-in supported production mode using the schema-v2 fail-closed shared
   workflow

The exact supported bundled shared-package set is:

- `std::text`
- `std::int`
- `std::sequence`
- `std::codec`
- `std::contract`
- `std::eth`
- `std::solana`
- `std::cosmos`

Anything outside that package-id set is outside the Milestone 3 supported shared contract.

## Why Milestone 3 Is Closed

Milestone 3 is production-ready because the required product conditions are now all locked:

1. the shared workflow has one deterministic production shape for lock, release, verify-bundle,
   runtime loading, key rotation, and rollback
2. the supported shared package boundary is exact and documentation-backed
3. `std::contract`, `std::eth`, `std::solana`, and `std::cosmos` all ship under the same
   fail-closed trust, ABI, digest, provenance, and replay rules as the baseline shared packages
4. the product still defaults to the simpler `embedded` path, so existing users are not forced into
   a more operationally complex mode

That satisfies the Milestone 3 product bar described by the README principles:

- simple for users: the default remains simple and the opt-in shared surface is explicit
- AI-friendly: the package boundary, activation rules, and diagnostics remain predictable
- provably correct: supported surfaces stay narrow, typed, and fail closed instead of heuristic
- crypto-focused: release, trust, provenance, and runtime loading remain deterministic and
  audit-grade

## Intentionally Deferred Non-Goals

The following items remain intentionally deferred after Milestone 3 and are not release blockers:

1. changing the default delivery mode from `embedded` to `shared`
2. expanding the supported shared package-id set beyond the eight locked packages in this document
3. adding broader chain-specific helpers, runtime IO namespaces, event surfaces, or cross-chain
   convenience APIs
4. introducing implicit migration, implicit activation, or silent downgrade between delivery modes
5. weakening trust, ABI, digest, provenance, replay, rollback, or signer-rotation checks for
   operator convenience
6. treating experimental or compatibility-only std packages as production shared packages without a
   new design lock, documentation update, and behavior-gate coverage

These are future-scope product decisions, not unfinished Milestone 3 work.

## Post-M3 Steady-State Maintenance Expectations

After Milestone 3, the supported production contract is maintained under the following steady-state
rules:

1. any change to shared activation semantics, package membership, trust roots, bundle layout, ABI
   acceptance, or runtime validation must update the authoritative design and operator docs in the
   same patch
2. any expansion of the supported shared-package set requires a new design lock plus matching lock,
   release, verify-bundle, and runtime coverage
3. `embedded` and `shared` remain equally authoritative supported modes until a later explicit
   product decision changes that status
4. release readiness continues to require `xtask release-precheck` and
   `xtask shared-std-distribution-check` to stay green
5. signer rotation, rollback, and incident handling remain documentation-backed operational
   requirements, not optional best-effort procedures
6. unsupported or partial shared workflows continue to fail closed rather than falling back to
   embedded or accepting approximate evidence

## Final Product Position

Milestone 3 now closes as a full production milestone for the currently planned ClearLang shared-std
surface.

That means:

- no additional Phase 29 work is required to call the current supported product releasable
- remaining future std-package or chain-surface work is additive scope, not closure debt
- the authoritative supported surface after Milestone 3 is the exact contract locked here and in
  `28.9.2`

## References

- `README.md`
- `docs/design/phase-28.9.2-shared-std-product-contract-lock.md`
- `docs/design/phase-29.5.0-chain-adapter-production-ordering-lock.md`
- `docs/design/phase-29.8.0-breadth-complete-production-decision.md`
- `docs/release/shared-std-user-guide.md`
- `docs/release/shared-std-operations.md`
- `docs/todo/milestone_3_roadmap.md`
