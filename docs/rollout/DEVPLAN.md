# ClearLang Development Plan

## Current Focus - Phase 17: Language Gaps + Collections
- 17.1 User-defined structs/enums with pattern matching, lowering/codegen, runtime/ABI notes, and tests. (Done)
- 17.2 Generics and trait/interface abstractions beyond built-in ADTs. (Done)
- 17.3 Real runtime semantics for `List`/`Map`/`Set` (beyond typing stubs). (Done)
- 17.4 General arrays/slices with indexing semantics and bounds checks. (Done)
- 17.5 Module/import system with visibility controls. (Done)
- 17.6 Linear-aware collections: design + typer/VC integration prototype. (Done)
- 17.7 First-class functions and closures. (Done)

### Suggested Sequence
1) Complete 17.7.1 design lock for function types, capture semantics, effect compatibility, and parser/typer acceptance boundaries. (Done; see `docs/design/phase-17.7-closures.md`.)
   - Locked decisions include closure ABI shape (`{ code_id, env_ptr }`), wrapper-based dispatch (no `call_indirect` in Phase 17), no capture-list syntax, inferred lambda return types, no closure-env deallocation in v1, and no self-referential closure values in v1.
2) Implement 17.7.2 parser/AST support for lambdas and function-type syntax. (Done)
3) Implement 17.7.3 typer checks (capture/effect compatibility, closure-call typing). (Done)
   - Completed closure typing/capture checks, explicit self/mutual closure recursion diagnostics, and conservative effect checking for function-value calls.
4) Implement 17.7.4 lowering/codegen closure environment + dispatch wrappers. (Done)
5) Finish 17.7.5 examples/docs pass. (Done)

## Next Focus
- 17.8 Traits follow-ups (default bodies, optional explicit impl selection syntax if needed, and name-shortening ergonomics).

### Next Execution Slice (Prepared)
1) Complete 17.8.1.0 design lock for trait default methods, including README design-principles check (`simple for users`, `AI-friendly`, `provably correct`, `crypto-focused`) and acceptance boundaries. (Done; see `docs/design/phase-17.8-trait-defaults.md`.)
2) Implement 17.8.1.1 parser/AST support for trait methods in either declaration form (`...;`) or default-body form (`... { ... }`) with deterministic diagnostics.
3) Continue with 17.8.1.2 and 17.8.1.3 typer/effect enforcement once parser shape is stable.

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
