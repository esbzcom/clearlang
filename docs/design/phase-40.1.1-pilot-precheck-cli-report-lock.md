# Phase 40.1.1 - Local Pilot-Precheck CLI and Report Lock

Date: 2026-10-13
Status: Locked
Owner: release-assurance-owner

## Purpose

Define the one-command, local-only Milestone 5 acceptance interface. The command aggregates
evidence; it does not create a second release policy or contact a network. The implemented command
uses the full scope so every required local fixture must pass before pilot acceptance can be
claimed.

## Command Contract

The supported invocation is:

```powershell
cargo run -p xtask -- pilot-precheck --report <FILE>
```

`--report <FILE>` is required. It identifies the destination only and is never serialized into the
report. The command creates the parent directory when it is safe to do so, writes the final report
atomically, and exits non-zero for any failed, omitted, malformed, substituted, or out-of-order
gate. It accepts no option that skips, reorders, retries, or replaces a required gate.

The command is local-only. It must not discover or contact RPC endpoints, wallets, testnets,
hosted CI, or any other network service. A live testnet observation is verified later from its
recorded offline fixture, not created by this command.

## Staged Gate Order

The full implementation has exactly these canonical identifiers and order:

1. `release-precheck`
2. `contract-release-fixtures`

3. `verifier-distribution-fixtures`
4. `caller-authorization-fixtures`
5. `reference-pilot-offline-replay`

It writes `scope: "full"`, and every listed gate is mandatory. It must fail if a gate has not
been registered with a stable, local invocation and expected evidence inputs.
It must not replace an unavailable gate with a warning, a skipped result, a smoke test, an ambient
artifact, or a live target request.

`release-precheck` retains its existing fail-closed command. The contract-release, verifier,
caller-authorization, and replay gates run their named local fixture suites. The completed full
report has an empty `future_required_gates` list.

## Report Schema

The report format is `clg.pilot-precheck-report.v1` with `schema_version: 1`. Its canonical JSON
object fields appear in this order:

1. `schema_version` - integer `1`.
2. `kind` - exact string `clg.pilot-precheck-report.v1`.
3. `scope` - exact string `full`.
4. `overall_status` - `passed` or `failed` for the declared scope only.
5. `toolchain` - ordered, non-secret tool/version identifiers used by the gate.
6. `gate_order` - the exact executed-scope identifier sequence.
7. `future_required_gates` - empty for the completed full scope.
8. `gates` - one ordered entry per executed-scope gate.

Each `gates` entry contains only:

- `id`: one required gate identifier;
- `status`: `passed`, `failed`, or `not_run`;
- `command`: canonical argv tokens without an absolute path, credentials, or environment values;
- `input_digests`: ordered artifact identifiers and SHA-256 digests supplied or verified by the
  gate;
- `evidence_digests`: ordered artifact identifiers and SHA-256 digests emitted or checked by the
  gate; and
- `failure_code`: a stable non-secret error code when `status` is `failed`, otherwise omitted.

No report field may contain an absolute path, current directory, hostname, username, timestamp,
duration, process identifier, endpoint, key, signature secret, nonce, fee, or environment value.
The report has no self-digest; its caller may hash the finished bytes. Canonical serialization and
the fixed gate order make identical toolchain/input/result runs byte-stable.

On failure, the command writes one complete report showing the failed gate and each subsequent
gate as `not_run`, then exits non-zero. A `not_run` status necessarily makes `overall_status`
`failed`; it is evidence of fail-fast aggregation, never success.

## Validation and Redaction Rules

Before reporting success for its declared scope, the implementation must reject:

1. missing, duplicate, unknown, or reordered gate identifiers;
2. an omitted required gate, non-empty `future_required_gates`, `not_run` result, or successful
   result without required evidence digests;
3. unsafe evidence paths, path traversal, absolute paths, or evidence outside the designated
   fixture/output root;
4. malformed schema versions, non-canonical JSON, non-lowercase 64-character SHA-256 digests,
   substituted input/evidence digests, or a report containing forbidden host/secret fields;
5. attempts to execute a gate through a network-target command or to retry a failed gate.

## Documentation Contract

The full command is the local Milestone 5 acceptance path. It remains a local fixture gate, not
a live target action, remote-CI certification, mainnet claim, or production authorization.

When implemented, the following documents must agree on its spelling, local-only behavior, report
schema, and required fixture suite: the Milestone 5 roadmap, pilot product contract, local
toolchain policy, verifier walkthrough, and pilot operations guide.

## Design Principles Check

- **Simple for users:** one explicit report path and one fixed command replace a fragile manual
  checklist.
- **AI-friendly:** stable identifiers, fixed ordering, versioned JSON, and redacted canonical
  values are easy for automation to produce and validate.
- **Provably correct:** missing evidence or an unavailable prerequisite cannot be upgraded to a
  full passing result.
- **Crypto-focused:** the command keeps verifier, caller, source, and target evidence distinct
  and prevents secrets or live network behavior from contaminating reproducible acceptance.

## Exit Criteria

1. The Milestone 5 roadmap marks `40.1.1` and `40.1.4` complete after the authorization,
   reference-pilot, and verifier fixture suites are registered in the fixed full order.
2. The command interface, report schema, staged gate order, failure behavior, and redaction
   policy are explicit.
3. A passing full report is impossible when any required gate is absent or non-passing, and the
   command prohibits remote/network dependence.

## References

- `docs/todo/milestone_5_roadmap.md`
- `docs/design/phase-40.0-milestone-5-pilot-product-lock.md`
- `docs/release-process.md`
- `docs/testing.md`
