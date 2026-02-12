# Codex Session Context

## 2026-02-12 - Phase 17.7.5 closure tests/docs completion
- Completed 17.7.5 and closed Phase 17.7 in `docs/TODO.md`.
- Added runtime regression coverage for dispatcher unknown-`code_id` trap path in:
  - `crates/codegen-wasm/tests/closures_runtime.rs`
- Added user-facing closure syntax/examples/rules in:
  - `docs/typing.md` (function types, lambdas, v1 restrictions).

## 2026-02-12 - Phase 17.7.4 closure lowering/codegen completion
- Completed 17.7.4 end-to-end closure invoke path:
  - lowered lambdas now produce synthetic lambda-body functions with hidden `env_ptr` ABI,
  - dynamic closure calls now lower to signature-specific dispatcher wrappers (no `call_indirect`),
  - dispatcher callee indices are patched deterministically after module assembly.
- Added runtime coverage for closure dispatch in `crates/codegen-wasm/tests/closures_runtime.rs`:
  - non-capturing closure call via function value,
  - capturing closure call via function value.
- Updated `docs/TODO.md` to mark 17.7.4.1 through 17.7.4.4 complete.

## 2026-02-12 - Phase 17.7.3 typer closure + decision closure
- Completed and committed Phase 17.7.3 typer closure behavior and decision follow-through.
  - `d46dd93`: closure typing/capture checks, function-typed local/param call typing, lambda-body effect integration.
  - `a063176`: conservative effect checking for function-value calls, explicit self/mutual closure-recursion diagnostics, tests/docs rollout updates.
- Documented decisions in `docs/design/phase-17.7-closures.md`:
  - D15 function-value call effect policy (unknown callee effect requires `io` in v1).
  - D16 explicit closure recursion diagnostics policy (no fallback unknown-function behavior).
- Updated `docs/TODO.md` with completed 17.7.3.4 and 17.7.3.5 entries.

## 2026-02-11 - Phase 17.6 completion + post-review hardening
- Completed Phase 17.6 end-to-end in `docs/TODO.md` (17.6.1 design, 17.6.2 typer diagnostics/rules, 17.6.3 VC/effect prototype + fixtures/tests).
- Landed VC/test/doc closure for inline-owner collection flows:
  - `5ac55f4` tests/docs: add linear collection VC fixtures and workflow coverage.
  - `473dac3` typer: track rebound linear owners in VC control-flow obligations.
  - `4c48ce7` close 17.6.3 test gaps for inline-owner linear collections.
- Resolved review gap using the "consistency + simplicity" rule (lexical scoping wins):
  - `ddcf5d3` typer: respect lexical shadowing in linear VC owner tracking.
  - `50257c5` tests: cover loop inline-owner shadowing in linear VC generation.
- Added/updated rollout-adjacent references for the new fixture and design-note alignment:
  - `docs/proofs/fixtures/linear-collections-branch-inline.vc.json`
  - `docs/proofs/fixtures/README.md`
  - `docs/proofs/vc-schema.md`
  - `docs/design/phase-17.6-linear-aware-collections.md`

## Next Focus
- Start Phase 17.7.4 lowering/codegen for closures: environment layout + invoke ABI + dispatcher wrappers.
- Keep 17.7.5 docs/examples queued immediately after 17.7.4.
- Keep 17.8 trait follow-ups queued until 17.7 is fully closed.

## 2026-02-08 - Phase 17 focus realignment + 17.6 kickoff prep
- Reconciled rollout planning with `docs/TODO.md`: Phases 17.4 (arrays/slices) and 17.5 (modules/imports) are complete.
- Set Phase 17.6 (linear-aware collections) as the active next focus in `docs/rollout/DEVPLAN.md`.
- Expanded `docs/TODO.md` 17.6 into an actionable checklist for:
  - design note scope and invariants (`docs/design/phase-17.6-linear-aware-collections.md`),
  - typer/diagnostic implementation goals,
  - VC/effect integration and prototype test coverage.

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

## 2026-02-02 - Phase 17.1 structs/enums delivery
- Implemented struct/enum typing, lowering, and runtime layout (structs use tuple layout; enums reuse the 16-byte variant layout).
- Added enum tag validation for user enums via `VariantKind::Enum { max_tag }` and surfaced R003 detail as `Enum`.
- Added lowering coverage for enum constructors/matches and struct field access, plus wasm runtime tests for layout and match execution.
- Updated runtime/ABI docs for struct/enum layout and marked Phase 17.1 complete in `docs/TODO.md`.

## 2026-02-02 - Phase 17.2 generics/traits delivery
- Implemented monomorphization for generic functions and trait calls (static dispatch) with lowering for generic structs/enums.
- Added identifier-safe mangling (`$`-separated) and tests enforcing allowed charset; documented the mangling scheme and VC scope.
- Improved trait impl diagnostics (call-site spans, ambiguous impl candidates) and documented new error code T248.
- Added positive/negative generics+traits tests (bounds, contracts/invariants, nested generics, multi-field enum matches, missing bounds/impls).
- Documented Phase 17.2 decisions (default bodies deferred, explicit impl selection unsupported) and reserved `$` for mangling.

## 2026-02-06 - Phase 17.3 collections hardening + test closure
- Implemented structural equality for composite Map/Set keys (Option/Result/struct/enum/tuple/array) and added short-circuiting for structural comparisons.
- Added collection handle validation with explicit invalid-handle traps (R010): header bounds, data_ptr alignment/range checks, header consistency (`len <= cap`, `cap > 0`), and overflow-safe length guards.
- Introduced unsigned comparison support in IR (`LeU`) plus `memory.size` plumbing; aligned heap start to 16 bytes to keep variant/collection layout consistent.
- Expanded runtime tests for list/set/map correctness and invalid-handle cases (null/misaligned/out-of-bounds data_ptr, header inconsistencies, overflow guards) and composite-key equality coverage.
- Updated collection docs and TODO roadmap to reflect the new validation and test-gap closure.

