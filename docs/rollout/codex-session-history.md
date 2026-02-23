# Codex Session Context

## 2026-02-23 - Phase 18.0.6.4 strict-mode defaults rollout
- Rolled strict proof-mode validation forward for VC-emitting builds:
  - `crates/cli/src/main.rs`: added build flag `--proof-strict` (default `true`).
  - `crates/cli/src/commands/build.rs`: added strict proof validation that requires selected assumption boundaries when corresponding SMT-model surfaces are detected; emits deterministic build diagnostic `C014` on violation.
- Added targeted unit coverage in `crates/cli/src/commands/build.rs` for:
  - missing required boundary detection,
  - success path with complete boundary labeling.
- Docs/roadmap updates:
  - `docs/diagnostics.md`: added `C014`.
  - `docs/typing.md` and `docs/proofs/proof-coverage-matrix.md`: documented strict default behavior and permissive escape hatch (`--proof-strict=false`).
  - `docs/TODO.md`: marked `18.0.6.4`, parent `18.0.6`, and parent `18.0` complete.
  - `docs/rollout/DEVPLAN.md`: marked Phase 18 proof-model execution complete and moved next focus to Phase 19.
- Validation:
  - `cargo test -p clg-cli --test diagnostics_codes`
  - `cargo test -p clg-cli --test cli_it vc_outputs::build_emits_assumption_boundaries_in_vc_json_and_proof_section`
  - `cargo test -p clg-cli --test proof_coverage_matrix`
  - `cargo test -p clg-cli --test ci_workflow`

## 2026-02-23 - Phase 18.0.6.3 proof-coverage matrix publication
- Published proof-coverage matrix artifacts:
  - `docs/proofs/proof-coverage-matrix.md` (human-readable summary),
  - `docs/proofs/proof-coverage-matrix.json` (machine-readable source of truth).
- Matrix captures per-feature/per-intrinsic `proved` vs `assumed` status and maps each entry across `L0`-`L3`.
- Added maintenance CI gate:
  - `crates/cli/tests/proof_coverage_matrix.rs` validates schema, deterministic ordering, known assumption-boundary IDs, tier mapping invariants, and required intrinsic coverage.
  - wired into `.github/workflows/ci.yml` (`Proof regression gates`) and guarded by `crates/cli/tests/ci_workflow.rs`.
- Docs updates:
  - linked matrix from `docs/proofs/vc-schema.md` and `docs/typing.md`.
- Roadmap updates:
  - marked `18.0.6.3` complete in `docs/TODO.md`,
  - moved `docs/rollout/DEVPLAN.md` next focus to `18.0.6.4`.
- Validation:
  - `cargo test -p clg-cli --test proof_coverage_matrix`
  - `cargo test -p clg-cli --test ci_workflow`
  - `cargo test -p clg-cli --test vc_snapshots`
  - `cargo test -p clg-cli --test cli_it vc_outputs::build_emits_assumption_boundaries_in_vc_json_and_proof_section`
  - `cargo test -p clg-typer --test vc declares_bitwise_and_builtin_helpers`

## 2026-02-23 - Phase 18.0.6.2 proof-regression CI suites
- Added explicit CI proof-regression gates in `.github/workflows/ci.yml`:
  - `cargo test -p clg-cli --test vc_snapshots`
  - `cargo test -p clg-cli --test cli_it vc_outputs::build_emits_assumption_boundaries_in_vc_json_and_proof_section`
  - `cargo test -p clg-typer --test vc declares_bitwise_and_builtin_helpers`
- Added CI workflow guard assertions in `crates/cli/tests/ci_workflow.rs` so gate commands cannot be removed silently.
- Expanded proof fixture coverage to lock assumption-boundary behavior:
  - added `docs/proofs/fixtures/proof-model-assumptions.vc.json`,
  - added fixture source notes in `docs/proofs/fixtures/README.md`,
  - wired fixture into `crates/cli/tests/vc_snapshots.rs`.
- Roadmap updates:
  - marked `18.0.6.2` complete in `docs/TODO.md`,
  - updated `docs/rollout/DEVPLAN.md` to move next focus to `18.0.6.3`.
- Validation:
  - `cargo test -p clg-cli --test vc_snapshots`
  - `cargo test -p clg-cli --test ci_workflow`
  - `cargo test -p clg-cli --test cli_it vc_outputs::build_emits_assumption_boundaries_in_vc_json_and_proof_section`
  - `cargo test -p clg-typer --test vc declares_bitwise_and_builtin_helpers`

