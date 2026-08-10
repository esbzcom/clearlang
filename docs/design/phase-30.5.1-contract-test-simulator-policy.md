# Phase 30.5.1 - Contract Test Simulator Policy

Date: 2026-08-09
Status: Locked
Owner: developer-experience-owner

Target-aware contract tests must execute through `clg simulate`, never through an ambient target
or RPC endpoint. A contract test case supplies explicit source, transition/init selection, state,
arguments, caller/value/block context, fuel and memory limits, and an optional deterministic
campaign seed. Its report captures the canonical simulator trace and a replay argv that contains
every such input.

Property and fuzz campaigns are deferred until the test-plan schema gains explicit generator and
seed fields. They must not be approximated with nondeterministic host randomness. A failed or
unsupported simulator execution is a test failure and cannot fall back to Wasm or network
execution.

## References

- `docs/design/phase-30.3.2-deterministic-local-simulator-lock.md`
- `docs/design/phase-30.5.0-contract-runtime-lifecycle-limits.md`
- `docs/testing.md`
