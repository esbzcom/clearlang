# Phase 18.2 - Attestation Data Availability Policy

## Status
Policy lock for Phase `18.2` (pinning, backup, retention, and restore drills).

## Goals
- Ensure attestation payloads remain retrievable and verifiable over time.
- Prevent single-provider or single-region data loss from breaking verification.
- Define explicit retention and restore objectives with auditable evidence.

## Scope
- Off-chain attestation payload availability policy.
- Backup/restore process for payload blobs and index metadata.
- Operational drill requirements and success criteria.

Out of scope:
- On-chain contract security logic (covered by 18.1/18.3).
- Build artifact/package resolution for compiled imports (18.4).

## DA Architecture Baseline

### Canonical Invariants
1. Every registry entry must map to at least one payload object retrievable by content hash.
2. Verification must reject payload bytes when `sha256(payload_bytes) != payload_hash`.
3. At least two independent storage backends must hold each payload.

### Storage Topology
- Primary: IPFS pinning provider A.
- Secondary: IPFS pinning provider B (or self-hosted IPFS cluster in a separate region/account boundary).
- Tertiary backup store: object storage archive (immutable bucket, versioned, encrypted at rest).

### Metadata Index
- Maintain a signed index record per attestation:
  - `attestation_id`
  - `payload_hash`
  - `schema_version`
  - `uri`
  - `pin_status` per backend
  - `first_seen_at`, `last_verified_at`
- Index snapshots are backed up with payload archives.

## Backup Policy

### Frequency
- Incremental sync: every 6 hours.
- Full snapshot: daily.
- Monthly immutable checkpoint: one per calendar month.

### Backup Artifacts
- Raw payload blobs keyed by `payload_hash`.
- Index database export (JSONL or SQL dump).
- Drill logs and verification reports.

### Integrity Checks
- On each backup cycle:
  - sample at least 5% of changed payloads (minimum 20) and verify hash match,
  - verify index entry count consistency against on-chain registry delta for the period.

## Retention Policy

### Classes
1. `release-critical` (production release attestations):
   - retain indefinitely (or minimum 7 years if legal policy requires bounded retention).
2. `compliance-critical`:
   - retain minimum 5 years.
3. `dev/test`:
   - retain 180 days, then eligible for pruning.

### Pruning Rules
- Only `dev/test` class can be pruned.
- Pruning requires:
  - no regulatory hold,
  - index tombstone record,
  - signed prune report with operator + timestamp.

## Restore Objectives
- RPO (Recovery Point Objective): <= 6 hours.
- RTO (Recovery Time Objective): <= 4 hours for release-critical datasets.

## Restore Drill Policy
- Frequency: monthly.
- Drill type: restore from backup into clean environment, then verify payload integrity and random sample replay.
- Success criteria:
  - restore completes within RTO,
  - recovered data loss within RPO bound,
  - 100% hash match for sampled payloads,
  - index-to-payload linkage consistency checks pass.

## Alerting and Escalation
- Alert if any release-critical payload is unavailable on both primary and secondary backends for > 15 minutes.
- Alert if backup job fails twice consecutively.
- Escalate to incident process if restore drill fails or RTO/RPO thresholds are exceeded.

## Evidence Required for Release
- Latest successful monthly restore drill report.
- Latest daily backup verification report.
- Pinning status report for all release-critical attestations in the release window.

## References
- `docs/TODO.md` (`18.2`, `18.0.2`)
- `docs/design/phase-18.1-attestation-hardening.md`
- `docs/design/phase-18.2-attestation-data-availability.md`
