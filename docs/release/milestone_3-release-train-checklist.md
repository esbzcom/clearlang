# Milestone 3 Binary Release-Train Checklist

## Preconditions
- `checks` workflow is green.
- `milestone3-proof-parity` is green.
- `milestone3-test-parity-compare` is green.
- `milestone3-binary-smoke` is green.
- `milestone3-binary-repro-compare` is green.

## Artifact Checklist
- GA baseline binaries are present for Windows and Linux.
- Binary checksums are present.
- Signed binary metadata (`milestone3-binary-bundle.signed.json`) is present.
- Release bundle includes signed provenance artifact.
- `clg verify-bundle --require-provenance` passes against release keyring.
- SBOM/license evidence artifacts are present.
- `release_notes/milestone_3.md` is published.

## Sign-off Checklist
- Publication policy lock (`25.6.13`) reviewed.
- Incident/rollback runbook reviewed by on-call owner.
- Known limitations and compatibility matrix verified in release notes.

## References
- `docs/release/milestone_3-binary-incident-runbook.md`
- `docs/design/phase-25.6.13-binary-publication-policy-lock.md`
