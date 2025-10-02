# Codex Session Context

## 2025-10-01 - Phase 7.1 Option/Result Lowering plan
- Locked in the canonical 16-byte `{tag, payload_lo, payload_hi, reserved}` layout in `docs/design/phase-7.1-option-result-runtime.md`, including the shared R003 invalid-tag trap helper.
- Implemented `VariantInit`/`VariantLoad*` IR helpers with lowering + Wasm codegen so Option/Result constructors allocate the canonical layout and zero reserved bytes.
- Expanded Phase 7.1 in `docs/TODO.md` into spec/layout, IR/desugaring, and codegen/runtime buckets tied to provable invariants.
- Highlighted the need for a dedicated layout design slice so Option/Result lowering stays AI-friendly and mathematically checkable.
- Called out test coverage for constructors, destructors, and `Expr::Try` propagation before lifting the experimental sugar flag.

## 2025-09-27 - Phase 6.6 ADT Ergonomics follow-up
- Parser, typer, and tests cover `if let`, `??`, and postfix `?` behind the experimental flag.
- Design and typing docs refreshed: see `docs/design/phase-6.6-adt-ergonomics.md` and `docs/typing.md`.
- VC regression snapshots for the new sugar live in `crates/typer/tests/vc.rs`.

## Next Focus
- Extend destructors and sugar rewrites so lowering emits explicit tag checks and early-return paths.
- Hook `Expr::Try`/ADT sugar into the new helpers and wire invalid-tag traps through the runtime helper.
- Refresh VC snapshots/SMT encodings once the tuple `(tag, lo, hi)` representation is live in the backend.

## Quick Links
- TODO roadmap: `docs/TODO.md`
- Phase 6.6 design note: `docs/design/phase-6.6-adt-ergonomics.md`
- Typing overview: `docs/typing.md`
- Layout spec: `docs/design/phase-7.1-option-result-runtime.md`

