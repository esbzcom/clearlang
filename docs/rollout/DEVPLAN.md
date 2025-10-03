# ClearLang Development Plan

## Current Focus – Phase 8: Resource Types (In Design)
- Introduce `resource Name { fields ... drop { ... } }` with explicit drop blocks, enabling deterministic cleanup and proof-friendly semantics.
- Extend function signatures with the `consume` modifier (defaults borrow) and thread ownership metadata through parsing, typing, and lowering.
- Enforce linear usage at the typer level: each resource is consumed or dropped exactly once; emit targeted diagnostics for reuse or borrow after consume.
- Temporarily disallow resources inside standard `List`/`Map`/tuple aggregates and sketch follow-up designs for linear-aware collections/borrows.
- Publish a concise "Resource Guide" plus unit/integration tests covering consume flows, borrow checks, and container rejection.

## Recently Completed
- **Phase 7.3 – Option/Result rollout**: defaulted `if let`/`??`/`?` sugar, refreshed SMT encoder with canonical variant helpers, updated `--emit-vcs` docs, and tightened integration tests.
- **Phase 7.2 – Verification & SMT**: introduced the `SmtEncoder`, canonical `(tag, payload_lo, payload_hi)` modeling, and VC schema updates.
- **Phase 7.1 – IR & runtime encoding**: locked the 16-byte variant layout, trap helpers, and runtime documentation.

## Snapshot of Earlier Milestones
- Phase 6: contract syntax, VC generation, and CLI `--emit-vcs` surface.
- Phase 5: IR/Wasm pipeline became default build path.
- Phases 1–4: parser foundations, typer, collections stubs, and CLI plumbing.

## Upcoming Phases (High-Level)
- **Phase 9 – Totality & Loops**: require invariants for `while`, structural measures for recursion, and supporting diagnostics/tests.
- **Phase 10 – Refinement Types**: add `type Alias = Base where predicate` syntax, propagate refinements through typing/VCs, and extend JSON schema.

### Notes
- Historical detail from earlier phases is archived in `docs/rollout/codex-session-history.md` for reference.
- This document is intentionally concise to keep LLM context lean; see module-specific docs for deep technical notes.
