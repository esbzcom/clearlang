# Milestone 3 Proof/Release Gate Evidence

This document is the evidence index for TODO item `25.0.8`.

## Required Proof Matrix Artifacts
- `docs/proofs/proof-coverage-matrix.json`
- `docs/proofs/proof-coverage-matrix.md`
- `docs/proofs/verified-std-core-subset.json`
- `docs/proofs/verified-std-core-subset.md`
- Lock artifact: `docs/evidence/milestone_3-proof-gate.lock.json`

## CI and Release-Train Gate
- Proof regression command set (CI `checks` job):
  - `cargo test -p clg-cli --test vc_snapshots`
  - `cargo test -p clg-cli --test proof_coverage_matrix`
  - `cargo test -p clg-cli --test verified_std_core_subset`
  - `cargo test -p clg-cli --test profile_regression_gate`
  - `cargo test -p clg-cli --test milestone3_release_gate`
- Tag gate (`refs/tags/milestone_3`) runs:
  - `cargo test -p clg-cli --test milestone3_release_gate`
  - with `CLG_ENFORCE_MILESTONE3_RELEASE_GATE=1`

## Release Workflow Proof Requirements
- Production build must include:
  - `--release-profile production`
- Release verification must include theorem-grade gate:
  - `--require-assurance proved_all`
- Canonical workflow reference:
  - `docs/release-process.md`

## Sign-off Snapshot
- Evidence index prepared: 2026-03-28.
- Gate status: complete for `25.0.8`.
