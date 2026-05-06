# ClearLang Milestone 3 Release Notes

## Scope
Milestone 3 focuses on theorem-grade release assurance workflows and first usable binary release operations.

## Compatibility Matrix
- Windows: GA baseline
- Linux: GA baseline
- macOS: preview (non-blocking in this phase)

## Highlights
- Primary release workflow: `clg release` with deterministic bundle emission.
- Bundle-first verification: `clg verify-bundle`.
- Key rotation support: `verify-bundle --keyring` key-id resolution.
- Provenance support: signed bundle provenance artifact, with fail-closed required mode (`--require-provenance`).
- Deterministic release readiness preflight: `clg release --check-only`.
- Signed binary bundle artifact: `xtask milestone3-binary-bundle` emits binary + checksums + signed metadata + SBOM/license evidence.

## Known Limitations
- Channel-specific installers (MSI/deb/rpm/Homebrew) are deferred.
- macOS remains preview in this phase.

## Upgrade Notes
1. Adopt keyring-driven verification for historical key rotation safety.
2. For release-train workflows, require provenance verification (`--require-provenance`).
3. Use milestone_3 release-train checklist/runbook before publishing.

## Verification Commands
```powershell
clg release --check-only --root <project-root>
clg release --key <signing-key.json> --pubkey <public-key.json> --root <project-root>
clg verify-bundle --bundle <path/to/release-bundle.json> --keyring <path/to/release-keyring.json> --require-provenance
```
