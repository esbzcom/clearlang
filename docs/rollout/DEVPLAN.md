# ClearLang Development Plan

## Current Focus - Phase 17: Language Gaps + Collections
- 17.1 User-defined structs/enums with pattern matching, lowering/codegen, runtime/ABI notes, and tests. (Done)
- 17.2 Generics and trait/interface abstractions beyond built-in ADTs. (Done)
- 17.3 Real runtime semantics for `List`/`Map`/`Set` (beyond typing stubs). (Done)
- 17.4 General arrays/slices with indexing semantics and bounds checks. (Done)
- 17.5 Module/import system with visibility controls. (Done)
- 17.6 Linear-aware collections: design + typer/VC integration prototype. (Next)
- 17.7 First-class functions and closures. (Queued)

### Suggested Sequence
1) Start 17.6.1 design note for linear-aware collections (ownership model, consume/borrow rules, diagnostics).
2) Implement 17.6.2 typer + diagnostics for linear-aware collection ops.
3) Add 17.6.3 VC/effect integration with prototype tests, then revisit 17.7 scope.

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
