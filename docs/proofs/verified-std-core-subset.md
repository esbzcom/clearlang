# Verified std/core subset (`verified.std_core.v1`)

Status: Phase `19.3.2` publication baseline.

Machine-readable source of truth:
- `docs/proofs/verified-std-core-subset.json`

This profile captures the currently verified std/core surfaces that are eligible for strict verified-by-construction workflows.

## Included coverage IDs

- `feature.contract_implication_core`
- `feature.refinement_premises`
- `feature.loop_obligations`
- `feature.linear_collection_control_flow`

## Inclusion policy

An entry is eligible only if:
- it exists in `docs/proofs/proof-coverage-matrix.json`,
- `status == "proved"`,
- `assumption_boundary == null`,
- all tier labels are `proved`.

## Exclusions (current)

All intrinsic rows remain excluded from this subset because they are currently `assumed`
(`unsigned.int_model`, `bitwise.uninterpreted`, `crypto.uninterpreted`) and therefore not part
of the verified std/core baseline.

## Regression obligations

Each subset entry is tied to at least one concrete regression test reference in
`docs/proofs/verified-std-core-subset.json` (`regression_obligations`).

Subset regression checks verify:
- subset IDs remain mapped to `proved` coverage rows with no assumption boundaries,
- referenced test files exist and still contain the declared test function names.

CI profile-regression gate fixtures:
- `docs/proofs/verified-profile-fixtures.json`
- enforced by `crates/cli/tests/profile_regression_gate.rs`
