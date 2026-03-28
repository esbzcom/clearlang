# Phase 25.0.9 - Zero Assumption-Boundary Release Gate

## Status
Design lock + implementation note for `25.0.9` in `docs/TODO.md`.

## Goal
Add CI/release enforcement so milestone_3 publish workflows reject release bundles that carry assumption boundaries.

## Policy
1. Prohibited release assumption boundaries:
   - `unsigned.int_model`
   - `bitwise.uninterpreted`
   - `crypto.uninterpreted`
2. Release verification under `--require-assurance proved_all` is fail-closed:
   - rejects signature payloads with non-empty `assumption_boundaries`,
   - rejects assurance manifests with non-empty `assumptions.items`.
3. CI proof regression includes a targeted gate test that forges `proof_status=proved_all` over an assumed bundle and confirms verification rejects it.

## Enforcement Wiring
- Runtime verifier policy check (`V005`) enforces zero assumption boundaries for `proved_all`.
- CI proof regression step includes:
  - `cargo test -p clg-cli --test signing verify_require_assurance_rejects_manifest_with_assumption_boundaries_when_proved_all`
- Milestone 3 release gate lock (`docs/evidence/milestone_3-proof-gate.lock.json`) pins prohibited boundary ids and required proof commands.

## Exit Criteria for 25.0.9
1. Verification fails closed when `proved_all` is paired with any assumption boundary in signed payload/manifest.
2. CI includes explicit regression coverage for this fail-closed behavior.
3. TODO marks `25.0.9` complete with design + evidence + test references.

## References
- `docs/TODO.md`
- `docs/evidence/milestone_3-proof-gate.lock.json`
- `docs/evidence/milestone_3-proof-gate.md`
- `crates/cli/src/commands/verify.rs`
- `crates/cli/tests/signing/tests.rs`
