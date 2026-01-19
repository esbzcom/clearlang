# ClearLang Development Plan

## Current Focus - Phase 11: Proof-Carrying Wasm Verification
- Build `clg verify`: parse Wasm, extract `clearlang.proof` + signature sections, validate hashes/signatures, and emit structured diagnostics.
- Add end-to-end docs and runnable fixtures for signing/verifying flows (pass/fail expectations).
- Close refinement UX debt: document T701-T708, make fixtures runnable, and add shallow linear normalization for `n + k`/`n - k`.

## Recently Completed
- **Phase 10.5 - Refinement tests**: positive/negative coverage plus VC snapshots.
- **Phase 10.4 - Refinement VC/SMT**: schema extensions, SMT encoding, and fixtures/docs updates.
- **Phase 9 - Totality & loops**: invariants, variants, and totality enforcement with diagnostics.

## Snapshot of Earlier Milestones
- Phase 8: resource types, linear tracking, and docs/tests.
- Phase 7: Option/Result lowering, SMT, and proof layout.
- Phase 6: contract syntax, VC generation, and CLI `--emit-vcs` surface.
- Phase 5: IR/Wasm pipeline became default build path.
- Phases 1-4: parser foundations, typer, collections stubs, and CLI plumbing.

## Upcoming Phases (High-Level)
- **Phase 12 - Safety & Tooling Hardening**: CI validation, runtime bounds, contracts mode design.
- **Phase 13 - Developer Experience**: tracing/logging, tooling ergonomics, error hygiene, perf, release profile tuning.
- **Phase 14 - Enhancements**: WASI `print`, on-chain attestation for signatures.

### Notes
- Historical detail from earlier phases is archived in `docs/rollout/codex-session-history.md` for reference.
- This document is intentionally concise to keep LLM context lean; see module-specific docs for deep technical notes.
