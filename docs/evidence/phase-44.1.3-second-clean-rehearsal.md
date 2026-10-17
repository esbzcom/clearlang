# Phase 44.1.3 - Second Clean Rehearsal

Date: 2026-10-16
Status: Passed locally; reports identical
Commit: `928e9ab3822562493abe9291ee00fdad92c0e166`

## Independent Rehearsal

A second local `git clone --no-local` was created at
`C:\Users\felto\AppData\Local\Temp\clg-m5-44.1.3-928e9ab-second`. It began clean, with no
project-local configuration or `target/` directory. The clone used the tracked LF checkout rules
for `Cargo.lock`, the canonical projection fixture, and the first report.

With `CARGO_NET_OFFLINE=true`, it executed the full command:

```text
cargo run --offline -p xtask -- pilot-precheck --report target/closure-evidence/pilot-precheck-full.json
```

The command exited `0`. Its five gates—release precheck, contract release, distributed verifier,
caller authorization/tamper, and offline target replay—all reported `passed`.

## Comparison

The first retained report is
`docs/evidence/phase-44.1.2/pilot-precheck-full.json`. The second report is retained as
`docs/evidence/phase-44.1.3/pilot-precheck-full.second.json`. Their SHA-256 values are both:

```text
fed40ee8b5054747c6416815a8324f44c585cba18096aef7635a978312d18d87
```

The reports were compared byte-for-byte and are identical. Therefore no difference required
investigation. This remains local/offline fixture evidence and does not claim live deployment or
production readiness.
