# Phase 19.2.2 - Deterministic Crypto-Intrinsic Modeling Policy

## Status
Design lock for `19.2.2` in `docs/TODO.md`.
This slice makes crypto assumption outputs explicitly per-intrinsic so assurance tooling can consume stable machine-readable metadata.

## Goal
Emit deterministic per-intrinsic assurance metadata for crypto-model assumptions without changing current proof strength claims.

## Design Principles Check
- Simple for users: crypto boundaries stay under one familiar assumption id (`crypto.uninterpreted`) while exposing clear intrinsic-level labels.
- AI-friendly: outputs gain a stable `intrinsic_levels` field with deterministic ordering and fixed keys.
- Provably correct: unsupported crypto semantics remain explicitly downgraded to `assumed` (`L0`) instead of being implied as proved.
- Crypto-focused: each touched crypto/constant-time intrinsic is labeled directly in emitted assurance artifacts.

## Scope
1. Keep existing `crypto.uninterpreted` assumption boundary semantics.
2. Add `intrinsic_levels` for `crypto.uninterpreted` items in:
   - VC JSON output (`--emit-vcs`),
   - `clearlang.proof` section VC assumption entries.
3. Define deterministic shape per entry:
   - `intrinsic`: exact symbol,
   - `tier`: `L0`,
   - `label`: `assumed`.
4. Preserve deterministic ordering aligned with the existing `symbols` order.

## Locked Behavior
1. Only crypto assumption items carry `intrinsic_levels` in this slice.
2. `intrinsic_levels` values are emitted in lockstep with `symbols`.
3. Assurance-tier semantics are unchanged: any assumption boundary keeps VC/module tier at `L0`.

## Non-Goals
1. No change to SMT crypto encoding strength (still uninterpreted in current model).
2. No new strict-mode policy checks beyond existing assumption labeling checks.
3. No new assumption ids per crypto intrinsic in this slice.

## Exit Criteria for 19.2.2
1. VC JSON/proof-section artifacts include deterministic per-intrinsic crypto assurance metadata.
2. Regression tests/fixtures lock the new field shape.
3. TODO/rollout/proof docs reflect completed policy.

## References
- `docs/TODO.md`
- `docs/rollout/DEVPLAN.md`
- `docs/proofs/vc-schema.md`
- `docs/proofs/proof-section.md`
- `docs/proofs/crypto-limitations.md`
