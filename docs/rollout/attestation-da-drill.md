# Attestation Data Availability - Backup/Restore Drill Runbook

This runbook operationalizes Phase 18.2 data-availability requirements.

## Frequency
- Run monthly (recommended first business week).
- Run ad-hoc after major storage/pinning topology changes.

## Preconditions
- Access to:
  - primary and secondary pinning providers,
  - backup archive storage,
  - on-chain attestation registry query endpoint.
- Selected drill scope:
  - at least 100 release-critical attestations or all attestations from last 30 days (whichever is larger).

## Drill Steps

1. Snapshot selection
- Export attestation index slice for drill scope:
  - `attestation_id`, `payload_hash`, `uri`, `schema_version`, `class`.
- Save snapshot manifest with timestamp.

2. Simulated failure setup
- In a clean restore environment, block access to primary pinning source.
- Restore only from backup archive + secondary pin source.

3. Restore execution
- Restore index data and payload blobs.
- Rebuild lookup tables keyed by `payload_hash` and `attestation_id`.
- Record start/end timestamps for RTO measurement.

4. Integrity verification
- For each restored payload in drill scope:
  - compute `sha256(payload_bytes)`,
  - assert exact match with recorded `payload_hash`.
- Query on-chain registry for same scope and compare:
  - `attestation_id`,
  - `payload_hash`,
  - `schema_version` (if tracked in index metadata).

5. Availability verification
- Confirm each payload has at least one retrievable URI in restored environment.
- Confirm release-critical class has dual-backend availability after restore.

6. Report generation
- Produce signed drill report including:
  - drill scope size,
  - recovered count,
  - hash mismatch count,
  - missing payload count,
  - measured RPO and RTO,
  - pass/fail result and remediation actions.

## Pass/Fail Criteria
- `PASS` only if all are true:
  - RTO <= 4 hours.
  - Effective data loss <= 6 hours (RPO).
  - 0 hash mismatches in drill scope.
  - 0 missing release-critical payloads.
  - Registry/index consistency checks pass.
- Otherwise `FAIL`.

## Failure Handling
- Open incident ticket immediately.
- Freeze new release attestations if release-critical payload availability is impacted.
- Execute remediation and rerun drill within 48 hours.

## Evidence Artifacts
- Drill manifest (scope input).
- Restore logs with timing.
- Integrity verification output.
- Registry reconciliation output.
- Signed drill report.

## Recommended Report Template
- `drill_id`
- `run_at`
- `operator`
- `scope_count`
- `recovered_count`
- `hash_mismatch_count`
- `missing_payload_count`
- `rpo_hours`
- `rto_hours`
- `status`
- `notes/remediation`
