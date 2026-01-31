# Codex Session Context

## 2025-10-01 - Phase 7.1 Option/Result Lowering plan
- Locked in the canonical 16-byte `{tag, payload_lo, payload_hi, reserved}` layout in `docs/design/phase-7.1-option-result-runtime.md`, including the shared R003 invalid-tag trap helper.
- Implemented `VariantInit`/`VariantLoad*` IR helpers with lowering + Wasm codegen so Option/Result constructors allocate the canonical layout and zero reserved bytes.
- Wired the invalid-tag runtime helper (R003) through `VariantLoadTag`, so destructors trap when tags fall outside {0,1}.
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

## 2025-10-02 - Phase 7.3 wrap-up & Phase 8 planning
- Defaulted ADT sugar (`if let`/`??`/`?`) and refreshed SMT encoding with canonical variant helpers.
- Slimmed `docs/rollout/DEVPLAN.md` to a lean overview and captured Phase 8 resource plans (struct-with-drop + `consume`).
- Updated `docs/TODO.md` to track the new resource milestones (parser/typer changes, diagnostics, test plan).
- Next up: implement resource parsing, linear tracking, and the `consume` signature modifier.

## 2026-01-03 - Phase 10.3 refinement completion
- Implemented refinement-preserving shadowing checks so rebinding a refined value to a weaker type raises T705 (covers loop bodies and general blocks).
- Defined VC pre/post ordering for refinement obligations and added ordering regression tests in `crates/typer/tests/vc.rs`.
- Added loop/refinement regression coverage (invariant accepts refined binders; loop body cannot drop refinements) plus a shadowing-loss test.
- Updated `docs/typing.md` with refinement preservation rules, interaction matrix, VC tie-in, and the "no escape hatch" decision.
- Marked Phase 10.3 feature interactions, docs, and migration decisions complete in `docs/TODO.md`.

## 2026-01-19 - Phase 11 focus alignment
- Marked Phase 10.4/10.5 as complete in `docs/TODO.md` and shifted the roadmap focus to Phase 11 verification.
- Normalized verifier CLI naming to `clg verify` in the TODO roadmap.
- Updated rollout planning to target proof-carrying verification as the active phase.

## 2026-01-30 - Phase 11 completion + Phase 16 kickoff
- Verified Phase 11 deliverables are complete (proof section hashing/signing, `clg verify`, diagnostics, and regression tests).
- Confirmed signing/verifying docs and runnable fixtures are present and referenced from the main checklist.
- Updated rollout planning to shift the active focus to Phase 16 crypto intrinsics.

## Next Focus
- Implement Phase 16 crypto intrinsics (hashes/HMAC, signature verification, constant-time compare) with deterministic semantics.
- Define error/diagnostic behavior for crypto intrinsics and add SMT encoding or explicit axioms.
- Document proof limitations for crypto primitives and design on-chain attestation.

## 2026-01-31 - Phase 16 completion + attestation reference
- Completed Phase 16 crypto intrinsics + proofs: deterministic runtime semantics, diagnostics, SMT/VC helpers, and JSON error stability tests.
- Documented crypto proof limitations and migration options in `docs/proofs/crypto-limitations.md`.
- Added Phase 16.7 attestation reference design and minimal registry contract with sample payload and workflow docs.
- Updated the rollout plan and TODO checklist to mark Phase 16 complete and add Phase 18 production hardening items.

