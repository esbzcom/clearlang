# ClearLang Milestone 3 Release Notes

## Scope
Milestone 3 focuses on theorem-grade release assurance workflows and first usable binary release operations.

## Compatibility Matrix
- Windows: GA baseline
- Linux: GA baseline
- macOS: GA baseline

## Highlights
- Primary release workflow: `clg release` with deterministic bundle emission.
- Bundle-first verification: `clg verify-bundle`.
- Key rotation support: `verify-bundle --keyring` key-id resolution.
- Provenance support: signed bundle provenance artifact, with fail-closed required mode (`--require-provenance`).
- Deterministic release readiness preflight: `clg release --check-only`.
- Signed binary bundle artifact: `xtask milestone3-binary-bundle` emits binary + checksums + signed metadata + SBOM/license evidence.
- Installer-channel outputs: `xtask milestone3-installer-channels` emits canonical Homebrew and winget metadata from the signed GitHub bundle set.

## Known Limitations
- apt/rpm repository publication remains deferred.
- Windows MSI packaging remains deferred.

## Upgrade Notes
1. Adopt keyring-driven verification for historical key rotation safety.
2. For release-train workflows, require provenance verification (`--require-provenance`).
3. Generate Homebrew/winget metadata from the signed bundle set for the exact release tag being published.
4. Use milestone_3 release-train checklist/runbook before publishing.

## Verification Commands
```powershell
clg release --check-only --root <project-root>
clg release --key <signing-key.json> --pubkey <public-key.json> --root <project-root>
clg verify-bundle --bundle <path/to/release-bundle.json> --keyring <path/to/release-keyring.json> --require-provenance
```
