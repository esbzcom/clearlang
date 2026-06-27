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
  - `cargo test -p clg-cli --test phase25_solver_support_matrix`
  - `cargo test -p clg-cli --test phase25_solver_supply_chain_gate`
  - `cargo test -p clg-cli --test proof_coverage_matrix`
  - `cargo test -p clg-cli --test verified_std_core_subset`
  - `cargo test -p clg-cli --test profile_regression_gate`
  - `cargo test -p clg-cli --test milestone3_release_gate`
  - `cargo test -p clg-cli --test milestone3_proof_parity`
  - `cargo test -p clg-cli --test signing verify_require_assurance_rejects_manifest_with_assumption_boundaries_when_proved_all`
  - `cargo test -p clg-cli --test signing verify_require_assurance_rejects_symbols_outside_proved_allowlist_when_proved_all`
  - `cargo test -p clg-cli --test cli_it diagnostics::production_release_profile_rejects_non_proved_std_surface_with_c122`
  - `cargo test -p clg-cli --test cli_it diagnostics::production_release_profile_rejects_crypto_assumption_boundary_with_c123`
  - `cargo test -p clg-cli --test cli_it parse_rejects_theorem_keyword_with_explicit_code`
- Release-target parity artifact:
  - Windows: `milestone3-proof-parity-windows`
  - Linux: `milestone3-proof-parity-linux`
  - macOS: `milestone3-proof-parity-macos`
- Platform scope:
  - Milestone 3 release-target parity now covers Windows, Linux, and macOS.
  - parity replay runs and artifact emission are enforced on `windows-latest`, `ubuntu-latest`, and `macos-latest`.
  - cross-platform release-target parity is enforced by `milestone3-proof-parity-compare`.
  - `cargo test -p clg-cli --features rust-z3-lib --test solver_backend_parity`
  - `cargo test -p clg-cli --features rust-z3-lib --test solver_rust_cutover_packaging`
- Tag gate (`refs/tags/milestone_3`) runs:
  - `cargo test -p clg-cli --test milestone3_release_gate`
  - with `CLG_ENFORCE_MILESTONE3_RELEASE_GATE=1`

## Release Workflow Proof Requirements
- Production build must include:
  - `--release-profile production`
- Release verification must include theorem-grade gate:
  - `--require-assurance proved_all`
- Release verification should include signed manifest input:
  - `--assurance-manifest <FILE>`
- Prohibited release assumption boundaries:
  - `unsigned.int_model`
  - `bitwise.uninterpreted`
  - `crypto.uninterpreted`
- Fail-closed rule:
  - `proved_all` verification rejects bundles/manifests carrying any assumption boundaries.
- Release symbol allowlist rule:
  - bundle `std::...` symbols must be a subset of matrix entries with `status == proved`.
  - canonical source: `docs/proofs/proof-coverage-matrix.json`
- Production compile release-surface gate:
  - `--release-profile production` fails with `C122` when used `std::...` symbols are outside the proved allowlist.
- Production compile crypto-boundary gate:
  - `--release-profile production` fails with `C123` when `crypto.uninterpreted` remains in VC assumptions.
- Docs/profile alignment gate:
  - root `README.md` must state that `permissive`/`standard` are dev/evidence profiles and do not imply `proved_all`.
  - `examples/projects/README.md` quick-release flow must include `--release-profile production` and `--require-assurance proved_all`.
- Language-surface minimization gate:
  - parser rejects `theorem` syntax with `P014`.
  - theorem-grade remains certification status (build/verify policy), not milestone_3 syntax.
- Canonical workflow reference:
  - `docs/release-process.md`

## Sign-off Snapshot
- Evidence index prepared: 2026-03-28.
- Gate status: complete for `25.0.8`.
