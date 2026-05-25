# Phase 26.2 Finite-Set Performance Guardrails

This evidence file defines deterministic solver-performance guardrails for Gate C finite-set proof fixtures.

## Guardrail Thresholds
- Median wall-time regression: `<= +10%` versus pinned baseline.
- p95 wall-time regression: `<= +20%` versus pinned baseline.
- Timeout budget: `0` timeouts on release-enabled finite-set fixtures.

## Release-Enabled Fixture Set
- `set_subset_membership`
- `set_algebra`
- `set_cardinality`

## Latest Measured Snapshot
- Snapshot date: `2026-05-25`
- Worst median regression: `+6%` (gate: `<= +10%`)
- Worst p95 regression: `+10%` (gate: `<= +20%`)
- Total timeouts: `0` (gate: `0`)

## Machine-Readable Contract
- `docs/evidence/phase-26.2-finite-set-performance.json`
