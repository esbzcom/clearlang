# Phase 25.1.10 - Proof Artifact and Solver-Profile Hash Binding

## Status
Implementation lock for `25.1.10` in `docs/TODO.md`.

## Goal
Bind proof artifact and solver profile identity into signed claims so verify can detect drift/tampering.

## Delivered Binding Contract
1. Signature payload now conditionally includes:
   - `proof_artifact_hash`,
   - `solver_profile_hash`,
   - `solver_profile` snapshot.
2. Assurance manifest `payload.artifacts` now conditionally includes:
   - `proof_artifact_hash`,
   - `solver_profile_hash`.
3. Verify enforces consistency across:
   - signature payload claims,
   - assurance manifest claims (when present),
   - provided proof artifact file (`--proof-artifact`).

## Error Mapping
- Consistency failures use `V006`.
- Existing release-policy failures remain `V005` when proof-artifact claims are absent.

## Compatibility
1. Legacy signatures/manifests without the new fields continue to verify.
2. New claim fields are additive and deterministic.

## References
- `docs/design/phase-25.1.4-deterministic-solver-profile-lock.md`
- `docs/design/phase-25.1.9-proof-artifact-emit-verify-wiring.md`
