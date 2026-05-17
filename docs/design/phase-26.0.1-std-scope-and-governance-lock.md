# Phase 26.0.1 - Std Scope and Governance Lock

## Status
Design/governance lock for Phase 26 Std Gate A.

## Goal
Freeze a practical v1 standard-library scope for first production release and make defer/stability policy explicit.

## v1 Scope Lock

### Must-Have (first production target)
- `std::core`
- `std::str`
- `std::bytes`
- `std::int`
- `std::collections`
- `std::codec`
- `std::crypto`
- `std::host`
- `std::unit`
- `std::contract`

### Stretch / Deferred
- `std::chain::<target>`
- `std::dynamic`

## Defer List (explicit)
The following are deferred unless a launch-blocking production need is demonstrated:
1. `std::chain::<target>` package activation in the default profile.
2. Shared/dynamic std runtime linking (`std::dynamic`) in any production profile.
3. Non-baseline `std::unit` assertion expansion beyond Gate D baseline names.
4. Advanced `std::core` helpers already marked deferred in `docs/std/core.md`.

## First-Production Policy Alignment
1. Embedded std linking only for production release profile.
2. Deterministic dead-code elimination/tree-shaken symbol emission required.
3. Size-regression guardrails are required for protected release fixtures.
4. Any unresolved policy mismatch fails closed in production profile.

## Stability and Versioning Policy (pre-GA lock)
1. Compatibility surface
   - First production uses additive-only API evolution for std package method surfaces.
   - Breaking changes require a major std version bump and migration notes.
2. Deprecation policy
   - Mark deprecated APIs for at least one minor cycle before removal.
   - Production profile must keep deterministic diagnostics for deprecated usage.
3. Versioning policy
   - Std package versions are semver-governed.
   - Lockfile pinning remains the source of truth for resolved std package versions.
4. Governance gates
   - Any API addition/removal requires TODO update + docs/std package update + coverage matrix update.
   - No implicit API drift is allowed in release branches.

## Post-First-Production Roadmap Lock
Dynamic/shared std linking remains deferred until explicit activation gates are complete:
1. Trust/signature policy gate.
2. ABI compatibility gate.
3. Runtime determinism/replay gate.
4. Rollback/incident runbook gate.
5. Provenance parity gate with embedded artifacts.

## References
- `docs/TODO.md`
- `docs/std/README.md`
- `docs/design/phase-26.0.0-std-embedded-first-policy-lock.md`
