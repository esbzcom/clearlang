# Phase 40.2.2 - Local Evidence Compatibility and Rollback Lock

Date: 2026-10-15
Status: Locked
Owner: release-assurance-owner

## Purpose

Define how an operator treats a changed local toolchain, solver, host, target/verifier format, or
pilot-precheck report. The rule is deliberately narrow: accepted evidence is reproduced under an
explicit compatible contract, or it is rejected with an actionable deterministic explanation. It
is never silently reinterpreted, upgraded in place, or repaired by a remote system.

This lock applies to the bounded Milestone 5 pilot only. It does not define production deployment,
mainnet migration, automatic update, dependency resolution, or a general evidence conversion API.

## Evidence identity and immutability

An accepted local evidence set consists of the canonical
`clg.pilot-precheck-report.v1` report, the exact checkout revision and `Cargo.lock` from which it
was generated, its named input/evidence digests, and the retained release bundle/verifier evidence
named by the report's fixture outputs. The report is retained byte-for-byte; it is not edited to
change a version, digest, status, scope, gate order, or path after execution.

Compatibility is evaluated against the exact set in
`phase-40.2.1-local-toolchain-support-lock.md`: Rust/Cargo `1.89.0`, a listed x86_64 host,
trusted Z3 `4.16.0`, the locked pilot target/execution profiles, verifier capability line, and v1
bundle, verifier-result, and pilot-precheck schemas. A version-like name, matching extension,
similar host, or compatible-looking JSON field does not extend this set.

## Decision table

| Observed change or condition | Deterministic decision | Required operator action |
| --- | --- | --- |
| Every identity matches the 40.2.1 lock and the report validates as `full` / `passed` | Accept the evidence for the bounded pilot | Retain the canonical report and referenced fixture evidence unchanged. |
| Rust/Cargo, host, native linker, Cargo lock, solver bytes/signature, or solver profile changed | Reproduce; do not reuse the old acceptance report | Use the exact locked environment or produce a new report from a separately approved compatibility lock. |
| Host is unlisted, architecture is not x86_64, or the bundled solver is absent/untrusted/wrong version | Reject | Move to a listed host and restore the checked-in trusted solver bundle. Do not use emulation, a container, a host-PATH solver, or a hosted build. |
| Target profile, execution profile, release-bundle format/schema, verifier result schema, or precheck format/schema is unknown or differs | Reject | Use an artifact whose signed, explicit capability declaration names the exact required v1 contract; otherwise stop and escalate. |
| Report has an unknown/reordered/missing gate, non-empty future gates, non-passing gate, malformed digest, or does not validate against the checkout | Reject | Re-run the one-command local acceptance path from the approved checkout; never hand-edit or merge reports. |
| Remote automation reports pass while local acceptance fails or cannot run | Reject | Resolve the local failure. Remote output is informational and has no acceptance authority. |
| A replacement lock explicitly approves a new compatible toolchain or schema | Reproduce under the new lock | Preserve the prior evidence, run the full local precheck anew, and record the replacement-lock revision beside the new report. |

The required diagnostic is the decision and the exact mismatched identity from this table—for
example `unsupported host: linux-aarch64`, `solver version mismatch: expected 4.16.0`, or
`pilot-precheck schema mismatch: expected clg.pilot-precheck-report.v1 schema 1`. Diagnostics
must be deterministic, non-secret, and must not contain absolute paths, hostnames, credentials,
or values from a network endpoint. They must direct an operator to one of the listed actions;
falling back to a nearby version is not an action.

## Upgrade procedure

An upgrade begins with a new committed compatibility lock. It must name every changed identity,
the new supported matrix, the exact solver and trust material, and each accepted schema/profile.
It must also include a local fixture proving that the new command either reproduces the required
evidence projection or fails closed. The existing report is historical evidence; it neither
becomes a report for the new lock nor proves the new environment.

Until such a lock and fixture exist, changing `rust-toolchain.toml`, `Cargo.lock`, solver bundle,
target profile, verifier capability, or any v1 schema is an incompatibility, not a routine patch
upgrade. The approved response is to retain the old evidence and reject the new acceptance attempt.

## Rollback procedure

Rollback means returning to a specifically identified prior checkout revision and its complete
compatibility inputs: `rust-toolchain.toml`, `Cargo.lock`, trusted solver executable and sidecars,
the applicable compatibility lock, and the named verifier artifact. The operator then reruns the
one-command offline precheck and retains the newly generated report alongside the historical one.

It does not mean restoring an unversioned `latest` tool, changing a report's fields, deleting a
failed report, copying a solver from `PATH`, or asking remote CI to bless an older result. A
suspected verifier compromise or withdrawal follows the stricter explicit-artifact procedure in
Phase 43.0.3; a withdrawn artifact is never silently selected as the rollback target.

## Remote-automation boundary

Remote automation may detect or display an incompatibility, but cannot decide it away. It has no
authority to modify the lock, generate pilot acceptance evidence, substitute a missing local
toolchain, or certify a report that fails local validation. Every approval, reproduction, and
rollback decision remains local and explicit.

## Design Principles Check

- **Simple for users:** every mismatch has one bounded response: use the exact lock, reproduce
  under a new lock, or stop.
- **AI-friendly:** the decision table rules out nearest-version guesses and report editing.
- **Provably correct:** identity-bound, canonical evidence is retained and revalidated instead of
  being reinterpreted.
- **Crypto-focused:** solver and verifier trust material remain explicit, versioned inputs with no
  ambient or network fallback.

## References

- `docs/design/phase-40.2.1-local-toolchain-support-lock.md`
- `docs/design/phase-40.1.1-pilot-precheck-cli-report-lock.md`
- `docs/design/phase-25.1.16-solver-supply-chain.lock.json`
- `docs/design/phase-43.0.2-verifier-compatibility-lock.md`
- `docs/design/phase-43.0.3-verifier-lifecycle-and-rollback-lock.md`
