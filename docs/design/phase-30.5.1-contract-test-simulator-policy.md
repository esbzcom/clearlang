# Phase 30.5.1 - Contract Test Simulator Policy

Date: 2026-08-09
Status: Locked
Owner: developer-experience-owner

Target-aware contract tests must execute through `clg simulate`, never through an ambient target
or RPC endpoint. A contract test case supplies explicit source, transition/init selection, state,
arguments, caller/value/block context, fuel and memory limits, and an optional deterministic
campaign seed. Its report captures the canonical simulator trace and a replay argv that contains
every such input.

`clg contract test <SOURCE> --plan <FILE> --report-out <FILE> --trace-dir <DIR>` implements the
campaign runner. `clg.contract-test-plan.v1` requires a seed, case count, transition, initial
state, caller/value/block/fuel/memory context, and one ordered generator per transition argument.
The first schema supports `bool`, bounded `u8`, bounded `u64`, and bounded `int` generators.

The runner uses a specified xorshift stream (including a fixed non-zero normalization for seed
zero), writes canonical generated argument files and simulator traces per case, and emits
`clg.contract-test-report.v1`. Every report contains its canonical plan digest, seed, generated
arguments, trace paths, and a complete `clg simulate` replay argv. A failed or unsupported
simulator execution is a campaign failure and cannot fall back to Wasm or network execution.

Plans may add ordered `result_equals_arg` and `state_field_equals_arg` properties. These compare
the canonical simulator trace result or final state with a generated argument and fail the
campaign on the first violation.

## References

- `docs/design/phase-30.3.2-deterministic-local-simulator-lock.md`
- `docs/design/phase-30.5.0-contract-runtime-lifecycle-limits.md`
- `docs/testing.md`
