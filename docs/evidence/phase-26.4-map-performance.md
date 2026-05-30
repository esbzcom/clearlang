# Phase 26.4 Map Performance Guardrails

This evidence file defines deterministic solver-performance guardrails for Gate E map proof fixtures.

## Guardrail Thresholds
- Median wall-time regression: `<= +10%` versus pinned baseline.
- p95 wall-time regression: `<= +20%` versus pinned baseline.
- Timeout budget: `0` timeouts on release-enabled map fixtures.

## Release-Enabled Fixture Set
- `map_readonly`
- `map_membership_overwrite`
- `map_mutation_take`

## Latest Measured Snapshot
- Snapshot date: `2026-05-30`
- Worst median regression: `+6%` (gate: `<= +10%`)
- Worst p95 regression: `+11%` (gate: `<= +20%`)
- Total timeouts: `0` (gate: `0`)

## Machine-Readable Contract
- `docs/evidence/phase-26.4-map-performance.json`
