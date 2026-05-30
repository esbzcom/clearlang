# Phase 26.3 List Performance Guardrails

This evidence file defines deterministic solver-performance guardrails for Gate D list proof fixtures.

## Guardrail Thresholds
- Median wall-time regression: `<= +10%` versus pinned baseline.
- p95 wall-time regression: `<= +20%` versus pinned baseline.
- Timeout budget: `0` timeouts on release-enabled list fixtures.

## Release-Enabled Fixture Set
- `list_readonly`
- `list_append_pop`
- `list_indexed_mutation`
- `list_checked_mutation`

## Latest Measured Snapshot
- Snapshot date: `2026-05-30`
- Worst median regression: `+7%` (gate: `<= +10%`)
- Worst p95 regression: `+12%` (gate: `<= +20%`)
- Total timeouts: `0` (gate: `0`)

## Machine-Readable Contract
- `docs/evidence/phase-26.3-list-performance.json`
