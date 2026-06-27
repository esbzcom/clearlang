# Milestone 3 Binary Distribution Incident Runbook

## Triggers
- Binary checksum mismatch in published artifacts.
- `clg verify-bundle --require-provenance` failure on published release.
- Cross-runner reproducibility witness mismatch.
- Missing GA target binary or parity artifact.
- Cross-platform proof parity artifact mismatch.
- Homebrew formula or winget manifest URL/SHA mismatch against the published release tag.

## Immediate Actions
1. Freeze milestone_3 tag/release promotion.
2. Mark latest binary artifacts as suspect in release channel notes.
3. Capture failing evidence (workflow URL, artifact hashes, diagnostics).

## Containment
1. Revoke affected key IDs from keyring if signer compromise is suspected.
2. Publish rollback advisory referencing last known-good release tag.
3. Re-run release-train gates against rollback candidate.

## Recovery
1. Rebuild and republish corrected artifacts.
2. Validate:
   - proof parity compare,
   - binary smoke gate,
   - binary reproducibility witness compare,
   - verify-bundle with required provenance,
   - installer channel metadata URL/SHA parity against the published tag.
3. Update release notes with incident summary and remediation.

## Post-Incident
- Add root-cause and preventive controls to follow-up tracking.
- Confirm publication policy and checklist were updated if gaps were found.
