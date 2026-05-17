# Phase 19.5.3 - Release Policy Gates for Assurance Manifests

## Status
Design lock for `19.5.3` in `docs/TODO.md`.

## Goal
Add deterministic release-policy checks so pipelines reject signed assurance manifests whose assurance tier is below a required minimum.

## Design Principles Check
- Simple for users: release gating is one explicit verify invocation with manifest and policy files.
- AI-friendly: policy schema is stable and minimal (`schema_version`, `minimum_assurance_tier`), with deterministic verify code `V005`.
- Provably correct: policy decisions are evaluated only after signature/hash verification and manifest-signature verification.
- Crypto-focused: release decisions remain bound to signed artifacts and explicit trust boundaries.

## Scope
1. Extend `clg verify` with release-policy flags:
   - `--assurance-manifest <FILE>`
   - `--release-policy <FILE>`
2. Verify manifest signature with provided public key before policy evaluation.
3. Enforce hash binding:
   - manifest artifact hashes must match verified signature payload hashes.
4. Enforce tier gate:
   - reject when manifest `assurance.tier` ranks below policy `minimum_assurance_tier`.
5. Add integration coverage for pass/fail paths and mismatch rejection.

## Locked Behavior
1. Policy schema:
   - `schema_version = 1`
   - `minimum_assurance_tier = L0|L1|L2|L3`
2. Manifest schema:
   - `schema_version = 1`
   - `payload.format = clg.assurance_manifest.v1`
3. Policy gate failures use verify diagnostic `V005`.

## Non-Goals
1. No automatic remote policy retrieval.
2. No additional policy dimensions beyond minimum assurance tier in this slice.
3. No changes to trust-anchor compile-time mode semantics (`V004` path unchanged).

## Exit Criteria for 19.5.3
1. Verify command supports deterministic release-policy gate checks on assurance manifests.
2. Integration tests cover tier pass/fail and manifest-hash mismatch rejection.
3. `docs/TODO.md` and rollout plan mark Phase 19.5 complete.

## References
- `docs/TODO.md`
- `docs/TODO.md`
- `docs/diagnostics.md`
- `crates/cli/src/main.rs`
- `crates/cli/src/commands/verify.rs`
- `crates/cli/src/signing.rs`
- `crates/cli/tests/signing.rs`
