# Phase 44.1.2 - Full Local Evidence Chain

Date: 2026-10-16
Status: Passed locally
Commit: `118fd2c3922b0ac71272c3559c23f59c0df6254d`

## Clean Clone

The evidence was generated from a new `git clone --no-local` of the exact commit above at
`C:\Users\felto\AppData\Local\Temp\clg-m5-44.1.2-118fd2c-rerun`. Before validation,
the clone had no source changes, no untracked files, no project-local configuration, and no
pre-existing `target/` directory. `Cargo.lock` and the canonical digest projection were checked
out as LF bytes under the tracked `.gitattributes` policy.

With `CARGO_NET_OFFLINE=true`, the clone executed:

```text
cargo run --offline -p xtask -- pilot-precheck --report target/closure-evidence/pilot-precheck-full.json
```

It exited `0`. The retained canonical report is
`docs/evidence/phase-44.1.2/pilot-precheck-full.json`; its SHA-256 is recorded in the adjacent
`.sha256` file as `fed40ee8b5054747c6416815a8324f44c585cba18096aef7635a978312d18d87`.

## Results

The report records all required gates as `passed`, in order:

1. `release-precheck`
2. `contract-release-fixtures` — reference release and independent verifier coverage
3. `verifier-distribution-fixtures`
4. `caller-authorization-fixtures` — authorization and tamper coverage
5. `reference-pilot-offline-replay` — offline target replay

This is local, offline fixture evidence only. It does not authorize a live target transaction,
use private credentials, assert production deployment readiness, or replace the independent
second rehearsal required by 44.1.3.