## 2026-02-23 - Phase 18.0.6.1 assumption-boundary artifacts
- Implemented explicit proof-model assumption boundaries on emitted verification artifacts:
  - `crates/typer/src/vc.rs`: added `AssumptionBoundary`/`AssumptionCategory` and attached `assumptions` to `VerificationCondition`.
  - `crates/typer/src/vc/generate.rs`: added deterministic per-function assumption collection for selected Phase 18.0.0 models:
    - `unsigned.int_model` (unsigned types/constructors/intrinsics),
    - `bitwise.uninterpreted` (bitwise/shift operators),
    - `crypto.uninterpreted` (`std::crypto::*` and `std::bytes::eq_ct` boundaries).
  - `crates/cli/src/commands/build.rs`: `--emit-vcs` JSON now emits optional `assumptions: { items: [...] }`.
  - `crates/cli/src/proofs.rs`: `clearlang.proof` VC entries now carry optional `assumptions`.
- Regression coverage:
  - `crates/typer/tests/vc.rs`: validates unsigned/bitwise/crypto assumption boundaries are emitted for mixed-limit modeling cases.
  - `crates/cli/tests/cli_it/vc_outputs.rs`: validates assumption boundaries in both VC JSON and proof-section CBOR.
- Validation:
  - `cargo test -p clg-typer --test vc`
  - `cargo test -p clg-cli --test cli_it vc_outputs`
  - `cargo test -p clg-cli --test vc_snapshots`
  - `cargo test -p clg-cli --test signing`
- Docs/roadmap updates:
  - `docs/proofs/vc-schema.md`, `docs/proofs/proof-section.md`, `docs/proofs/crypto-limitations.md`, and `docs/typing.md` now document assumption IDs and artifact fields.
  - marked `18.0.6.1` complete in `docs/TODO.md`.
  - updated `docs/rollout/DEVPLAN.md` to set `18.0.6.2` as the next active sub-step.

## 2026-02-23 - Phase 18.0.5.4 SDK usability gates
- Implemented SDK usability regression gates:
  - added `crates/cli/tests/cli_it/sdk_usability.rs` with metrics checks covering import ergonomics, error quality, and migration friction,
  - wired the module into `crates/cli/tests/cli_it.rs`.
- CI gate enforcement:
  - added explicit `SDK usability gates` step to `.github/workflows/ci.yml` running:
    - `cargo test -p clg-cli --test cli_it sdk_usability_`
    - `cargo test -p clg-cli --test cli_it migration_`
    - `cargo test -p clg-cli --test cli_it imports::`
  - extended `crates/cli/tests/ci_workflow.rs` assertions to fail if those gate commands are removed.
- Parser structured-error determinism follow-up:
  - when specialized parse diagnostics (`P011`, `P012`, `P013`) overlap generic parse failures, structured `P001` duplicates are filtered to reduce migration friction.
- Validation:
  - `cargo test -p clg-cli --test cli_it sdk_usability_`
  - `cargo test -p clg-cli --test cli_it migration_`
  - `cargo test -p clg-cli --test cli_it imports::`
  - `cargo test -p clg-cli --test ci_workflow`
- Roadmap updates:
  - marked `18.0.5.4` and parent `18.0.5` complete in `docs/TODO.md`,
  - moved active focus to `18.0.6` in `docs/TODO.md` and `docs/rollout/DEVPLAN.md`.

## 2026-02-23 - Phase 18.0.5.3 deterministic migration coverage
- Added migration fixtures for deferred ergonomics restrictions:
  - `clearlang-tests/migration/01_inline_refinement_param.clear` (`P013`)
  - `clearlang-tests/migration/02_interface_type_params.clear` (`T246`)
  - `clearlang-tests/migration/03_implementation_method_type_params.clear` (`T245`)
  - `clearlang-tests/migration/04_generic_refinement_alias.clear` (`T244`)
  - `clearlang-tests/migration/05_set_resource.clear` (`T806`)
  - `clearlang-tests/migration/06_array_resource.clear` (`T806`)
- Added deterministic CLI IT assertions in `crates/cli/tests/cli_it/diagnostics.rs`:
  - parse/build JSON errors return exactly one expected restriction code for migration fixtures.
