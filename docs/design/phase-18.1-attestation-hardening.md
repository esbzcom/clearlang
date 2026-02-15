# Phase 18.1 - Attestation Registry Hardening

## Status
Implemented baseline hardening for:
- signer authorization,
- key rotation controls,
- attestation revocation,
- schema version gating.

Implementation:
- `contracts/attestation/AttestationRegistry.sol`
- `contracts/attestation/test/AttestationRegistry.t.sol`

## Goals
- Ensure only authorized signers can publish attestations.
- Support operational key lifecycle changes without redeploying callers.
- Allow explicit revocation for compromised/invalid attestations.
- Version schema compatibility at the registry boundary.

## Design Summary

### Authorization and Ownership
- The deployer becomes `owner`.
- `owner` can authorize/deauthorize signers via `setSignerAuthorization`.
- `owner` can transfer ownership via `transferOwnership`.

### Schema Versioning
- Registry tracks `allowedSchemaVersions`.
- Schema version `1` is enabled at deployment.
- `owner` can toggle schema versions via `setSchemaVersion`.
- `register` rejects unsupported schema versions.

### Attestation ID Binding
- `register` validates:
  - caller authorization,
  - schema support,
  - uniqueness,
  - canonical id match.
- Canonical id:
  - `keccak256(abi.encode(address(this), payloadHash, signer, schemaVersion))`

### Revocation
- `revoke(attestationId)` allowed for:
  - the original signer, or
  - registry owner.
- Revocation is one-way; second revoke attempts fail.
- Stored attestation retains immutable content plus revocation state (`revoked`, `revokedAt`).

## Security Notes
- This slice enforces access controls and integrity checks at registry ingress.
- Additional hardening still required in Phase 18:
  - fuzzing + audit depth (`18.3`),
  - data availability and retention policy (`18.2`).

## References
- `docs/TODO.md` (`18.1`, `18.2`, `18.3`)
- `docs/rollout/attestation-production.md`
