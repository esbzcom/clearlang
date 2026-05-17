# Phase 26.0.0 - Std Embedded-First Policy Lock

## Status
Design lock for Phase 26 standard-library productionization.

## Goal
Ship a production-usable std surface quickly for real projects while preserving deterministic, audit-grade release behavior.

## Policy (Locked)
1. First production release uses **embedded std linking** only.
   - Std symbols are compiled into the target artifact.
   - Release profile does not depend on runtime dynamic std loading.
2. Release builds must use deterministic **dead-code elimination**.
   - Emit only reachable std symbols from the resolved module graph.
   - Full-package embedding is not allowed for production release profile.
3. Dynamic/shared std linking is **deferred**.
   - Dynamic runtime std package loading is not enabled in the first production release.
   - Future activation requires explicit follow-up phase gates.

## Rationale
- Simple for users: one artifact path for first production release, no runtime std deployment coordination.
- AI-friendly: predictable symbol reachability and deterministic diagnostics for unused/unsupported std surfaces.
- Provably correct: proof/release gates evaluate one closed artifact boundary.
- Crypto-focused: reduced runtime trust surface and deterministic replay of release evidence.

## First-Release Implementation Requirements
1. Link mode gate
   - `--release-profile production` enforces embedded std mode.
   - Any dynamic std runtime-link request in production profile fails closed with deterministic diagnostics.
2. Reachability + DCE gate
   - Only used std symbols appear in final artifact/import map.
   - CI regression guard checks std symbol count/size trend on protected fixtures.
3. Determinism gate
   - Identical source + lock/trust inputs produce identical embedded std symbol set and artifact hashes.

## Deferred Dynamic Blueprint (Post-First-Production)
Future dynamic std linking can be enabled only after a dedicated follow-up phase that includes:
1. Strict trust/signature policy for shared std artifacts.
2. Deterministic runtime loader diagnostics and replay guarantees.
3. ABI compatibility/version negotiation policy with fail-closed behavior.
4. Cross-platform reproducibility and rollback runbooks for shared std packages.
5. Provenance and tamper-evidence parity with embedded release artifacts.

## Non-Goals (This Lock)
- No dynamic std runtime loading in first production release.
- No compatibility commitment for dynamic std UX before follow-up phase gates are complete.

## References
- `docs/TODO.md`
- `docs/design/phase-20.0-std-packaging-runtime-linking.md`
- `docs/release-process.md`
