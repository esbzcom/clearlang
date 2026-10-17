# Phase 50.1 - Clean v2 Pilot-Precheck Reproduction

Date: 2026-10-17

Status: Passed locally; this is evidence for the Milestone 5 v2 reproduction
prerequisite only. It authorizes no target action.

## Clean Checkout

The run used a newly created detached Git worktree at committed revision
`79dcc75a8d7d629acb33ed06c804101f94915464`, whose Git tree is
`e494d2bbe4041aa15949e5f9fff3d6de3839dfad`. The report output was written to a
new system temporary directory outside the worktree, leaving the worktree free
of tracked or untracked changes throughout the qualifying run.

The repository's published clean-directory fixture was invoked from that
worktree:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File docs/fixtures/phase-40.2.3/pilot-precheck-clean-directory-reproduction.ps1
```

The fixture invokes `cargo run --offline --locked -p xtask -- pilot-precheck`,
writes a report into an otherwise empty temporary directory, and validates the
report schema, `full` scope, ordered gates, passing statuses, and every locked
input/evidence digest against the digest projection. It completed successfully.

## Retained Evidence

| Artifact | SHA-256 | Purpose |
| --- | --- | --- |
| `docs/evidence/phase-50.1/pilot-precheck-full.json` | `7ae16cc8c95b7dac13265cb9a4fdc5c2038c8c58a05a9f4fc14306ce91e576ed` | Canonical full v2 pilot-precheck report. |
| `docs/evidence/phase-50.1/pilot-precheck-full.sha256` | n/a | Plain-text checksum record for the retained report. |
| `docs/evidence/phase-50.1/pilot-precheck-digest-projection.v2.json` | Tracked projection at the reproduced revision. | The locked input/evidence digest projection used by the fixture. |

The report records:

- `schema_version: 2`
- `kind: clg.pilot-precheck-report.v2`
- `scope: full`
- `overall_status: passed`
- Rust/Cargo `1.89.0`, the checked-in Windows Z3 trust material, and the
  `Cargo.lock` SHA-256
  `6a50f62920ff103f68e25f08923e932686ce188c5e8ed63d58e80a00315c9bf2`.

All five required gates passed in the locked order:

1. `release-precheck`
2. `contract-release-fixtures`
3. `verifier-distribution-fixtures`
4. `caller-authorization-fixtures`
5. `reference-pilot-offline-replay`

## Historical Comparison

Milestone 5 retained two historical **v1** reports with SHA-256
`fed40ee8b5054747c6416815a8324f44c585cba18096aef7635a978312d18d87`.
Those reports remain immutable historical evidence and were neither changed nor
reinterpreted as v2 evidence. This run is a distinct schema-v2, full-scope
report for the committed revision above; its successful locked-projection
validation resolves the v2-reproduction prerequisite stated in the Milestone 5
closure decision.

## Boundary

This local, offline, locked reproduction does not select a mainnet, approve a
signer/governance policy, submit a transaction, certify an RPC or chain, or
make a production-profile claim. Phase 50.2 remains the next task and must lock
one named mainnet before any later target work.
