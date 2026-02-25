# Phase 19.4.2 - Proof-failure slicing and counterexample reporting

## Status
Design lock for `19.4.2` in `docs/TODO.md`.

## Goal
Expose deterministic, machine-readable proof-failure slices and counterexample envelopes that map directly to user source spans.

## Design Principles Check
- Simple for users: each VC carries a focused span plus related spans, so failures can be shown at the exact clause location.
- AI-friendly: failure slices and counterexample envelopes use stable JSON fields under `diagnostics`.
- Provably correct: reporting is trace-only and does not change VC semantics, assumptions, or assurance tiers.
- Crypto-focused: strict assurance workflows get explicit failure localization without weakening trust-boundary rules.

## Scope
1. Emit `diagnostics.failure_slice` for each VC with:
   - VC id and clause-kind classification,
   - focus span and related spans (`pre`/`post` as available).
2. Emit `diagnostics.counterexample` envelope for failed-VC workflows with:
   - deterministic report format and state,
   - symbol bindings extracted from the VC goal expression,
   - span mapping for where the model applies.
3. Add integration tests for deterministic field shape and expected span/counterexample metadata.

## Locked Behavior
1. Reporting fields are deterministic functions of emitted VC metadata (`vc_id`, `pre/post spans`, `post.ast`).
2. Counterexample values remain `null` until external solver/model data is attached; envelope shape is locked now.
3. New fields are emitted in VC JSON only for this slice.

## Non-Goals
1. No solver execution or model generation in this slice.
2. No automatic source rewrites.
3. No proof-section/CBOR schema changes for counterexample transport in this slice.

## Exit Criteria for 19.4.2
1. VC JSON includes `diagnostics.failure_slice` and `diagnostics.counterexample`.
2. CLI integration and fixture snapshot tests pass with deterministic outputs.
3. `docs/TODO.md` and `docs/rollout/DEVPLAN.md` advance focus to `19.4.3`.

## References
- `docs/TODO.md`
- `docs/proofs/vc-schema.md`
- `crates/cli/src/commands/build.rs`
- `crates/cli/tests/cli_it/vc_outputs.rs`
- `crates/cli/tests/vc_snapshots.rs`
