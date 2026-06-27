# Shared Std Rollback Procedure

Use this runbook when a published shared std artifact is wrong, unverifiable, or operationally unsafe.

## Triggers
- Shared std provenance digest mismatch.
- Shared std package signature verification failure.
- ABI mismatch against the locked release flow.
- Registry copy missing required bundle members.

## Immediate Actions
1. Stop promoting new shared-std-backed releases.
2. Mark the affected shared std version as suspect in operator notes.
3. Capture failing diagnostics and artifact digests.

## Recovery
1. Select the last known-good shared std version directory.
2. Re-point lock inputs to that version.
3. Re-run:
   - `clg release`
   - `clg verify-bundle --require-provenance`
4. If signer compromise is suspected, revoke the affected `key_id` before republishing.

## Post-Incident
- Record the bad version, replacement version, and cause.
- Update key rotation or publish procedure if operator error was involved.
- Keep the failed artifacts for audit history; do not silently rewrite them.
