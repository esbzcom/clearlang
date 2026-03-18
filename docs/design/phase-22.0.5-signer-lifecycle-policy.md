# Phase 22.0.5 - Signer Lifecycle Policy Expansion

## Status
Design lock target for `22.0.5` in `docs/TODO.md`.

## Goal
Expand trust-policy coverage from the Phase 20.1 bootstrap (`schema_version: 0`) to a production-oriented lifecycle model that explicitly covers:
- signer rotation controls,
- revocation lists,
- signer expiry windows,
- emergency compromise handling.

## Trust Policy Schema Compatibility
1. `schema_version: 0` remains accepted for migration.
2. `schema_version: 1` adds lifecycle and compromise fields.
3. Unsupported schema versions fail closed with deterministic `C103`.

## Schema v1 Additions
```json
{
  "schema_version": 1,
  "trusted_signers": [ ... ],
  "revoked_key_ids": ["k_old"],
  "compromised_key_ids": ["k_compromised"],
  "lifecycle": {
    "rotation_overlap_days": 30,
    "max_signer_age_days": 365,
    "compromise_response_hours": 24
  }
}
```

## Locked Behavior
1. `trusted_signers` validation remains strict and deterministic (`ed25519`, key format, RFC3339 windows).
2. `compromised_key_ids` must reference existing `trusted_signers`.
3. Effective revoked set is deterministic union:
   - `effective_revoked = revoked_key_ids ∪ compromised_key_ids`.
4. Trust gate uses `effective_revoked` as fail-closed authority:
   - any signature from an effective-revoked signer is rejected.
5. Lifecycle controls must be positive integers:
   - `rotation_overlap_days > 0`,
   - `max_signer_age_days > 0`,
   - `compromise_response_hours > 0`.

## Regression Gates
- `crates/cli/src/commands/build/strict_trust_policy.rs` unit tests lock schema v1 parsing, compromised-key handling, and lifecycle validation.
- `crates/cli/src/commands/build/strict_package_signatures.rs` unit test locks compromised signer rejection in trust gate flow.

## References
- `docs/TODO.md` (`22.0.5`)
- `docs/design/phase-20.1.0-trust-policy-v0.md`
- `docs/design/phase-22.0.6-phase22-diagnostics-reservation.md`
