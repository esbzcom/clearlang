# Phase 25.1.9 - Proof Artifact Emission and Verify Wiring

## Status
Implementation lock for `25.1.9` in `docs/TODO.md`.

## Delivered
1. `clg build` accepts `--emit-proof <FILE>`.
2. Build emits deterministic JSON proof artifact (`clg.proof_artifact.v1`) with:
   - solver profile snapshot + hash,
   - per-VC status entries,
   - deterministic summary counters.
3. `clg verify` accepts `--proof-artifact <FILE>`.
4. Verify checks proof artifact consistency when proof-claim fields are present in the signed payload.

## Fail-Closed Behavior
1. If signed payload includes proof-artifact claims, verify requires `--proof-artifact`.
2. Artifact parse/hash/profile mismatches fail with `V006`.
3. Legacy signatures without proof-artifact claims remain backward-compatible.

## Validation Coverage
- `crates/cli/tests/signing/tests.rs`:
  - `verify_accepts_matching_proof_artifact_claims`
  - `verify_rejects_missing_proof_artifact_when_claim_present_with_v006`
  - `verify_rejects_tampered_proof_artifact_with_v006`

## References
- `docs/proofs/proof-artifact-schema.md`
- `docs/design/phase-25.1.3-emit-proof-schema-v1.md`