- Parser deterministic-resolution refinement:
  - when specialized parse diagnostics (`P011`, `P012`, `P013`) are emitted, overlapping generic `P001` entries are dropped in structured output to avoid fallback ambiguity.
- Validation:
  - `cargo test -p clg-cli --test cli_it migration_`
  - `cargo test -p clg-parser --test parse_structured_errors`
  - `cargo test -p clg-cli --test diagnostics_codes`
- Roadmap updates:
  - marked `18.0.5.3` complete in `docs/TODO.md`,
  - moved next active sub-step to `18.0.5.4` in `docs/rollout/DEVPLAN.md`.

## 2026-02-23 - Phase 18.0.5.2 deferred-form diagnostics closure
- Added explicit inline-refinement parse diagnostic for alias-only refinement policy:
  - parser emits `P013` when inline param/return refinement syntax is used,
  - structured parser error coverage added in `crates/parser/tests/parse_structured_errors.rs`.
- Docs updates:
  - added `P013` in `docs/diagnostics.md`,
  - updated refinement diagnostics wording in `docs/typing.md`,
  - updated `docs/design/phase-18.0.5-language-ergonomics.md` acceptance matrix.
- Roadmap updates:
  - marked `18.0.5.2` complete in `docs/TODO.md`,
  - moved DEVPLAN next focus within 18.0.5 to `18.0.5.3`.

## 2026-02-23 - Phase 18.0.5.1 ergonomics design lock
- Published `docs/design/phase-18.0.5-language-ergonomics.md` to lock:
  - accepted ergonomics execution scope from `18.0.0`,
  - non-goals (no surface expansion in this slice),
  - deterministic syntax/typing/diagnostics boundaries and acceptance matrix.
- Roadmap updates:
  - marked `18.0.5.1` complete in `docs/TODO.md`,
  - updated `docs/rollout/DEVPLAN.md` to set `18.0.5.2` as the next active sub-step.

## 2026-02-23 - Phase 18.0.4 runtime-ops gate closure
- Completed runtime-ops gate documentation for closure-env no-free policy:
  - published concrete host runbook defaults, thresholds, triggers, and alert severities in `docs/rollout/closure-env-ops-runbook.md`,
  - added evidence index scaffold in `docs/evidence/closure-env/README.md`,
  - linked closure operational guidance from `docs/typing.md`.
- Roadmap updates:
  - marked `18.0.4` and `18.0.4.1`-`18.0.4.5` complete in `docs/TODO.md`,
  - moved current focus to `18.0.5`,
  - updated `docs/rollout/DEVPLAN.md` to set `18.0.5` as next active execution slice.

## 2026-02-14 - Phase 17.8.3.3 shortened-name traceability
- Implemented emitted-name to canonical-name traceability in proof/debug artifacts:
  - `crates/typer/src/check/monomorphize/mod.rs`: `monomorphize_program` now returns both the monomorphized program and the emitted->canonical mangled-name origin map.
  - `crates/typer/src/check/mod.rs` and `crates/typer/src/check/fast_path.rs`: wired the map into `TypecheckOutput`.
  - `crates/cli/src/commands/build.rs`: `--emit-vcs` JSON now writes optional `canonical_function` when function names are shortened.
  - `crates/cli/src/proofs.rs`: `clearlang.proof` function entries now include optional `canonical_name` for shortened symbols.
- Added regression coverage:
  - `crates/cli/tests/cli_it/vc_outputs.rs`: new test forces shortening via `CLG_MANGLE_MAX_LEN` and verifies canonical mapping in both VC JSON and proof section CBOR.
- Docs/roadmap updates:
  - `docs/proofs/vc-schema.md`: documented optional `canonical_function`.
  - `docs/proofs/proof-section.md`: documented optional function-level `canonical_name`.
  - `docs/design/phase-17.2-generics-traits.md`: recorded 17.8.3.3 traceability policy.
  - `docs/TODO.md`: marked `17.8.3` and `17.8.3.3` complete.
  - `docs/rollout/DEVPLAN.md`: moved next execution slice beyond 17.8.3.

## 2026-02-14 - Phase 17.8.3.2 shortening collision safety
- Added deterministic collision safety for optional mangling shortening:
  - `crates/typer/src/check/monomorphize/mod.rs`: added tracking map from emitted mangled names to canonical (unshortened) names.
  - `crates/typer/src/check/monomorphize/dispatch.rs`: records canonical and emitted names for both generic functions and interface implementations, rejecting collisions.
  - `crates/typer/src/errors/generics.rs`: added `T250` for emitted-name collisions under shortening mode.
