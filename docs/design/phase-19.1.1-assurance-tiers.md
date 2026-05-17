# Phase 19.1.1 - Assurance Tiers in Diagnostics/Artifacts

## Status
Design lock for `19.1.1` in `docs/TODO.md`.
This introduces explicit assurance-tier metadata in emitted diagnostics/artifacts.

## Goal
Make assurance levels explicit and machine-readable across core outputs:
- `L0`: `assumed`
- `L1`: `checked core`
- `L2`: `verified module`
- `L3`: `verified package profile`

## Design Principles Check
- Simple for users: every emitted artifact carries the same tier labels, so users can interpret assurance without cross-referencing hidden policy.
- AI-friendly: tier metadata is deterministic and structured (`tier`, `label`, `levels`) for tooling loops and policy automation.
- Provably correct: no implied assurance upgrades; current tier selection is explicit and conservative.
- Crypto-focused: artifacts expose assurance boundaries directly for audit/release workflows in production contract systems.

## Scope
1. VC JSON emission includes per-VC `assurance`.
2. `clearlang.proof` section includes per-VC and module-level `assurance`.
3. Signing payload includes module-level `assurance`.
4. Schema docs describe the shape and current tier mapping.

## Tier Mapping (Current Locked Behavior)
1. VC/module with one or more assumption boundaries: `L0` (`assumed`).
2. VC/module with no assumption boundaries: `L1` (`checked core`).
3. `L2`/`L3` labels are emitted in `levels` for deterministic tooling compatibility but are not auto-claimed by current build flow.

## Artifact Shape
`assurance` map:
- `tier`: `L0|L1|L2|L3`
- `label`: tier label text
- `levels`:
  - `L0`: `assumed`
  - `L1`: `checked core`
  - `L2`: `verified module`
  - `L3`: `verified package profile`

## Non-Goals
1. No strict-mode fail-closed enforcement for `L3` (owned by `19.1.3`).
2. No trust-anchor integration changes (owned by `19.1.4`).
3. No policy gate changes for release pipelines yet.

## Exit Criteria for 19.1.1
1. Tier metadata is present in VC JSON, proof section, and signing payloads.
2. Tier schema is documented in proof schema docs.
3. Regression tests validate tier emission in both JSON and CBOR artifacts.

## References
- `docs/TODO.md`
- `docs/TODO.md`
- `docs/proofs/vc-schema.md`
- `docs/proofs/proof-section.md`
- `docs/design/phase-18.0-open-questions.md`
