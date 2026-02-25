# Phase 19.3.1 - Strict Language Profile (Verified-by-Construction)

## Status
Design lock for `19.3.1` in `docs/TODO.md`.

## Goal
Define strict profile behavior for `clg build --compiler-mode strict` so production verification flows fail closed on deferred or unchecked proof surfaces.

## Design Principles Check
- Simple for users: one mode (`strict`) enforces one policy boundary with deterministic diagnostics.
- AI-friendly: failure shape is stable (`C033`) and points to the exact assumption boundary causing rejection.
- Provably correct: strict profile rejects assumed proof boundaries instead of permitting deferred/unchecked semantics.
- Crypto-focused: strict profile avoids shipping contracts that rely on unchecked external or deferred crypto/bitwise proof surfaces.

## Scope
1. Strict profile gate:
   - In `--compiler-mode strict`, reject builds when any VC contains an assumption boundary.
2. Diagnostics:
   - Add build diagnostic `C033` for strict profile boundary rejection.
3. Regression coverage:
   - CLI integration tests confirm strict-mode rejection of assumed surfaces and standard-mode acceptance for labeled assumptions.

## Locked Behavior
1. `--compiler-mode strict` still requires `--emit-vcs` (`C029`) and disallows `--proof-strict=false` (`C030`).
2. `--compiler-mode strict` still runs strict assumption-shape checks (`C014`) and unlabeled-assumption checks (`C031`).
3. `--compiler-mode strict` now rejects any emitted assumption boundary with `C033`.
4. `--compiler-mode standard` behavior remains unchanged: labeled assumptions are allowed and emitted in artifacts.

## Non-Goals
1. No new assumption ids or assurance-tier schema changes.
2. No trust-anchor workflow changes.
3. No `verify` command behavior changes.

## Exit Criteria for 19.3.1
1. Strict-mode builds fail with deterministic `C033` when deferred/unchecked assumed boundaries are present.
2. Strict-mode builds with zero assumptions remain valid.
3. TODO/rollout/docs point next execution to `19.3.2`.

## References
- `docs/TODO.md`
- `docs/rollout/DEVPLAN.md`
- `docs/diagnostics.md`
- `docs/design/phase-19.0.3-compiler-modes.md`
