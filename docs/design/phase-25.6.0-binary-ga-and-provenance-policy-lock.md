# Phase 25.6.0 - Binary GA + Provenance Policy Lock

## Status
Policy lock for Phase `25.6` execution order and acceptance semantics.

## Purpose
Resolve two open scoping decisions before further implementation:
1. What counts as the GA artifact in `25.6`.
2. Whether provenance is optional or required for GA release workflows.

## Locked Decisions

### 1) GA Artifact Definition (`25.6.x`)
Phase `25.6` targets **distributable `clg` compiler binaries** as the GA artifact scope.

Implications:
- `25.6.2`/`25.6.3`/`25.6.4`/`25.6.5`/`25.6.6`/`25.6.12`/`25.6.13` are evaluated against binary distribution outcomes.
- Existing signed release-bundle outputs (`*.wasm`, proof/signature manifests) remain required assurance evidence, but are not the only deliverable for GA.

### 2) GA Target Matrix (`25.6.1`)
Support policy is locked as:
- Baseline GA targets (required gates): `windows`, `linux`, and `macos`.

### 3) Provenance Requirement (`25.6.10`)
Provenance attestation is:
- **Required for GA release-train artifacts**.
- Optional for local/dev ad-hoc runs outside GA release-train gating.

`clg verify-bundle` must fail closed in GA release-train contexts when required provenance is absent, malformed, or signature-invalid.

## Non-Goals (This Lock)
- Defining the final provenance schema fields (tracked by `25.6.10` implementation).
- Completing binary packaging/install mechanics (tracked by `25.6.2`/`25.6.3`/`25.6.6`).
- Completing cross-runner reproducibility witness implementation (tracked by `25.6.12`).

## References
- `docs/TODO.md` (`25.6.1`, `25.6.10`, `25.6.12`, `25.6.13`)
- `docs/release-process.md`
- `crates/cli/src/commands/release.rs`
