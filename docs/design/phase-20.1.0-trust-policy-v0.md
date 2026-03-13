# Phase 20.1.0 - Strict Trust-Policy v0 Schema

## Goal
Define the minimal trust-policy schema required by strict package/link gates in Phase 20.1.

Scope for this bootstrap slice is intentionally narrow:
- direct package-signature trust decisions only,
- explicit signer allowlist,
- explicit revocation and expiry checks,
- deterministic parsing/validation behavior.

This schema is for strict package preflight and is separate from the existing
compile-time verify trust policy used by `clg verify --verify-mode compile-time`.

## File and Scope
- Canonical filename: `clg.trust-policy.json`.
- Location: module root for the strict build input.
- Applicability: `clg build --compiler-mode strict` preflight only.
- Non-goal: full signer lifecycle policy (tracked in `22.0.4`).

## Schema (v0)

```json
{
  "schema_version": 0,
  "trusted_signers": [
    {
      "key_id": "std-core-release-ed25519-2026q1",
      "scheme": "ed25519",
      "public_key": "hex:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    }
  ],
  "revoked_key_ids": []
}
```

### Required fields
- `schema_version`: integer, must equal `0`.
- `trusted_signers`: array of signer entries (may be empty).
- `revoked_key_ids`: array of signer key ids (may be empty).

### Per-signer required fields
- `key_id`: non-empty string, unique across `trusted_signers`.
- `scheme`: string, must be exactly `ed25519` in v0.
- `public_key`: string, `hex:<64 lowercase hex>` (32-byte key).
- `not_before`: RFC3339 UTC timestamp string.
- `not_after`: RFC3339 UTC timestamp string, strictly greater than `not_before`.

### Rejected in v0
- Unknown top-level keys.
- Unknown signer keys.
- Duplicate `key_id` entries.
- `revoked_key_ids` entries not present in `trusted_signers`.
- Non-UTC or non-RFC3339 timestamps.
- Unsupported signing schemes.

## Determinism Rules
1. Identity key is `key_id`.
2. Internal evaluation order is deterministic:
   - sort `trusted_signers` by `key_id`,
   - sort `revoked_key_ids` lexicographically.
3. Duplicate detection reports the lexicographically first conflicting `key_id`.
4. Signature trust decision uses only:
   - policy file bytes,
   - package signature envelope,
   - evaluator clock input from preflight context (no ambient system-time reads).

## Trust Decision (v0)
A signer is accepted only when all are true:
1. signer `key_id` exists in `trusted_signers`,
2. signer `key_id` is not listed in `revoked_key_ids`,
3. evaluator time is within `[not_before, not_after)`,
4. signature cryptographically verifies under the pinned `public_key` and `scheme`.

Any violation fails closed.

## Integration With 20.1 Gates
- Trust gate (`20.1.2.3`): untrusted/revoked/expired/invalid-signature signer fails with `C103`.
- Determinism gate (`20.1.2.7`): identical inputs, including policy file and evaluator time input, produce identical trust decisions and diagnostics ordering.

## Diagnostics Mapping (Design-Locked Proposal)
- Missing/unreadable trust policy: `C103`.
- Malformed trust-policy schema/version: `C103`.
- Signer trust failure (unknown/revoked/expired/signature-invalid): `C103`.

Final canonical code registration remains tracked under TODO `20.1.4.3`.

## Remediation (Bootstrap)
When strict preflight fails on trust policy:
1. ensure `<module-root>/clg.trust-policy.json` exists and is readable,
2. use schema v0 exactly (`schema_version`, `trusted_signers`, `revoked_key_ids`),
3. ensure each signer has valid `ed25519` key material and valid UTC time window,
4. rotate or un-revoke by replacing policy content deterministically (no implicit fallback).

## Non-Goals
- Multi-algorithm key support.
- Threshold/multi-sig requirements.
- Network trust-anchor discovery.
- Emergency compromise workflow automation.