- Added coverage:
  - `crates/typer/src/check/monomorphize/tests.rs`: new collision test ensures conflicting origins for one emitted name fail with `T250`.
- Docs/roadmap updates:
  - `docs/diagnostics.md`: added `T250`.
  - `docs/TODO.md`: marked `17.8.3.2` complete.
  - `docs/design/phase-17.2-generics-traits.md`: documented collision-safety behavior.
  - `docs/rollout/DEVPLAN.md`: next execution now focuses on `17.8.3.3` traceability.

## 2026-02-14 - Phase 17.8.3.1 optional mangling shortening
- Implemented an optional deterministic shortening mode for long mangled names:
  - `crates/typer/src/check/monomorphize/mangle.rs`:
    - added `MangleConfig` with env-based config (`CLG_MANGLE_MAX_LEN`),
    - added deterministic FNV-1a hash suffix shortening (`$h<16-hex>`),
    - preserved default behavior when shortening mode is not enabled.
  - `crates/typer/src/check/monomorphize/tests.rs`:
    - added coverage for deterministic shortened output, max-length enforcement, charset safety, and distinct-input differentiation.
- Roadmap/docs updates:
  - removed deferred `17.8.2.2` from `docs/TODO.md` per strict-coherence decision (no relax plan),
  - marked `17.8.3.1` complete in `docs/TODO.md`,
  - updated next execution slice in `docs/rollout/DEVPLAN.md` to focus on `17.8.3.2` and `17.8.3.3`,
  - documented the optional shortening mode in `docs/design/phase-17.2-generics-traits.md`.

## 2026-02-13 - Phase 17.8.4 keyword hard switch completed
- Implemented the pre-release hard switch from `trait`/`impl` to `interface`/`implementation` with no compatibility shim:
  - `crates/parser/src/trait_decl.rs`: parser now accepts `interface`; default-body parser diagnostic now references interface wording.
  - `crates/parser/src/impl_decl.rs`: parser now accepts `implementation`.
  - `crates/parser/src/tokens.rs`: reserved-word lists now reserve `interface`/`implementation` instead of `trait`/`impl`.
  - `crates/parser/src/program.rs`: export diagnostic now says "implementation blocks cannot be exported".
- Updated user-facing typer diagnostics/context wording:
  - `crates/typer/src/errors/traits.rs`: messages now use interface/implementation terminology while preserving existing codes (`T230`-`T249`).
  - `crates/typer/src/check/mod.rs` and `crates/typer/src/check/fast_path.rs`: context strings now use interface/implementation wording.
- Migrated parser/typer fixtures and regression coverage:
  - `crates/parser/tests/generics_traits.rs`
  - `crates/parser/tests/parse_negatives.rs` (added explicit rejection tests for legacy `trait` and `impl` keywords)
  - `crates/typer/tests/generics_traits.rs`
  - `crates/typer/tests/generics_traits_negatives.rs`
  - `crates/typer/src/check/monomorphize/tests.rs` (diagnostic substring expectation update)
- Updated docs wording/examples for the new surface:
  - `docs/diagnostics.md`
  - `docs/typing.md`
  - `docs/design/phase-17.2-generics-traits.md`
  - `docs/design/phase-17.5-modules-imports.md`
  - `docs/design/phase-17.8-trait-defaults.md`
  - `docs/TODO.md`
  - `docs/rollout/DEVPLAN.md`
- Validation:
  - `cargo test -p clg-parser`
  - `cargo test -p clg-typer --tests`
  - `cargo test -p clg-cli --test diagnostics_codes`

## 2026-02-13 - Phase 17.8 hard-switch decision documented
- Recorded the pre-release hard-switch decision to rename keyword surface from `trait`/`impl` to `interface`/`implementation` (no backward-compatibility shim).
- Added an actionable checklist in `docs/TODO.md` under Phase 17.8 (`17.8.4.1` through `17.8.4.5`) covering parser keywords, reserved words, diagnostics/docs wording, tests/fixtures migration, and final validation.
- Updated `docs/rollout/DEVPLAN.md` so the next execution slice starts with 17.8.4 before optional name-shortening work.

