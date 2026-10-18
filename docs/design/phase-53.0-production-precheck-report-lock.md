# Phase 53.0 - Production Precheck and Candidate Report Lock

Date: 2026-10-17

Status: Locked local candidate-evidence contract. It authorizes no endpoint
contact, signing, deployment, invocation, Safe operation, or mainnet claim.

## Command

```powershell
cargo run --offline --locked -p xtask -- production-precheck --report <FILE>
```

`--report` is required and only names a new output destination. The command
accepts no skip, reorder, retry, endpoint, key, wallet, target, transaction,
or approval argument. It requires a clean committed checkout, refuses to
overwrite, creates a safe output parent when needed, writes atomically, and
exits non-zero unless every required Gate D input passes. Every nested Cargo
invocation uses `--offline --locked`; no gate may open a network connection.

## Fixed Gate Order

The report uses exactly this order:

1. `release-identity`
2. `safe-operator-policy`
3. `safe-evidence-anchor-replay`
4. `independent-observation-policy`
5. `public-testnet-soak-decision`
6. `production-support-policy`

An unavailable prerequisite produces `blocked`; a malformed, substituted, or
failed prerequisite produces `failed`; later gates are `not_run`. No warning,
ambient checkout state, testnet request, or a Gate E review/launch record can
replace a gate. The current public-testnet-soak decision is `not_started`, so
the complete precheck is expected to remain blocked until the separately
authorized Phase 52.5 execution has completed.

## Canonical Report

The report format is `clg.production-precheck-report.v1`, schema version `1`.
Its exact fields, in canonical JSON order, are:

1. `schema_version`
2. `kind`
3. `scope` (`candidate_evidence`)
4. `overall_status` (`passed`, `failed`, or `blocked`)
5. `candidate_profile` (the exact Phase 50.2 profile and chain identity)
6. `identity` (ordered, secret-free digest bindings for profile, policy,
   release, evidence, anchor, replay, and support matrix)
7. `toolchain` (ordered local tool identities)
8. `gate_order`
9. `gates`
10. `gate_e` (always `not_run` in this report)

Every gate entry contains `id`, `status`, `command`, `input_digests`,
`evidence_digests`, and optional non-secret `diagnostic`. It rejects unknown,
missing, duplicate, or reordered fields/gates; unsafe paths; noncanonical JSON;
unrecognized versions; invalid digest syntax; credentials; keys; endpoint data;
absolute paths; host/user/process/time data; and a success status with missing
evidence. Gate E contains only `independent_review: not_run` and
`launch_decision: not_run`; it is never inferred.

## Status and Diagnostics

`passed` means the named local candidate-evidence gate completed. `failed`
means a required artifact or comparison was invalid. `blocked` means an
explicit prerequisite is not yet available. `not_run` is valid only for a
later fail-fast gate or the two Gate E fields. A report with any `failed`,
`blocked`, or `not_run` gate cannot have `overall_status: passed`.

Stable initial diagnostics are `M6D001` invalid report/schema or order,
`M6D002` missing/unsupported profile or policy identity, `M6D003` release,
anchor, or replay mismatch, `M6D004` unsupported support matrix, and `M6D005`
public-testnet soak not available or not eligible. These are local reporting
diagnostics, not target or production statuses.
