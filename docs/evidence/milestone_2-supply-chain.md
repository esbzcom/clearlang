# Milestone 2 Supply-Chain Compliance Gate

This document tracks TODO item `24.2.10`.

## Gate Definition
CI runs:
- `python scripts/ci/milestone2_supply_chain_gate.py`

The gate fails closed if either condition is violated:
1. A resolved Cargo dependency (including workspace crates) is missing both `license` and `license_file` metadata.
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
- CI runs this gate after std-core artifact reproducibility so generated package artifacts are included in compliance scope.
- Package metadata scan scope is deterministic: tracked `clg.package-metadata.json` files plus generated `tmp/std-core/**/clg.package-metadata.json`.
- Release train remains blocked if this gate fails.
