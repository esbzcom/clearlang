# Phase 25.1.11 - Deterministic `--emit-proof` Reproducibility Gate

## Status
Implementation lock for `25.1.11` in `docs/TODO.md`.

## Goal
Require deterministic proof artifact bytes/hashes across identical inputs and runs.

## Delivered
1. Added integration replay test:
   - `crates/cli/tests/proof_artifact_replay.rs`
   - `emit_proof_replay_with_identical_inputs_is_byte_identical`
2. Wired replay test into proof regression CI step.
3. Added replay test command to `milestone_3` gate lock required CI list.

## Determinism Contract
For identical source and CLI inputs:
1. emitted proof artifact bytes are identical,
2. emitted proof artifact hash is identical.

## References
- `.github/workflows/ci.yml`
- `docs/evidence/milestone_3-proof-gate.lock.json`
- `crates/cli/tests/ci_workflow.rs`
