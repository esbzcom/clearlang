# Attestation: Production Readiness Notes

This document explains what is required beyond the Phase 16.7 "minimal reference implementation" and how to migrate from the reference into a production-ready attestation system.

## Phase 18.1 Status Update

Baseline registry hardening has been implemented:
- signer authorization and owner controls,
- signer key rotation/deauthorization controls,
- attestation revocation,
- schema-version allowlist checks,
- canonical `attestation_id` validation against `(registry, payload_hash, signer, schema_version)`.
- input hardening for payload hash and URI bounds.

Baseline security-review and fuzzing slice has also landed:
- contract property/fuzz tests for authorization, schema gating, ID mismatch, and revocation authorization,
- payload envelope validation checks with negative unit coverage.

Remaining production work is still required (notably fuzzing/audit depth and DA operations policy).

## What "Minimal Reference Implementation" Means

The reference implementation is a small, auditable prototype that proves the end-to-end design:

- A basic on-chain registry contract with a minimal data model.
- A consistent payload format (hash + metadata + pointer).
- A happy-path verification flow and sample tooling.

It is **not** production-hardened. It intentionally omits operational controls, key management, and defense-in-depth.

## Production Readiness Checklist

### Security and Trust Model

- Explicit threat model: replay protection, signer authorization, domain separation, and chain ID binding.
- Key rotation and revocation strategy (multi-sig, threshold signing, or delegated keys).
- Clear definition of what a "valid attestation" implies and what it does not.

### Contract Hardening

- Access control and admin roles (or explicit immutability with a migration plan).
- Event logs designed for indexing and monitoring.
- Gas-aware storage layout, bounded input sizes, and replay-safe identifiers.
- Audit and fuzzing coverage for registry correctness and signature handling.

### Data Availability

- Verified content addressing (hash must match payload CID).
- Pinning strategy, backups, or fallback availability for off-chain data.
- Governance policy for pruning/retention if registry size grows.

### Operational Controls

- Monitoring, alerting, and incident response procedures.
- CI verification of attestation payloads and schema versions.
- Documented rollback or deprecation process for payload formats.

## Migration Path: Reference -> Production

Reminder: production hardening work is expected to land in a later phase (target: Phase 18) after the Phase 16.7 reference implementation is validated.

1) **Version the schema now**
   - Add explicit version fields in the payload and registry storage.
   - Make verification reject unknown versions by default.

2) **Introduce authorization**
   - Add a signer allowlist or role-based permissions.
   - Require signatures to bind to chain ID and registry address.

3) **Add revocation and rotation**
   - Support key rotation with audit trails.
   - Add revocation entries and a policy for superseding attestations.

4) **Operationalize data availability**
   - Enforce `hash(payload) == declared_hash`.
   - Pin IPFS or mirror payloads with uptime guarantees.

5) **Harden and audit**
   - Conduct a security audit of the contract.
   - Add fuzzing for serialization and signature parsing edge cases.

## Notes on Reuse

The reference implementation **is not wasted**. It should be treated as a scaffold:

- Keep the core schema and verification flow.
- Extend it with authorization, versioning, and hardening.
- Use it as the baseline for production audits and formal review.
