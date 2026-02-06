# ClearLang Development Plan

## Current Focus - Phase 17: Language Gaps + Collections
- 17.1 User-defined structs/enums with pattern matching, lowering/codegen, runtime/ABI notes, and tests. (Done)
- 17.2 Generics and trait/interface abstractions beyond built-in ADTs. (Done)
- 17.3 Real runtime semantics for `List`/`Map`/`Set` (beyond typing stubs). (Done)
- 17.4 General arrays/slices with indexing semantics and bounds checks. (Next)
- 17.5 Module/import system with visibility controls. (Queued)

### Suggested Sequence
1) Finish arrays/slices semantics and module system (17.4/17.5).
3) Revisit linear-aware collections and additional runtime packages as needed (17.6+).

## Recently Completed
- **Phase 11 - Proof-carrying Wasm verification**: `clg verify` CLI, proof-section hashing/signing, diagnostics, fixtures, and regression tests.
- **Phases 12-15 (per `docs/TODO.md`)**: safety/tooling hardening, DX improvements, runtime decoupling, and unsigned/bitwise expansion.

## Snapshot of Earlier Milestones
- Phase 10: refinements (VC/SMT, fixtures, diagnostics).
- Phase 9: totality & loops.
- Phase 8: resource/linear types.
- Phase 7: Option/Result lowering and proof layout.
- Phase 6: contract syntax + VC generation.
- Phase 1-5: parser/typer/IR + Wasm pipeline foundations.

## Upcoming Phases (High-Level)
- **Phase 17 - Language gaps + collections**: user-defined structs/enums, generics/traits, runtime collections, arrays/slices, module system, linear-aware collections.

### Notes
- `docs/TODO.md` is the canonical checklist; keep this file high-level.
- Historical detail from earlier phases is archived in `docs/rollout/codex-session-history.md`.
- Attestation production checklist + migration path: `docs/rollout/attestation-production.md`.
