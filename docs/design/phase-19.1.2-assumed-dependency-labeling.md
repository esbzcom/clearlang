# Phase 19.1.2 - Assumed Dependency Labeling

## Status
Design lock for `19.1.2` in `docs/TODO.md`.
This requires non-proved primitive and external dependencies to be labeled as explicit `assumed` boundaries in emitted reports.

## Goal
Guarantee no hidden dependency assumptions in emitted VC/proof artifacts by labeling:
1. Non-proved primitive std dependencies.
2. External imported dependencies.

## Design Principles Check
- Simple for users: emitted reports directly show which dependency surfaces are assumed, without requiring internal compiler knowledge.
- AI-friendly: boundaries are deterministic machine IDs with stable categories and symbol lists.
- Provably correct: non-proved dependency surfaces are never implied as proved; they are explicitly marked `assumed`.
- Crypto-focused: audit artifacts preserve dependency trust boundaries for production contract/release policy checks.

## Scope
1. Extend assumption categories to include `primitive` and `external`.
2. Emit new boundary IDs:
   - `primitive.unproved`
   - `external.dependency`
3. Preserve existing IDs:
   - `unsigned.int_model`
   - `bitwise.uninterpreted`
   - `crypto.uninterpreted`
4. Keep strict proof-mode validation deterministic for all known IDs/categories.

## Emission Rules (Locked)
1. Primitive dependency boundary:
   - Triggered when called std intrinsics are in the current non-proved primitive set.
   - Emitted with `id = primitive.unproved`, `category = primitive`, `status = assumed`, and touched intrinsic symbols.
2. External dependency boundary:
   - Triggered when a called function resolves to an external imported dependency.
   - Emitted with `id = external.dependency`, `category = external`, `status = assumed`, and touched external symbols.

## Non-Goals
1. No change to tier fail-closed policy (`19.1.3` owns `L3` blocking behavior).
2. No trust-anchor integration changes (`19.1.4`).
3. No claim that dependency behavior is formally proved in this slice.

## Exit Criteria for 19.1.2
1. VC JSON/proof artifacts include explicit assumed labels for non-proved primitive/external dependencies.
2. Strict-mode assumption validation accepts and validates the new IDs/categories.
3. Regression tests cover primitive and external assumed-boundary emission.

## References
- `docs/TODO.md`
- `docs/TODO.md`
- `docs/proofs/vc-schema.md`
- `docs/proofs/proof-section.md`
- `docs/design/phase-19.1.1-assurance-tiers.md`
