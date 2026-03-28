# Phase 25.0.8 - CI/Release Gate for Proof Matrix + Evidence

## Status
Design lock + implementation note for `25.0.8` in `docs/TODO.md`.

## Goal
Block publish/tag workflows unless required proof-matrix artifacts and evidence artifacts are present and CI proof gates are wired.

## Policy
1. Proof matrix artifacts are required release evidence:
   - `docs/proofs/proof-coverage-matrix.{json,md}`
   - `docs/proofs/verified-std-core-subset.{json,md}`
2. Milestone 3 evidence index and lock artifact are required:
   - `docs/evidence/milestone_3-proof-gate.md`
   - `docs/evidence/milestone_3-proof-gate.lock.json`
3. CI proof regression suite must include:
   - `vc_snapshots`, `proof_coverage_matrix`, `verified_std_core_subset`, `profile_regression_gate`, `milestone3_release_gate`
4. Tag-time publish gate (`refs/tags/milestone_3`) must fail closed unless `milestone3_release_gate` passes with enforcement enabled.

## Enforcement Wiring
- CI `checks` job includes `milestone3_release_gate` in proof regression commands.
- Dedicated release-train job:
  - `if: startsWith(github.ref, 'refs/tags/milestone_3')`
  - runs `cargo test -p clg-cli --test milestone3_release_gate`
  - sets `CLG_ENFORCE_MILESTONE3_RELEASE_GATE=1`
- Gate test validates:
  - lock schema + required artifact paths,
  - CI wiring for required proof commands and release-train job,
  - release-process proof command requirements.

## Exit Criteria for 25.0.8
1. CI contains an explicit milestone_3 tag gate that runs a fail-closed enforcement test.
2. Required proof matrix + evidence artifacts are locked and validated by tests.
3. TODO marks `25.0.8` complete with links to implementation and evidence.

## References
- `docs/TODO.md`
- `docs/evidence/milestone_3-proof-gate.lock.json`
- `docs/evidence/milestone_3-proof-gate.md`
- `.github/workflows/ci.yml`
- `crates/cli/tests/milestone3_release_gate.rs`