## 2026-02-13 - Phase 17.8.1.5 tests/docs closure
- Closed 17.8.1.5 with docs and regression coverage updates:
  - `docs/typing.md`: added a dedicated "Traits and Default Methods (Phase 17.8.1)" section covering dual syntax forms, omission/override rules, exact-effect policy, and static dispatch behavior.
  - `crates/typer/tests/generics_traits.rs`: added positive coverage for explicit impl override when a trait default exists.
  - `crates/typer/tests/generics_traits_negatives.rs`: added negative coverage for override effect mismatch (`T235`) with trait defaults present.
- Validation:
  - `cargo test -p clg-typer --tests`
- Roadmap updates:
  - marked 17.8.1.5 complete and set 17.8.1 parent to complete in `docs/TODO.md`,
  - updated `docs/rollout/DEVPLAN.md` to move next execution toward 17.8.3.1.

## 2026-02-13 - Phase 17.8.1.3 trait-default effect exactness
- Implemented exact effect enforcement for trait default method bodies:
  - `crates/typer/src/check/function_checks.rs`: added trait-default checker that type-checks the default body under `Self` bounds and enforces exact effect equality.
  - `crates/typer/src/check/mod.rs` and `crates/typer/src/check/fast_path.rs`: wired trait-default checking into both normal and fast-path typecheck flows.
  - `crates/typer/src/errors/traits.rs`: added `T249` (`trait default method body effect mismatch`).
- Added tests:
  - `crates/typer/tests/generics_traits_negatives.rs`: mismatch when body is stronger than declared; mismatch when body is weaker than declared.
- Docs:
  - `docs/diagnostics.md`: added `T249` to the error-code table.
- Validation:
  - `cargo test -p clg-typer --tests`
  - `cargo test -p clg-cli --test diagnostics_codes`
- Roadmap updates:
  - marked 17.8.1.3 complete in `docs/TODO.md`,
  - updated `docs/rollout/DEVPLAN.md` to make 17.8.1.5 the next sub-step.

## 2026-02-13 - Phase 17.8.1.2 typer default-method omission
- Implemented 17.8.1.2 behavior:
  - `crates/typer/src/check/trait_env.rs` now allows an impl to omit a trait method only when that trait method has a default body; non-default omissions still emit `T233`.
  - `crates/typer/src/check/monomorphize/dispatch.rs` now falls back to trait default method bodies when an impl omits the method, instantiating the canonical mangled impl symbol.
- Added regression coverage:
  - `crates/typer/tests/generics_traits.rs`: positive case for omitted impl method with trait default.
  - `crates/typer/tests/generics_traits_negatives.rs`: missing non-default method still errors with `T233`.
- Validation:
  - `cargo test -p clg-typer --tests`
- Roadmap updates:
  - marked 17.8.1.2 complete in `docs/TODO.md`,
  - updated `docs/rollout/DEVPLAN.md` to set 17.8.1.3 as the next step.

## 2026-02-13 - Phase 17.8.1.1 parser/AST trait defaults
- Implemented parser/AST support for trait default methods (`... { ... }`) while keeping declaration form (`...;`):
  - `crates/ast/src/lib.rs`: `TraitMethod` now carries `default_body: Option<Expr>`.
  - `crates/parser/src/trait_decl.rs`: parser accepts either `;` or a block default body.
- Added parser coverage:
  - positive: `crates/parser/tests/generics_traits.rs` validates declaration + default-body forms.
  - negative: `crates/parser/tests/parse_negatives.rs` rejects malformed mixed `; { ... }` forms.
- Validation:
  - `cargo test -p clg-parser`
  - `cargo test -p clg-typer --tests`
- Roadmap updates:
  - marked 17.8.1.1 complete in `docs/TODO.md`,
  - marked DEVPLAN execution slice step 2 done in `docs/rollout/DEVPLAN.md`.

## 2026-02-13 - Phase 17.8.1.0 design lock for trait defaults
- Added `docs/design/phase-17.8-trait-defaults.md` to lock 17.8.1.0 decisions.
- Captured:
  - dual trait-method surface (`...;` declaration or `... { ... }` default body),
  - impl completeness behavior (reuse `T233`/`T234`/`T235`),
  - exact-effect requirement for default bodies with planned dedicated code `T249`,
  - acceptance matrix and deterministic diagnostics policy.
- Updated roadmap state:
  - marked 17.8.1.0 complete in `docs/TODO.md`,
  - updated `docs/rollout/DEVPLAN.md` to mark step 1 done and point to 17.8.1.1 next.

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

