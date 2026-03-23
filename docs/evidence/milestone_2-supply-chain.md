# Milestone 2 Supply-Chain Compliance Gate

This document tracks TODO item `24.2.10`.

## Gate Definition
CI runs:
- `cargo run -p xtask -- milestone2-supply-chain-gate`

The gate fails closed if either condition is violated:
1. A resolved Cargo dependency (including workspace crates) is missing both `license` and `license_file` metadata.
2. Any committed `clg.package-metadata.json` entry has invalid compliance fields:
   - schema version not in `{0,1}`,
   - missing/non-`sha256:` digest,
   - missing artifact path,
   - referenced artifact file does not exist.
3. Runtime dependency artifacts (`clg.runtime-link.json` + lock/store index)
   are missing or inconsistent:
   - no runtime-link artifact is discovered in scope,
   - runtime package digest is not `sha256:` or does not match artifact bytes,
   - runtime package digest does not match both `clg.lock.json` and
     `clg.package-store-index.json`,
   - runtime-link bindings reference provider package ids not present in the
     runtime package list.

## Artifacts
CI uploads `milestone2-supply-chain` from `tmp/sbom/*`, including:
- `milestone2-sbom.json`
- `milestone2-package-artifact-report.json`
- `milestone2-runtime-dependency-report.json`
- `milestone2-supply-chain-summary.json`

## Notes
- The gate provides deterministic JSON artifacts for audit/replay.
- CI runs this gate after std-core artifact reproducibility so generated package artifacts are included in compliance scope.
- Package metadata scan scope is deterministic: tracked `clg.package-metadata.json` files plus generated `tmp/std-core/**/clg.package-metadata.json`.
- Runtime dependency scan scope is deterministic: tracked runtime-link artifacts plus generated `tmp/perf/**/clg.runtime-link.json`.
- Release train remains blocked if this gate fails.
