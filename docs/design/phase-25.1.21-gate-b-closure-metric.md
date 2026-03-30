# Phase 25.1.21: Gate B Closure Metric

## Goal
Define Gate B completion as a machine-checkable closure rule for release-enabled surfaces and deterministic proof evidence.

## Closure rule
Gate B is considered closed only when:
1. release-enabled surfaces are a subset of `status == proved` entries in `docs/proofs/proof-coverage-matrix.json`;
2. release bundles contain zero prohibited assumption boundaries:
   - `unsigned.int_model`
   - `bitwise.uninterpreted`
   - `crypto.uninterpreted`
3. deterministic solver/proof evidence tests remain wired in CI;
4. release workflow requires:
   - `--release-profile production`
   - `--assurance-manifest`
   - `--require-assurance proved_all`

## Lock
- `docs/design/phase-25.1.21-gate-b-closure-metric.lock.json`
