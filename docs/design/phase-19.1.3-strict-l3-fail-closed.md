# Phase 19.1.3 - Strict L3 Fail-Closed Gate

## Status
Design lock for `19.1.3` in `docs/TODO.md`.
This blocks strict-mode `L3` claims when assumptions are present but not fully labeled.

## Goal
Fail closed in strict mode by rejecting unlabeled assumption boundaries before an `L3` claim can be accepted.

## Design Principles Check
- Simple for users: strict-mode failure explains exactly which VC/function/assumption label is missing.
- AI-friendly: deterministic failure code (`C031`) and stable message shape for tooling loops.
- Provably correct: strict mode does not allow ambiguous assumption boundaries to pass as high-assurance claims.
- Crypto-focused: release-grade assurance claims require explicit dependency boundary labeling in emitted artifacts.

## Scope
1. Add strict-mode unlabeled-assumption gate with build diagnostic `C031`.
2. Define unlabeled assumptions in strict mode as:
   - empty `message`,
   - empty `symbols`,
   - or any empty symbol label.
3. Keep existing strict boundary-shape checks (`C014`) unchanged.

## Locked Behavior
1. `--compiler-mode strict` + `--emit-vcs` runs:
   - strict boundary-shape validation (`C014`),
   - strict unlabeled-assumption fail-closed gate (`C031`).
2. Labeled assumptions remain valid in strict mode.
3. Strict mode with no assumptions remains valid.

## Non-Goals
1. No trust-anchor integration changes (`19.1.4`).
2. No additional tier semantics beyond fail-closed strict-mode enforcement.
3. No solver/model expansion work in this slice.

## Exit Criteria for 19.1.3
1. Strict mode blocks unlabeled assumptions with deterministic diagnostics.
2. Regression tests cover strict-mode acceptance for labeled assumptions.
3. TODO/rollout/docs record completion and behavior.

## References
- `docs/TODO.md`
- `docs/rollout/DEVPLAN.md`
- `docs/diagnostics.md`
- `docs/proofs/vc-schema.md`
- `docs/proofs/proof-section.md`
