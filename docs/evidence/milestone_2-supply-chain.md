# Milestone 2 Supply-Chain Compliance Gate

This document tracks TODO item `24.2.10`.

## Gate Definition
CI runs:
- `python scripts/ci/milestone2_supply_chain_gate.py`

The gate fails closed if either condition is violated:
1. A resolved non-workspace Cargo dependency is missing both `license` and `license_file` metadata.
2. Any committed `clg.package-metadata.json` entry has invalid compliance fields:
   - schema version not in `{0,1}`,
   - missing/non-`sha256:` digest,
   - missing artifact path,
   - referenced artifact file does not exist.

## Artifacts
CI uploads `milestone2-supply-chain` from `tmp/sbom/*`, including:
- `milestone2-sbom.json`
- `milestone2-package-artifact-report.json`
- `milestone2-supply-chain-summary.json`

## Notes
- The gate provides deterministic JSON artifacts for audit/replay.
- Release train remains blocked if this gate fails.
