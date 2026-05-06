# Phase 25.6.6 - Single Distributable Bundle Contract

## Status
Contract lock for a single operator-facing distributable release package.

## Bundle Contract
Single distributable package contract for milestone_3:
- Package root directory contains:
  - `bin/<platform>/clg(.exe)` for GA baseline targets.
  - `release-bundle/` evidence artifacts:
    - `<stem>.wasm`
    - `<stem>.strict-import-map.json`
    - `<stem>.vc.json`
    - `<stem>.proof.json`
    - `<stem>.sig.json`
    - `<stem>.assurance.json`
    - `<stem>.provenance.json`
    - `<stem>.release-bundle.json`
  - `checksums/` detached SHA-256 files for binaries and bundle artifacts.
  - `sbom/` and license evidence outputs.
  - `release_notes/milestone_3.md`.

Operator contract:
- Verification starts from the bundle manifest (`clg verify-bundle`).
- Manual per-file verification wiring is non-canonical.

## Non-Goals (This Contract)
- Defining channel-specific installer formats (MSI/deb/rpm/brew) in this phase.

## References
- `docs/release-process.md`
- `docs/design/phase-25.6.13-binary-publication-policy-lock.md`
