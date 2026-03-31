# ClearLang TODO

A focused, actionable checklist to move from Phase 17.8 and beyond.

## Phase 0 -" Workspace & Toolchain (Done)

- [x] Set up Rust toolchain and Cargo workspace.

- [x] Add crates: `cli`, `parser`, `ast`, `typer`, `ir`, `codegen-wasm`.

- [x] Install and use Wasmtime and `wasm-tools` locally.

## Phase 1 -" Hello WASM (Done)

- [x] Implement `emit_trivial_main` producing `main() -> i32` returning 42.

- [x] Add CLI `emit-hello` subcommand writing `hello.wasm` (creates parent dirs).

- [x] Validate with `wasm-tools validate` and run with Wasmtime.

- [x] Add tests to check Wasm header/export and `i32.const 42`.

## Phase 2 -" Parser (Done)

2.1 Syntax & Literals

- [x] Basic syntax: functions, parameters (`name: Type`), Int/Bool types.

- [x] Literals: integers, `true`/`false` (Bool in AST).

2.2 Calls & Commas

- [x] Function calls with comma-separated args; missing comma errors.

- [x] Trailing comma policy: allowed in parameter lists, disallowed in call args.

2.3 Precedence & Grouping

- [x] Operator precedence: `* /` > `+ -`; left-associative.

- [x] Parentheses for grouping, multi-line formatting.

2.4 Tooling & Samples

- [x] CLI `parse` subcommand pretty-prints AST.

- [x] clearlang-tests samples for positive and negative cases.

2.5 Error Coverage (Optional)

- [x] Improve parse error messages/spans where useful.

- [x] More negative tests (unknown idents, reserved keywords).

## Phase 3 -" Typer & IR (Done)

Current focus: Phase 19 complete - next roadmap slice pending.

3.1 Typer Core

- [x] Function env: collect signatures (name, params, ret, effect).

- [x] Rules for `Int`/`Bool`, variables, `+ - * /`, and calls.

- [x] Arity/return checks; unknown function errors (spans deferred to 3.8).

3.2 Effects (Stub)

 - [x] Accept `Effect::None|Pure` initially; plan enforcement later.

 - [x] Reject Bool arithmetic; ensure function bodies match return types.

3.3 IR Shape

- [x] Minimal SSA-like IR in `crates/ir` (Function, Instr, Type; single block for now).

- [x] Instrs: `IConst`, `IBin(Add|Sub|Mul|Div)`, `Call`, `Ret`.

 - [x] Unit tests: construct simple functions and call variants.

3.4 Lowering

- [x] Lower AST +' IR guided by typer; preserve minimal names/locals.

3.5 Integration

- [x] CLI `build`: parse +' type-check +' lower to IR +' emit Wasm.

- [x] Codegen replaces const-eval; keep const-eval behind a feature flag (optional).

 - [x] Add `--validate` flag to run `wasm-tools validate` on outputs.

3.6 Tests

- [x] Typer errors: unknown function, arity mismatch, type mismatch.

- [x] IR/codegen e2e for `01_hello`, `02_arith`, `03_nested_calls`, `04_multiline_call`, `05_trailing_param_comma`.

- [x] Keep negative parse case `06_trailing_call_comma`.

3.7 Docs

- [x] Add `docs/typing.md` (rules/spec) and `docs/ir.md` (IR shape).

- [x] Update README "Current Status" to mark Phase 3 in progress.

3.8 Diagnostics & Spans

- [x] Attach source spans in AST via chumsky for identifiers and expressions.

- [x] Propagate spans into typer errors (unknown var/fn, arity, return/type mismatch).

- [x] Add tests that assert span presence/format in error messages.

- [x] Gate Phase 4 switch (IR+'Wasm as default) on basic span coverage to avoid tech debt.

## Phase 4 - Namespacing + Strings (Parse/Type) + Std Collections stubs + Small Optimizations (Done)

- 4.1 Namespacing (no tech debt)

  - [x] Add namespaced call syntax (paths): `std::str::len(s)`, `std::list::push(l,x)`, `std::map::get(m,k)`, `std::set::contains(s,x)`.

  - [x] Typer: accept namespaced callees as exact names (parser done; built-ins via 4.3-"4.7).

- 4.2 Strings (parse/type only)

  - [x] Add `Type::String` and `Expr::String` with escapes and multi-line string literal support.

  - [x] Typer: `std::str` built-ins (len/concat/eq) for strings.

  - [x] Typer: make `String` first-class in params/returns and literals.

  - [x] Tests: single-line, multi-line, escapes, span diagnostics.

  - [x] DX: Split parser into modules to unblock strings and path growth (`tokens.rs`, `types.rs`, `literals.rs`, `path.rs`, `expr.rs`, `func.rs`, `program.rs`; `lib.rs` wires them).

  - [x] Syntax: adopt `function` keyword exclusively (remove `fn`); update parser, tests, and docs for clarity and readability.

  - [x] CLI/Diagnostics: add `--json-errors` with short error codes (e.g., P001, T003) to support AI repair loops.

  - [x] Rename Str +' String: one-shot rename across AST/parser/typer/tests/docs; keep semantics unchanged.

- 4.3 Collections Strict Errors (Option A)

  - [x] Typer: detect `std::list/*`, `std::set/*`, `std::map/*` and emit one clear, spanful error

        (code `T101` "collections require generics/ADTs; planned in later slices").

  - [x] Tests: calls parse; type errors return stable JSON (`--json-errors`) with code and spans.

- 4.4 Parametric Types + Minimal ADTs + `match` (built-ins only)

  - [x] Parser: accept `Option<T>` and `Result<T,E>` types; add minimal `match` for these two ADTs.

  - [x] Tests: constructors and simple matches parse.

- 4.5 Option/Result Typing Rules

  - [x] Typer: rules for `Option`/`Result` `match` (exhaustiveness, binders, arm type unification); partial constructors support (`Some(expr)`); no codegen/runtime yet.

  - [x] Tests: typer-only tests for `Option`/`Result` matches and errors (T201-"T205); Option constructors (`Some`) and identity.

- 4.6 Collections Signatures (type-only)

  - [x] Provide namespaced APIs using Option/Result (type-check only):

        `std::list::{len,push,pop}` (pop -> Option<T>); `new` present but requires inference +' emits T206.

        `std::set::{len,insert,remove,contains}`.

        `std::map::{len,insert,remove,get (-> Option<V>), contains}`.

  - [x] Parser: add `List<T>`, `Set<T>`, `Map<K,V>` types.

  - [x] Typer: enforce element/key/value types; dedicated errors T206-"T208.

  - [x] Tests: typer-only tests for positive and negative cases.

- 4.7 Collections Docs/DX

  - [x] Document naming (modules lower-case: `std::list`; types PascalCase: `List<T>`),

        dual-API guidance (precondition vs Option/Result), and JSON error codes

        (include T206 "new requires inference" and T207 "expected collection kind").

  - [x] DX: Split typer - move rules to `check.rs` and IR lowering to `lower.rs`; keep `lib.rs` as public entry.

  - [x] Add `docs/collections.md` and update `docs/diagnostics.md` and `docs/typing.md`.

  - 4.8 Small Optimizations & DX

    - [x] Optional Wasm name section for function names (`--debug-names`).

    - [x] Remaining items moved to later phases (see 5.0 verbose logs, 5.5 shared Wasmtime Engine, and Developer Experience preallocations + release profile tuning).

- 4.9 Return (expression form)

  - [x] AST: add `Expr::Return { expr, span }`.

  - [x] Parser: reserve `return`; parse `return expr`.

  - [x] Typer: type-check `return expr` as inner type; include in spans.

  - [x] Lowering: treat `return e` as `e` in expression-bodied functions (final Ret unchanged).

  - [x] Const-eval: handle `return` by evaluating inner expr.

  - [x] Tests: add `clearlang-tests/18_return_simple.clear` and include in CLI IT.

- 4.10 Conditionals (expr-form if/else chain)

  - [x] Parser: `if cond { ... } (else if cond { ... })* else { ... }` (no `elif` alias).

  - [x] Typer: `cond: Bool` per arm; all branches unify to a single result type.

  - [x] Tests: multi-branch chains; require final `else` in expression form.

  - [x] Diagnostics: reserve codes P010 (MissingElseForExprIf) and T301 (BranchTypeMismatch);

        include spans in JSON and add tests.

## Phase 5 -" Codegen IR +' Wasm (Done)

5.0 Codegen Layout & DX

- [x] Gate build stage logs behind `--verbose` (keep default quieter).

- [x] Split `codegen-wasm` into `trivial.rs` (emit_trivial_main), `intrinsics/strings.rs` (string intrinsics), and `ir.rs` (IR+'Wasm encoder); re-export from `lib.rs`.

 - [x] Refactor CLI by splitting subcommands into `commands/{emit_hello,parse,build,run}.rs` and small helpers.

5.1 Module & Signatures

- [x] Types, function indices, and exports.

- [x] Deduplicate function signatures in the Wasm Type section (reuse type indices).

5.2 Locals & Stack

- [x] Local allocation for temps; map IR values to stack ops.

5.3 Ops & Calls

- [x] Encode `IConst`, `IBin`, and `Call` to Wasm; verify results in IT.

- [x] Switch calls to use callee indices; update IR lowering accordingly (resolve names to indices).

5.4 Replace Const-Eval

- [x] Switch CLI build to IR+'Wasm path by default; keep const-eval for quick checks.

- [x] Remove const-eval fallback once IR path is stable (cleanup).

5.5 Tests

- [x] Extend e2e tests to verify outputs across samples via Wasmtime (IR pipeline samples; strings len/eq/concat).

- [x] Reuse a shared Wasmtime `Engine` in tests (e.g., `once_cell`) to speed up instantiation.

 - [x] Verify Wasm Type section dedup via structure tests.

 - [x] Verify `--debug-names` emits custom name section.

 - [x] Add CLI `run` smoke test to execute `main` and assert stdout.

5.6 Strings Runtime

- [x] Define String memory model (ptr + len; utf-8 bytes).

- [x] Implement `std::str::len` intrinsic and memory scaffolding.

- [x] Implement `std::str::eq` intrinsic (byte-wise compare; length-decrementing loop).

- [x] Provide a minimal bump allocator (global heap_ptr) for concat.

- [x] Implement `std::str::concat` intrinsic (header write; memory.copy for bytes; aligned bump).

- [x] Add e2e tests for string ops (len, eq, concat).

- [x] Add property tests for strings invariants (len/concat/eq).

5.7 (moved) -" see 6.5 ADT Ergonomics (sugar)

## Phase 6 -" Contracts & Effects (Safety)

Docs & Proofs

- [x] Draft VC JSON schema (`docs/proofs/vc-schema.md`).

- [x] Add value-preservation proof sketch for Int/Bool arith + calls (`docs/proofs/value-preservation.md`).

6.1 Syntax

- [x] Parse `require { expr }` / `ensure { expr }` blocks with spans.

- [x] Parse effect qualifiers (`pure|mut|io`) and reject unknown effects.

- [x] Allow multiple `require`/`ensure` clauses and define conjoined semantics.

6.2 Typing & VC Generation

- [x] Enforce purity/effect rules in typer and reject contracts on non-`pure` functions for now.

- [x] Generate verification conditions for expression-bodied `pure` functions; emit single VC `P #' Q[e/result]` (QF_LIA + Bool).

- [x] Ensure VCs are ordered deterministically and match `docs/proofs/vc-schema.md` schema.

- [x] CLI `--emit-vcs` flag writes JSON with `{ function, vc_id, pre, post, smt2, status }`.

- [x] Snapshot tests: passing (inc/add) and failing ensure cases.

- [x] Extend expression grammar for contracts: add comparison ops (`>`, `>=`, `<`, `<=`, `==`, `!=`), logical ops (`&&`, `||`, `!`), and ensure resulting predicates type to `Bool`.

- [x] Update typing/codegen to support those operators or emit clear diagnostics until codegen handles them.

6.3 Runtime Enforcement

- [x] Lower `require`/`ensure` clauses into runtime guard blocks that emit trap code `R000` on failure while preserving span info for diagnostics.

- [x] Extend the string allocator to raise `R001` (OOM) and reject non-UTF-8 writes with `R002`; surface both in `docs/runtime/strings.md` and CLI help.

- [x] Integration tests: one contract-violation sample and one forced allocator failure asserting trap codes and JSON diagnostics stay AI-friendly.

- [x] Follow-up: centralize guard/trap helper scaffolding in codegen intrinsics to keep proofs/docstrings in sync as we add more runtime checks.

6.4 Mutable Collections (effects)

- [x] Harden the effect lattice by defining Effect::{Pure,Mut,Io} and preventing downgrades in typer and lowering.

- [x] Stub std::list/set/map::*_mut builtins with explicit pre/post contracts and emit deterministic T401 when callers lack mut allowance.

- [x] Require paired require guards before *_mut calls; extend VC generation with stable mut_pre obligations and snapshot tests.

- [x] Add regression tests covering accepted mut functions, rejected pure callers, and JSON diagnostics to keep outputs AI-friendly.

- [x] Update docs/typing.md and docs/collections.md with the effect table, soundness sketch, and SMT-friendly examples.

6.5 Proof Packaging & Signatures

- [x] Define `clearlang.proof` custom section v1 layout (versioning, VC linkage, optional proofs) and document it in `docs/proofs/proof-section.md`.

- [x] Emit the section whenever `--emit-vcs` is used, including canonical serialization plus SHA-256 hashes for module and proof payloads (JCS encoded).

- [x] Implement CLI signing flow: `--sign --key --key-id --scope {module,proofs,both}` + `--sig-out`, and verification via `clg verify --sig --pubkey` with clear error codes.

- [x] Add integration tests that sign/verify a small module and fail gracefully when a proof hash is tampered.

6.6 ADT Ergonomics (sugar)

- [x] Draft a design note for `if let`, `??`, and `?`, covering parser surface, typer desugaring, effect constraints, and proof obligations; include open tech-debt bullets (see `docs/design/phase-6.6-adt-ergonomics.md`).

- [x] Implement `if let` sugar: parser/AST support, typer desugar to two-arm `match`, parser/typer unit tests, and doc updates (VC snapshot coverage remains open).

- [x] Prototype `??` (Option coalesce) behind an experimental flag with parser/typer support, option-focused regression tests, and docs.

- [x] Prototype `?` (Option/Result propagation) behind an experimental flag with parser/typer support, purity/effect validation, and unit tests.

- [x] Add worked examples showing each desugar and track remaining tech debt (VC snapshots, codegen for `Expr::Try`) before lifting the experimental flag (see design note + `crates/typer/tests/vc.rs`).
- [x] Document the Option/Result lowering plan in `docs/typing.md` and related docs once the IR/runtime encoding is implemented.

- [x] Add VC snapshot artifacts for `if let`/`??`/`?` in `docs/proofs` after SMT encoding replaces placeholder comments (current output still notes placeholders).

- [x] Update CLI/docs with end-to-end examples of the sugar (build/run/--emit-vcs) once lowering is shipped and stable (documented current workflow + limitations).

## Phase 7 - Option/Result Lowering

7.1 IR & Runtime Encoding (Done)

Spec & Layout
- [x] Freeze the `{ tag, payload }` representation for `Option`/`Result`, including tag constants, payload alignment, and the shared panic-on-invalid-tag policy (see `docs/design/phase-7.1-option-result-runtime.md`).
- [x] Capture the layout/invariants in a dedicated design slice (`docs/design/phase-7.1-option-result-runtime.md`) and cross-link from `docs/typing.md`.

IR & Desugaring
- [x] Introduce IR helpers for constructing and projecting tagged values (`Option::{some,none}`, `Result::{ok,err}`) and plumb them through typer lowering.
- [x] Rewrite the Phase 6.6 sugar (`if let`, `??`, postfix `?`) so lowering emits explicit tag checks + payload extraction ahead of IR emission.
- [x] Update `Expr::Try` lowering to branch on the tag, thread early-return exits, and support nested `try` without re-checking payloads.

Codegen & Runtime
- [x] Emit Wasm for the new IR ops, writing tag/payload with proper alignment and zeroing unused payload bytes.
- [x] Add a shared runtime helper that traps on invalid tags and hook it into every destructor path.
- [x] Extend CLI integration (`clg run`) and IR unit tests to cover constructor/destructor pairs, success cases, and propagation paths.
7.2 Verification & SMT (Done)

- [x] Replace VC encodings for `if let`/`??`/`?` with the tagged layout; update the SMT translator and keep option/result axioms in one module.
- [x] Refresh VC regression snapshots and worked examples (`docs/proofs`) to cover both `Option` and `Result` success/failure cases.
- [x] Extend effect-gate and VC matrix tests so `Expr::Try` and coalescing sugar appear in unit, snapshot, and integration suites.

7.3 Docs & Tooling (Done)

- [x] Update the CLI docs and `--emit-vcs` walkthrough to demonstrate lowering-enabled workflows; note how to inspect the emitted tagged encoding.
- [x] Revise `docs/typing.md` and the Phase 6.6 design note with the final lowering semantics, desugars, and outstanding follow-ups.
- [x] Remove the experimental flag for the new sugar once tests pass and release notes/README are updated.

## Phase 8 - Resource/Linear Types

8.1 Syntax & Semantics

- [x] Capture grammar + AST/consume plan in `docs/design/phase-8.1-resource-syntax.md` so implementation has a locked target.
- [x] Parse/AST support for `resource Name { fields ... drop { ... } }`, requiring an explicit drop block (empty block permitted for no-op cleanup).
- [x] Extend function signatures with the `consume` parameter modifier (borrows remain the default) and plumb the metadata through typer/lowering.

8.2 Typing Rules

- [x] Add an AST/Type representation for resources, teach the parser to accept it, update the typer to understand block bodies (for drops and future function bodies), and capture the linear-typing design plan.
- [x] Implement linear tracking so each resource is consumed or dropped exactly once; reject implicit copies.
- [x] Allow immutable borrows of active resources and emit targeted diagnostics for use-after-consume or borrow-after-consume errors.

8.3 Aliasing & Collections

- [x] For Phase 8.1, reject storing resources inside standard `List`/`Map`/tuple types and surface a dedicated error; design linear-aware collections as a follow-up.

8.4 Tests & Docs

- [x] Publish a "Resource Guide" covering built-in defaults, custom drops, consume semantics, and diagnostics.
- [x] Add unit/integration tests for consume success, reuse-after-consume errors, borrow checks, and container rejection.

## Phase 9 -" Totality & Loops with Invariants

9.1 Block Expressions

- [x] Add block expression/multi-statement body support (expression blocks with optional early `return`) before enabling Phase 9 totality work.

9.2 Syntax

- [x] Introduce `while` loops with required loop invariants and optional variants/measures for termination (typing + lowering complete).

9.3 Totality

- [x] Lowering support for `while` loops (previously typing-only) so totality checks can target runtime control flow; emit runtime guards for invariants and decreasing variants (non-negative + strictly decreasing checks).
- [x] VC plumbing for loop invariants/variants to feed termination proofs.
- [x] Enforce totality for `pure` functions: reject unmeasured recursion (require a structural recursion or explicit decreasing measure before lifting the restriction).
- [x] Provide diagnostics with spans for missing or non-decreasing measures.

9.4 Typing & Checks

- [x] Type rules for loop invariants; ensure invariants are well-typed and refer to in-scope variables.

- [x] Guardrail: allow opting out behind a flag initially to ease migration (`CLG_DISABLE_TOTALITY`).

9.5 Tests & Docs

- [x] Positive/negative tests for loops and recursion with measures.

- [x] Document totality policy and examples in `docs/typing.md`.

## Phase 10 -" Refinement Types

10.1 Design & Scope

- [x] Draft refinement syntax/scope note: alias binder rules (`type Nat = Int where n >= 0`), how the bound name is introduced/hidden, and whether refinements are allowed inline on params/returns or only via aliases.
- [x] Define well-formedness for nested/recursive aliases and when refinements must be simplified or rejected early (unsat/contradictory predicates).

10.2 Syntax & AST

- [x] Implement refined aliases (`type Name = T where pred`) with explicit binder semantics and AST representation that preserves predicates for later phases (parser/AST landed; typing/VC still pending).
- [x] Decide and implement where refinements can appear (aliases only vs inline), including scoping of the refinement variable inside composite types (inline refinements parse-error; aliases stored on Program).

10.3 Typing & Constraint Propagation

- [x] Alias collection/resolution: resolve alias bases, reject cycles/duplicates/resource conflicts, type-check predicates to Bool under binder env, and enforce predicate purity.
- [x] Param/return/call-site obligations: emit alias predicates for params/returns and call arguments into VC pre/post sets with binder substitution.
- [x] In-body flow preservation: carry predicates through let-bindings, alias-typed locals, call/constructor flows, and Option/Result destructuring (match/if-let/??/?), generating obligations for projected binders.
- [x] Refinement safety enforcement (implementation):
  - [x] Detect weakening when binding refined values to non-refined types (let/param/return) and emit a stable diagnostic code.
  - [x] Track branch joins: reject when one branch drops a refinement that another preserves.
  - [x] Require obligations when re-wrapping refined values into containers (Option/Result/List/Map/Set) so predicates are not lost.
- [x] Feature interactions (spec + enforcement):
  - [x] `require`/`ensure`: define ordering with alias obligations and surface them in VC pre/post; ensure runtime traps and VC failures stay consistent.
  - [x] Loops/totality: allow invariants to mention refined binders; ensure loop bodies preserve refinements across iterations and variants do not erase them.
  - [x] Effects/resources: forbid impurity/consumption inside predicates; decide and implement whether refined aliases can wrap resources (likely reject with code) or must remain non-resource.
  - [x] Option/Result + sugar: ensure constructors/coalesce/try preserve refinements; add checks/VCs for re-wrapping and for dropped refinements in sugar desugars.
- [x] Diagnostics + docs:
  - [x] Add dedicated error codes/messages for refinement loss, unsupported refined-resource combos, and impurity-in-predicate violations.
  - [x] Document the refinement preservation rules, interaction matrix, and escape hatches in `docs/typing.md` (and link from 10.4 VC section).
- [x] Migration/escape hatch:
  - [x] Add a migration escape hatch flag if needed (documented) and keep off by default. (Decision: no escape hatch for refinements; enforcement stays on.)

10.4 VC/SMT & Tooling (Done)

  - [x] VC shape:
    - [x] Extend `--emit-vcs` JSON schema to carry refinement premises (binder names, substituted exprs, attachment points per param/return/flow).
    - [x] Version the schema and document backward-compat gates for consumers.
  - [x] SMT encoding:
    - [x] Encode refinement predicates alongside contract predicates with stable symbol naming; include binder substitution helpers.
    - [x] Add SMT helpers for arithmetic/boolean predicates and any needed Option/Result axioms when refinements appear under containers.
  - [x] Tooling integration:
    - [x] Update VC generator to populate the new schema fields and include refinement obligations in SMT emission order.
    - [x] Add CLI flags/docs clarifying how refinement VCs surface in `--emit-vcs` outputs.
  - [x] Fixtures/examples:
    - [x] Add worked examples and JSON fixtures showing refinement obligations alone and combined with requires/ensures and loop invariants.
    - [x] Document how to read the refinement parts in `docs/proofs` and link from `docs/typing.md`.

10.5 Tests

- [x] Positive coverage:
  - [x] Non-negative/bounded/equality refinements over Int; preservation through locals/calls/containers.
  - [x] Option/Result interaction: match binders keep refinements; coalesce/try do not drop predicates.
  - [x] Loop/invariant usage of refined binders.
- [x] Negative coverage:
  - [x] Unsat/contradictory predicates rejected at alias definition.
  - [x] Refinement loss/weakening diagnostics (assignments, branch joins, rewrap without proof).
  - [x] Inline refinements (still disallowed), refined resources (policy), impurity-in-predicate errors.
- [x] VC snapshots:
  - [x] Add `--emit-vcs` snapshots for refined params/returns, call-site obligations, and loop/invariant combinations.
  - [x] Include fixtures where refinement VCs interact with requires/ensures and totality checks.

## Phase 11 - Proof-Carrying Wasm Verification

- 11.1 Review/optimize refinement UX:
  - [x] Document refinement diagnostics (T701-T708) in `docs/diagnostics.md` and update `docs/typing.md` with the exact limits of shallow unsat checks.
  - [x] Make VC fixture snippets runnable (include `main` or note `--emit-vcs` usage constraints) in `docs/proofs/fixtures/README.md`.
  - [x] Extend shallow unsat detection with simple linear normalization for `n + k`/`n - k` (no SMT integration).
- 11.2 Verifier CLI (`clg verify`):
  - [x] Parse Wasm modules and extract the `clearlang.proof` section; read signature file payloads.
  - [x] Verify module/proof hashes match the signed payload; validate signatures against provided pubkey.
  - [x] Return structured diagnostics (codes) on mismatch, missing sections, or signature failures.
  - [x] Split verify failures into distinct codes (missing proof, hash mismatch, signature failure).
- 11.3 Integration workflow:
  - [x] Add README/docs for signing/verifying flows; sample commands and expected outputs.
  - [x] Add regression tests: valid proof/signature passes; tampered proof/module fails with clear code; missing proof section fails.

## Phase 12 - Safety & Tooling Hardening

- [x] CI & validation:
  - [x] Always run `wasm-tools validate` on emitted modules in CI.
  - [x] Add CI workflow to run `cargo test --workspace` and pipeline/integration tests.
  - [x] Add regression coverage to ensure CI enforcement for validation and core test workflows.
- [x] Runtime bounds:
  - [x] Document Wasmtime fuel/epoch/memory limits and recommended defaults.
  - [x] Enable fuel/epoch limits in integration tests to enforce bounded execution.
  - [x] Add runtime fuel/epoch limit integration tests once limits are wired.
- [x] Contracts mode flag (design):
  - [x] Draft design for `--contracts=runtime|hybrid|static` behavior and migration path.

- [x] Document pre-commit hook usage in README; provide skip toggles.

## Phase 13 - Developer Experience

- [x] Observability:
  - [x] Add structured tracing/logging for CLI pipeline stages with duration summaries.
  - [x] Support verbosity levels (`-v`/`-vv`) and a `CLG_LOG` env override.
- [x] Tooling ergonomics:
  - [x] Add `cargo xtask` or `justfile` targets for `fmt`, `clippy`, `test`, `validate`, `emit-vcs`, and `ci`.
  - [x] Document local dev workflows in `docs/dev.md` (setup, wasm-tools, common commands).
  - [x] Windows MSVC tests: disable PDB generation to avoid LNK1318 (`.cargo/config.toml` uses `/DEBUG:NONE`).
- [x] Error hygiene:
  - [x] Unify error types/messages across crates and keep JSON errors consistent.
  - [x] Add a single error-code table and a test that ensures codes are unique and documented.
- [x] Performance:
  - [x] Preallocate HashMaps/Vecs where sizes are known (builtins, params, funcs) in parser/typer/codegen.
  - [x] Add microbenchmarks for parser/typer/codegen hot paths.
- [x] Release profile:
  - [x] Set `[profile.release]` with `lto = "thin"`, `codegen-units = 1`, optional `strip = "symbols"`.
  - [x] Add perf/regression checks for release profile tuning impacts.

## Phase 14 - Platform & Runtime Decoupling

- [x] Contract/runtime decoupling:
  - [x] Define a pure state-transition core (e.g., `apply(state: Bytes, msg: Bytes) -> Bytes`).
  - [x] Specify a stable, deterministic ABI for `init`/`handle`/`query` plus canonical serialization.
  - [x] Add ABI conformance fixture tests for canonical CBOR envelopes.
  - [x] Model environment services (storage/crypto/time/log) as capability interfaces injected by the runtime.
  - [x] Decide whether capability interfaces are design-only or need runtime import stubs/tests.
  - [x] Use chain-scoped environment types (e.g., `std::eth::Address`, `std::solana::Pubkey`) instead of global `Address`.
  - [x] Keep business-logic utilities in chain packages, not core language features.
  - [x] Gate service calls behind effects so core proofs remain pure.
  - [x] Document the Wasm host import surface and runtime responsibilities.
- [x] Runtime/chain documentation:
  - [x] Publish `docs/runtime/abi.md` (host API, limits, ABI surface).
  - [x] Publish `docs/runtime/chain-packages.md` (chain-scoped types, versioning, evolution policy).
  - [x] Add a short separation-of-concerns section to README + style guide.
- [x] Determinism & metering:
  - [x] Explicit gas/step accounting for loops and recursion; deterministic runtime limits.
  - [x] Forbid nondeterministic APIs by default; gate randomness/time behind `io`.
  - [x] Add io-gated time/random intrinsics (or runtime stubs) and tests to make nondeterminism gating real.
  - [x] Add tests covering metering limits (loop/recursion).
  - [x] Add tests for nondeterminism gate errors.
- [x] WASI `print` intrinsic for observable output (design + implementation + docs/tests).
- [x] Diagnostics polish:
  - [x] (Tracked in Phase 4.10) Emit `P010` for missing `else` in expression-form `if` (parser + tests).
  - [x] Make `docs/diagnostics.md` JSON example strict JSON (move note outside the code block).
- [x] Benchmark follow-ups:
  - [x] Use benches to validate codegen string pre-scan overhead; reduce pass cost if it shows up on hot paths.

## Phase 15 - Crypto + Language Expansion

Ordering: 15 Core types & arrays, 16 Crypto intrinsics + proofs, 17 Language gaps + collections.

### 15 Core types & arrays
- [x] 15.1 Add fixed-width unsigned ints (`U64`, `U128`, `U256`) with explicit overflow semantics (wrap/checked/sat). (Parser, typer, IR, codegen, tests.)
  - [x] Reserve keywords and AST types for unsigned ints; reject usage via T110 until semantics land.
  - [x] 15.1.1 Implement U64 end-to-end (parser/typer/IR/codegen/tests).
- [x] 15.1.2 Add checked-overflow traps + wrap/sat intrinsics for U64.
    - [x] Emit runtime overflow guards + R005 mapping for U64 add/sub/mul.
    - [x] Add wrap_*/sat_* intrinsics for explicit overflow behavior.
  - [x] 15.1.3 Implement U128/U256 limb layout + helpers and enable in typer/codegen.
- [x] 15.1.4 Add contextual literal typing + explicit casts (e.g., `U64(42)`) and range checks.
    - [x] Allow contextual U64 literals and `U64(...)` casts (non-negative only).
    - [x] Add `U128(...)`/`U256(...)` casts and contextual literal coercion beyond U64.
    - [x] Enforce range checks for unsigned literals/casts (reject out-of-range values).
  - [x] Extend negative tests to cover U128/U256 (including nested types like `Option<U128>`).
  - [x] Document checked-overflow default and planned wrap/sat intrinsics (design note).
- [x] 15.2 Add bitwise ops, shifts/rotates, and byte/word conversions. (Syntax, typer rules, codegen, tests.)
  - [x] Add parser precedence for `&`/`|`/`^` and `<<`/`>>`, plus AST/IR ops.
  - [x] Extend typer/lowering/VC rules for bitwise and shift ops (U64-literal coercions only).
  - [x] Add wasm codegen for bitwise/shift ops and U64 rotate + bytes intrinsics.
  - [x] Add parser/typer/codegen tests and update typing/runtime docs.
- [x] 15.3 Introduce fixed-size arrays (e.g., `[U8; 32]`) plus tuples for hash/key pairs. (Parser/AST, typer, layout, codegen, tests.)
  - [x] 15.3.1 Decide element types (add `U8` now or restrict arrays to existing numeric types) and tuple arity/literal syntax.
  - [x] 15.3.2 Define type syntax + typing rules for `[T; N]` and `(T1, T2, ...)` (no indexing yet).
  - [x] 15.3.3 Update AST/parser to accept array/tuple types (and literals if chosen).
  - [x] 15.3.4 Extend typer validations (resource-in-collection checks) and type rendering for arrays/tuples.
  - [x] 15.3.5 Add lowering/codegen representation for arrays/tuples (opaque pointer or inline layout) and minimal runtime helpers.
  - [x] 15.3.6 Add parser/typer/codegen tests + update docs (typing + ABI layout in 15.5).
- [x] 15.4 Add diagnostics + error codes for numeric overflow, literal range checks, and array bounds.
  - [x] Centralize/validate diagnostics-code prefix allowlist in `diagnostics_codes` test.
  - [x] 15.4.1 Define diagnostic codes/messages for overflow, literal range, and array bounds (reserve codes even if features are stubbed).
  - [x] 15.4.2 Wire overflow diagnostics for U64 checked ops; align codes with runtime trap mapping where applicable.
  - [x] 15.4.3 Emit literal range diagnostics for unsigned casts and contextual literals (`U8`/`U64`/`U128`/`U256`).
  - [x] 15.4.4 Add array bounds diagnostics hooks (stub until indexing lands; ensure code exists and tests assert it).
  - [x] 15.4.5 Add unit + JSON snapshot tests for each new diagnostic (positive/negative cases, stable messages).
- [x] 15.5 Specify ABI/layout rules for arrays/tuples in `docs/runtime/abi.md` or a new layout note.

- [x] 15.6 Phase 15 completion checklist
  - [x] 15.6.1 Implement array/tuple lowering + runtime allocation using the Phase 15.5 layout.
  - [x] 15.6.2 Add array/tuple literals and indexing semantics (or explicitly gate behind Phase 17.4 with temporary stubs).
  - [x] 15.6.3 Add bounds checks for array indexing and wire T114 to runtime/typer as appropriate.
  - [x] 15.6.4 Integrate `U8` into numeric ops (typing + codegen + VC rules) or explicitly document its limitations.
  - [x] 15.6.5 Update `docs/typing.md` to reference the finalized arrays/tuples layout and remove stale "opaque" wording.
  - [x] 15.6.6 Add JSON error snapshot tests for new diagnostics (T112/T113/T114).

- [x] 15.7 Post-15 review fixes
  - [x] 15.7.1 Add `U8(...)` lowering so unsigned casts do not fail at build time.
  - [x] 15.7.2 Align nested array/tuple element layout with the pointer-based ABI decision (size/align 4 when nested).
  - [x] 15.7.3 Add a dedicated diagnostic for non-constant tuple indices (avoid using T114 for this case).
  - [x] 15.7.4 Add a runtime test that traps on dynamic out-of-bounds array indices.
  - [x] 15.7.5 Update Phase 17.4 tech-debt note after the fixes above are complete.

### 16 Crypto intrinsics + proofs
  - [x] 16.1 Hashes (SHA-256, Keccak, Blake2) and HMAC primitives with deterministic semantics. (API spec, builtins, codegen stubs, vectors/tests.)
    - [x] 16.1.1 Define API surface (module path, signatures, effects) and algorithm identifiers.
    - [x] 16.1.2 Update docs: `docs/runtime/host-imports.md`, `docs/typing.md`, `docs/diagnostics.md`.
    - [x] 16.1.3 Typer: add builtins and effect gating for hash/HMAC intrinsics.
    - [x] 16.1.4 IR/codegen: add intrinsics and Wasm import plumbing.
    - [x] 16.1.5 Runtime stubs for `clg run` with deterministic behavior.
    - [x] 16.1.6 Tests: vector-based positive cases and invalid alg/length diagnostics.
  - [x] 16.2 Signature verification (ed25519/secp256k1) with strict input validation. (API spec, builtins, codegen stubs, tests, error codes.)
    - [x] 16.2.1 Define API surface (return type, inputs, effect) and algorithm identifiers.
    - [x] 16.2.2 Typer: add builtins and effect gating for signature verification.
    - [x] 16.2.3 IR/codegen: add intrinsic and Wasm import plumbing.
    - [x] 16.2.4 Runtime stubs for `clg run` with deterministic behavior.
    - [x] 16.2.5 Tests: valid signature, wrong key/sig, bad alg/length, error codes.
  - [x] 16.3 Constant-time byte equality helper for secret comparisons. (API spec, intrinsic, tests.)
    - [x] 16.3.1 Decide implementation path (pure helper vs host import) and document the choice.
    - [x] 16.3.2 Implement helper/intrinsic and wire into IR/codegen as needed.
    - [x] 16.3.3 Tests: correctness and constant-time guardrails where possible.
  - [x] 16.4 Define deterministic error semantics + diagnostics for crypto intrinsics (invalid length/alg/etc.).
    - [x] 16.4.1 Enumerate error taxonomy (invalid alg, length, unsupported curve, malformed input).
    - [x] 16.4.2 Add diagnostics codes + JSON examples in `docs/diagnostics.md`.
    - [x] 16.4.3 Tests: JSON error outputs remain stable for each error class.
  - [x] 16.5 SMT encoding for modular arithmetic/bitwise ops (or explicit assumed axioms for crypto intrinsics).
    - [x] 16.5.1 Decide modeling strategy (axioms vs encoding) and document assumptions.
    - [x] 16.5.2 Update SMT/VC encoder and refresh snapshots.
  - [x] 16.6 Document proof limitations for cryptographic primitives in `docs/proofs`.
    - [x] 16.6.1 Add `docs/proofs/crypto-limitations.md` and link from `docs/typing.md`.
  - [x] 16.7 On-chain attestation: design anchoring flow (EVM registry + IPFS URIs), minimal reference impl, and docs.
    - [x] 16.7.1 Draft design note: registry schema, payload format, and verification flow (`docs/design/phase-16.7-attestation.md`).
    - [x] 16.7.2 Minimal reference implementation (Solidity contract + sample payload).
    - [x] 16.7.3 Documentation and example workflow.
    - [x] 16.7.4 Production migration checklist (see `docs/rollout/attestation-production.md`, Phase 18 hardening reminder).

### 17 Language gaps + collections
- [x] 17.1 Implement user-defined structs/enums (beyond resource types) with pattern matching.
  - [x] 17.1.1 Spec + docs: syntax, layout model, visibility, and resource interactions (`docs/design/phase-17.1-structs-enums.md`).
  - [x] 17.1.2 Parser/AST support for `struct`/`enum`, field access, and construction.
  - [x] 17.1.3 Typer: field/variant checking, pattern matching, exhaustiveness, and unreachable arms.
  - [x] 17.1.4 Lowering/codegen: concrete layout + tag/payload strategy.
  - [x] 17.1.5 Runtime/ABI notes + tests for constructors, match, and layout.
- [x] 17.2 Add generics and interface abstractions beyond built-in ADTs.
  - [x] 17.2.1 Design: generics syntax, interface bounds, coherence rules, monomorphization strategy, and canonical instantiation mangling for debug/proof metadata.
  - [x] 17.2.2 Parser/AST: type params, bounds, and implementation blocks. (Scaffolding only; no typechecking/interface resolution yet.)
  - [x] 17.2.3 Typer: inference, interface resolution, and error diagnostics.
  - [x] 17.2.4 Lowering/codegen: monomorphization or dictionary passing + caching.
  - [x] 17.2.5 Stdlib updates + regression tests.
- [x] 17.3 Provide real collections runtime semantics for `List`/`Map`/`Set` (not just typing stubs).
  - [x] 17.3.1 Design note: layout + semantics (pure vs mut), `can_mut` behavior, key equality strategy, and error taxonomy (see `docs/design/phase-17.3-collections-runtime.md`).
  - [x] 17.3.2 Runtime/ABI: implement list/set/map heap layouts, allocation/growth helpers, and deterministic traps (bounds/invalid handle/oom).
  - [x] 17.3.3 Compiler wiring: add IR/lowering/codegen intrinsics, enforce key constraints, and keep mut guard + VC integration aligned with runtime behavior.
  - [x] 17.3.4 Tests for correctness, guard behavior, and JSON diagnostics for runtime errors.
  - [x] 17.3.5 Enforce equatable key constraints for `Map`/`Set` in the typer (T220) with regression tests.
  - [x] 17.3.6 Add `R009` collection bounds trap plumbing (IR + CLI) with a runtime trap test.
  - [x] 17.3.7 Allow `std::list::new`/`std::set::new`/`std::map::new` to infer from expected types (keep T206 when no expected context).
  - [x] 17.3.8 Implement structural equality for non-primitive `Map`/`Set` keys (Option/Result/structs/enums/tuples; arrays remain deferred) and add runtime coverage.
  - [x] 17.3.9 Add runtime tests for `list::get`/`list::pop`, `set::contains`, `map::contains`/`map::get`, and no-op remove cases.
  - [x] 17.3.10 Docs refresh: update `docs/collections.md` + `docs/design/phase-17.3-collections-runtime.md` to reflect `new()` inference and current `can_mut` behavior.
  - [x] 17.3.11 Clarify invalid-handle behavior for collections and align trap code/docs (R002 vs new code) or add explicit pointer validation.
  - [x] 17.3.12 Use unsigned bounds checks in collection handle validation to avoid signed i32 overflow in pointer comparisons.
  - [x] 17.3.13 Close remaining collections runtime test gaps (invalid `data_ptr` cases + composite key equality coverage).
  - [x] 17.3.14 Treat header inconsistencies (len > cap / cap <= 0) as invalid handles (R010).
  - [x] 17.3.15 Guard against `len * stride` overflow and add Set/Map header-consistency tests.
- [x] 17.4 Add general array/slice types with indexing semantics and bounds checks.
  - [x] 17.4.1 Spec: array/slice syntax, indexing semantics, and bounds behavior (see `docs/design/phase-17.4-arrays-slices.md`).
  - [x] 17.4.2 Parser/AST + typer support for arrays/slices, `[T; N]` sugar, and indexing.
  - [x] 17.4.3 Lowering/codegen for layout + bounds checks.
  - [x] 17.4.4 Runtime helpers + tests (`std::array::len`, `std::slice::{from_array,sub}`, array/slice bounds + length-guard coverage).
  - Note: `[T; N]` is sugar over `Array<T>` with length-contract guards; docs/ABI updated accordingly.
- [x] 17.5 Add module/import system with visibility controls for libraries.
  - [x] 17.5.1 Design: file layout, module paths, visibility keywords, and re-exports.
  - [x] 17.5.2 Parser/AST for `import`/`export` and namespace nodes.
  - [x] 17.5.3 Typer: module graph resolution + name shadowing diagnostics.
  - [x] 17.5.4 Build system updates (module discovery, caching, error spans).
  - [x] 17.5.5 Tests + docs walkthrough.
  - [x] 17.5.6 Std/chain package export metadata (type/value classification for `std` imports).
  - [x] 17.5.7 Std/chain type layout metadata + value semantics (byte-wise equality).
  - [x] 17.5.8 Std/chain value constructors (`from_bytes`/`from_array`) + runtime tests + doc updates.
- [x] 17.6 Linear-aware collections: design note, effects/VC plan, and phased prototype.
  - [x] 17.6.1 Design: ownership rules for collections of resources.
    - [x] 17.6.1.1 Draft `docs/design/phase-17.6-linear-aware-collections.md` with goals/non-goals and safety invariants.
    - [x] 17.6.1.2 Define ownership/alias rules for `List<Resource>`, `Set<Resource>`, and `Map<K, Resource>` (insert/get/remove/iterate).
    - [x] 17.6.1.3 Specify consume/borrow behavior at collection boundaries (move-in, move-out, forbidden copy paths).
    - [x] 17.6.1.4 Define deterministic diagnostics + trap mapping for linear violations at type-check and runtime boundaries.
  - [x] 17.6.2 Typer rules + diagnostics for linear-aware collection ops.
    - [x] 17.6.2.1 Implement linear state tracking for resource values stored in collections.
    - [x] 17.6.2.2 Add targeted diagnostics for use-after-move, double-consume, and invalid borrow across collection calls.
    - [x] 17.6.2.3 Add regression tests for accepted and rejected flow patterns (insert/remove/contains/get + match/control-flow joins).
    - [x] 17.6.2.4 Adopt transitive ownership for tuple wrappers around linear values; align typer validation, tests, and docs.
    - [x] 17.6.2.5 Apply transitive ownership to wrapper constructors/literals (`Some`/`Ok`/`Err`/tuples) and add regression tests.
  - [x] 17.6.3 VC/effect integration plan + prototype tests.
    - [x] 17.6.3.1 Extend VC obligations so collection operations preserve linear invariants across branches/loops.
    - [x] 17.6.3.2 Align `pure`/`mut` effect gates with linear collection APIs and document proof/runtime split.
    - [x] 17.6.3.3 Add prototype fixtures (`--emit-vcs`) and runtime tests for representative linear-collection workflows.
- [x] 17.7 First-class functions and closures (if ClearLang is to be general-purpose).
  - [x] 17.7.1 Design: function types, capture semantics, and effect annotations (see `docs/design/phase-17.7-closures.md`).
    - [x] 17.7.1.1 Draft `docs/design/phase-17.7-closures.md` with goals/non-goals and a design-principles check (`simple for users`, `AI-friendly`, `provably correct`, `crypto-focused`).
    - [x] 17.7.1.2 Decide minimal function-type and lambda syntax that stays familiar and parser-deterministic.
    - [x] 17.7.1.3 Define capture semantics (`by value` vs borrow), including linear/resource capture restrictions.
    - [x] 17.7.1.4 Specify effect and contract behavior for closure creation/invocation plus VC obligations.
    - [x] 17.7.1.5 Define deterministic diagnostics and a parser/typer acceptance matrix before 17.7.2 starts.
  - [x] 17.7.2 Parser/AST: lambdas, capture lists (if any), and type annotations.
    - [x] 17.7.2.1 Parse `function(T1, ...) -> R` function types and `(x: T, ...) => expr` lambdas.
    - [x] 17.7.2.2 Require lambda parameter type annotations in v1; reject missing types with parser diagnostics.
    - [x] 17.7.2.3 Do not add capture-list syntax in Phase 17; reject any capture-list-like forms clearly.
  - [x] 17.7.3 Typer: closure typing, lifetime/capture checks, and effect compatibility.
    - [x] 17.7.3.1 Infer lambda return types (no explicit lambda return-type annotation in Phase 17).
    - [x] 17.7.3.2 Enforce lexical capture rules and reject linear/resource captures with deterministic diagnostics.
    - [x] 17.7.3.3 Reject self-referential and mutually recursive closure values in v1 (named-function recursion remains under existing totality rules).
    - [x] 17.7.3.4 Conservatively effect-check calls through function values (unknown callee effect requires `io`).
    - [x] 17.7.3.5 Emit explicit closure recursion diagnostics (self/mutual cycles) instead of fallback unknown-function errors.
  - [x] 17.7.4 Lowering/codegen: closure environment layout + call ABI.
    - [x] 17.7.4.1 Lower closures to `{ code_id, env_ptr }` runtime records with hidden `env_ptr` invoke argument. (invoke ABI now wired through generated lambda bodies + dispatcher calls)
    - [x] 17.7.4.2 Use `env_ptr = 0` for non-capturing lambdas while preserving one invoke ABI. (validated via lowering + runtime dispatch tests)
    - [x] 17.7.4.3 Implement signature-specific dispatcher wrappers for dynamic closure calls; do not add `call_indirect` table dispatch in Phase 17. (dispatcher generation and call patching landed)
    - [x] 17.7.4.4 Allocate closure environments with the shared bump allocator and keep no-deallocation semantics in v1.
  - [x] 17.7.5 Tests + docs examples. (added runtime unknown-`code_id` trap coverage and typing guide examples/rules)
  - [x] 17.8 Interface follow-ups (post-17.2).
   - [x] 17.8.1 Default interface method bodies with explicit effect checking and override rules.
     - [x] 17.8.1.0 Design lock + acceptance matrix before parser changes.
        - [x] Add `docs/design/phase-17.8-trait-defaults.md` with syntax, typing, and non-goals.
        - [x] Include a design-principles check from README (`simple for users`, `AI-friendly`, `provably correct`, `crypto-focused`).
        - [x] Freeze v1 scope: declaration or default-body interface methods only; no dynamic dispatch or interface-state features.
        - [x] Define deterministic diagnostics for unsupported forms and effect mismatches.
     - [x] 17.8.1.1 Parser/AST: allow interface methods to use either declaration form (`...;`) or default-body form (`... { ... }`).
       - [x] AST: represent an optional default body on interface methods without regressing existing interface signatures.
       - [x] Parser: accept both forms while preserving deterministic parsing for interface blocks.
       - [x] Diagnostics: reject mixed/invalid method forms with stable parser codes and spans.
       - [x] Tests: add parser positive/negative coverage for both forms and malformed default bodies.
     - [x] 17.8.1.2 Typer: permit implementations to omit methods only when the interface provides defaults; keep missing-method diagnostics for non-default methods.
     - [x] 17.8.1.3 Effect rules: require default-body effect to match the declared interface-method effect exactly.
     - [x] 17.8.1.4 Override signature/effect matching remains enforced for implementation-provided methods (baseline already implemented in 17.2).
     - [x] 17.8.1.5 Tests/docs: parser + typer positive/negative coverage for defaults, missing overrides, and effect mismatches.
    - [x] 17.8.4 Hard switch keyword surface: `trait`/`impl` -> `interface`/`implementation` (pre-release, no compatibility shim).
      - [x] 17.8.4.1 Parser/keywords: replace accepted declaration keywords (`trait` -> `interface`, `impl` -> `implementation`) and keep grammar deterministic.
      - [x] 17.8.4.2 Reserved words: update identifier rejection lists and keyword helpers to reserve the new words; drop old ones from accepted grammar.
      - [x] 17.8.4.3 Diagnostics/docs wording: update parser/typer messages and docs/examples to use `interface`/`implementation` terminology.
      - [x] 17.8.4.4 Tests/fixtures migration: rewrite parser/typer/integration samples using old keywords and keep coverage equivalent.
      - [x] 17.8.4.5 One-shot migration sweep: repo-wide source + docs rename and final validation run (`clg-parser`, `clg-typer`, `clg-cli` tests).
    - [x] 17.8.2 Optional explicit implementation selection syntax (only if coherence is relaxed).
      - [x] 17.8.2.1 Keep strict coherence as default behavior (baseline from 17.2); explicit implementation selection remains unsupported while coherence is strict.
        - [x] Lock conflict policy: overlapping implementations are compile-time conflicts (`T236`), and ambiguous resolution is a compile-time error (`T248`) with no implicit fallback.
    - [x] 17.8.3 Proof/debug name shortening (optional hash suffix for long mangled names).
      - [x] 17.8.3.1 Add an optional deterministic shortening mode for long mangled names (hash suffix).
      - [x] 17.8.3.2 Preserve uniqueness/collision safety guarantees and stable reproducibility.
      - [x] 17.8.3.3 Keep proof/debug traceability by documenting and testing the shortened-name mapping.
  - [x] 17.9 Remaining language gaps (post-17.8).
    - [x] 17.9.1 Generic call-site explicit type arguments (`f<T>(...)`) end-to-end.
      - [x] 17.9.1.1 Parser/AST: support optional call-site type-argument lists on calls.
      - [x] 17.9.1.2 Typer: integrate explicit type args with inference fallback and deterministic ambiguity errors.
      - [x] 17.9.1.3 Monomorphization/lowering: wire explicit call-site instantiations and add regression coverage.
      - [x] 17.9.1.4 Docs/tests: update examples and diagnostics guidance for explicit generic calls.
    - [x] 17.9.2 Map/Set key-equatability parity for arrays.
      - [x] 17.9.2.1 Decide policy: support `Array<T>` keys when `T` is equatable, or explicitly document exclusion.
      - [x] 17.9.2.2 Align typer/runtime equality implementation with the decided policy; add positive/negative tests.
      - [x] 17.9.2.3 Reconcile `docs/TODO.md` and design/docs wording for 17.3.8 array-key expectations.
    - [x] 17.9.3 Module ergonomics follow-ups (language surface).
      - [x] 17.9.3.1 Re-exports (`export import`) explicitly disallowed for v1 and deferred.
        - [x] Rationale: keep module resolution explicit/deterministic and avoid transitive export ambiguity; revisit post-v1 only if facade use cases justify added complexity.
        - [x] 17.9.3.1.1 Diagnostics: reject `export import` with explicit parser error code/message (P011) instead of only generic parse failure.
      - [x] 17.9.3.2 Item import aliasing (`import m::{A as B}`) explicitly disallowed for v1 and deferred.
        - [x] Rationale: keep imports explicit and minimal in v1; module aliasing (`import m as x`) and fully qualified paths already cover naming needs without adding item-level alias semantics.
      - [x] 17.9.3.3 Glob imports (`*`) explicitly disallowed for v1 and deferred.
        - [x] Rationale: avoid implicit name injection/shadowing and keep resolution + diagnostics deterministic and AI-friendly.
    - [x] 17.9.4 Resource user-defined type surface.
      - [x] 17.9.4.1 Implement parser/typer rules for `resource struct`/`resource enum`.
        - [x] Parser: accept `resource struct` and `resource enum` declarations.
        - [x] Typer: allow resource fields only on resource-marked structs/enums; keep `T218` for non-resource structs/enums containing resource fields.
      - [x] 17.9.4.2 Ownership/linearity interaction tests across collections, pattern matching, and closures.
    - [x] 17.9.5 Closure language-surface follow-ups (v1 restriction revisit).
      - [x] 17.9.5.1 Explicit capture-list syntax rejected for v1 with stable diagnostics.
        - [x] Parser diagnostics: emit dedicated code `P012` for capture-list forms (`[x](...) => ...`) instead of generic parse-only reporting.
      - [x] 17.9.5.2 Keep self/mutual closure recursion rejected in v1 with deterministic typer behavior.
        - [x] Typer diagnostics: mutual-recursion pair names are rendered in canonical order, independent of declaration order.
      - [x] 17.9.5.3 Closure environment reclamation strategy: keep no-free policy for this product line.
        - [x] Decision: closure environments remain module-instance-lifetime allocations; dropping closure values does not reclaim memory.
        - [x] Operational guidance: long-running hosts should recycle module instances/workers to bound closure-environment memory growth.

### 18 Production hardening + attestation
- [x] 18.0 Real-world readiness priorities (execution order).
  - [x] 18.0.0 Undecided open questions (must be resolved before proceeding with 18.0.1+ execution gates).
    - [x] 18.0.0.0 Decision lock published: `docs/design/phase-18.0-open-questions.md`.
    - [x] 18.0.0.1 Interface/implementation generics policy (`T246`/`T245`): v1 restriction vs bounded support scope.
    - [x] 18.0.0.2 Generic refinement aliases (`T244`): keep deferred vs introduce bounded substitution strategy.
    - [x] 18.0.0.3 Collection backend strategy: keep deterministic linear-search `Map`/`Set` vs introduce deterministic hashing profile.
    - [x] 18.0.0.4 Unsigned arithmetic scope: `U128`/`U256` modeling depth required for production proof workloads.
    - [x] 18.0.0.5 Bitwise/shift proof model: full SMT encoding vs explicit assumption boundaries.
    - [x] 18.0.0.6 Crypto proof model: axiom-based baseline vs stronger per-primitive encodings.
    - [x] 18.0.0.7 Inline refinements on params/returns: keep alias-only ergonomics vs add inline surface now.
    - [x] 18.0.0.8 Deferred resource/type-surface limits: `Set<Resource>`, resource arrays/slices, array-key equality.
    - [x] 18.0.0.9 External trust anchor choice for `verify` mode (Lean vs Coq first) and version pinning policy.
    - [x] 18.0.0.10 Assurance policy baseline: explicit acceptance criteria per tier (`L0`-`L3`) for production release gates.
  - [x] 18.0.1 P0 Security gate: complete 18.1 + 18.3 before production rollout.
  - [x] 18.0.2 P0 Reliability gate: complete 18.2 data availability/retention policy with backup + restore drills.
  - [x] 18.0.3 P1 Ecosystem gate: complete 18.4 compiled package/module imports for reusable libraries.
  - [x] 18.0.4 P1 Runtime-ops gate: publish host runbook for closure-env no-free policy (worker recycle, memory budgets, monitoring alerts).
    - [x] 18.0.4.1 Publish runbook skeleton in `docs/rollout/closure-env-ops-runbook.md` (scope, required inputs, evidence artifacts).
    - [x] 18.0.4.2 Define default worker recycle policy (time-based + memory-threshold triggers) and override knobs.
    - [x] 18.0.4.3 Define memory-budget tiers and per-tier action policy (warn, drain, recycle, incident).
    - [x] 18.0.4.4 Define monitoring + alerts for closure-env growth and recycle failures (signal list, thresholds, paging severity).
    - [x] 18.0.4.5 Add design-principles gate to runbook sign-off (`simple for users`, `AI-friendly`, `provably correct`, `crypto-focused`) and link verification evidence.
  - [x] 18.0.5 P2 Language ergonomics execution (after 18.0.0 decisions are locked).
    - [x] 18.0.5.1 For each accepted ergonomics change from 18.0.0, publish a design lock (syntax, typing, diagnostics, non-goals) before implementation. (`docs/design/phase-18.0.5-language-ergonomics.md`)
    - [x] 18.0.5.2 For each rejected/deferred ergonomics item from 18.0.0, add explicit parser/typer diagnostics and docs rationale (no silent fallback behavior). (`P013`, `T244`, `T245`, `T246`, `T806`; see `docs/design/phase-18.0.5-language-ergonomics.md`)
    - [x] 18.0.5.3 Implement accepted changes with deterministic resolution rules and migration tests/fixtures. (migration fixtures under `clearlang-tests/migration/` + CLI IT coverage in `crates/cli/tests/cli_it/diagnostics.rs`)
    - [x] 18.0.5.4 Add SDK usability checks (import ergonomics, error quality, migration friction) and fail gates if metrics regress. (CLI IT gates in `crates/cli/tests/cli_it/sdk_usability.rs` + CI step `SDK usability gates`)
  - [x] 18.0.6 P2 Proof-model execution (after 18.0.0 decisions are locked).
    - [x] 18.0.6.1 Implement selected unsigned/bitwise/crypto modeling paths and record exact assumption boundaries in emitted artifacts.
    - [x] 18.0.6.2 Add proof-regression CI suites so assurance tiers cannot silently downgrade on existing fixtures.
    - [x] 18.0.6.3 Publish and maintain a proof-coverage matrix per language feature/intrinsic (`proved` vs `assumed`, mapped to `L0`-`L3`).
    - [x] 18.0.6.4 Roll strict-mode defaults forward only after the proof-coverage matrix and CI gates are green.
- [x] 18.1 Attestation registry hardening (authz, key rotation, revocation, schema versioning).
  - [x] 18.1.1 Contract: owner-controlled signer authorization, signer deauthorization (rotation), and schema-version allowlist.
  - [x] 18.1.2 Contract: canonical `attestation_id` validation and explicit revocation flow (signer or owner).
  - [x] 18.1.3 Tests/docs: hardening behavior coverage and updated registry workflow docs (`contracts/attestation/*`, `docs/design/phase-18.1-attestation-hardening.md`).
- [x] 18.2 Data availability policy (pinning/backup/retention) for attestation payloads.
  - [x] 18.2.1 Policy lock: storage topology, integrity invariants, backup cadence, and retention classes (`docs/design/phase-18.2-attestation-data-availability.md`).
  - [x] 18.2.2 Recovery objectives: explicit RPO/RTO thresholds and release evidence requirements.
  - [x] 18.2.3 Operations drill runbook: monthly backup/restore drill with pass/fail criteria (`docs/rollout/attestation-da-drill.md`).
- [x] 18.3 Security review + fuzzing for attestation contract and payload validation.
  - [x] 18.3.1 Contract fuzz/property tests for authorization, schema gating, ID mismatch, and revocation authorization.
  - [x] 18.3.2 Payload envelope validation checks with negative coverage (schema/version/address/hex/scope consistency).
  - [x] 18.3.3 Security review note with resolved findings and residual-risk tracking (`docs/design/phase-18.3-attestation-security-review.md`).
- [x] 18.4 Compiled module/package import support (artifact metadata, versioning, and resolver flow).

### 19 High-assurance ergonomics (easier alternative to Coq/Agda/Lean/F*)
- [x] 19.0 Target and guardrails (execution order).
  - [x] 19.0.1 Define ClearLang success target: theorem-prover-grade assurance for bounded program classes, with lower user complexity. (`docs/design/phase-19.0.1-success-target.md`)
  - [x] 19.0.2 Keep README design-principles gate on all 19.x changes (`simple for users`, `AI-friendly`, `provably correct`, `crypto-focused`). (`docs/design/phase-19.0.2-design-principles-gate.md`, `crates/cli/tests/phase19_design_principles.rs`)
  - [x] 19.0.3 Ship Phase 19 behind explicit compiler modes so production users can choose strictness without ambiguity. (`docs/design/phase-19.0.3-compiler-modes.md`, `--compiler-mode {permissive,standard,strict}`, diagnostics `C029`/`C030`)
  - [x] 19.0.4 Document explicit non-goal: do not claim universal proof-power superiority over Coq/Agda/Lean/F*; target practical production assurance with simpler UX. (`docs/design/phase-19.0.4-non-goal-clarity.md`)
- [x] 19.1 Assurance levels + trust boundary model (no hidden assumptions).
  - [x] 19.1.1 Introduce explicit assurance tiers in diagnostics/artifacts (`L0` assumed, `L1` checked core, `L2` verified module, `L3` verified package profile). (`docs/design/phase-19.1.1-assurance-tiers.md`, `docs/proofs/vc-schema.md`, `docs/proofs/proof-section.md`)
  - [x] 19.1.2 Require every non-proved primitive/external dependency to be labeled as `assumed` in emitted reports. (`docs/design/phase-19.1.2-assumed-dependency-labeling.md`, `primitive.unproved`, `external.dependency`)
  - [x] 19.1.3 Fail closed in strict mode: block `L3` claims if any unlabeled assumptions remain. (`docs/design/phase-19.1.3-strict-l3-fail-closed.md`, diagnostic `C031`)
  - [x] 19.1.4 Add external trust-anchor integration for compile-time `verify` mode (pinned Lean/Coq checker versions), while keeping runtime bundles kernel-free. (`docs/design/phase-19.1.4-trust-anchor-compile-time-verify.md`, diagnostics `C032`/`V004`)
- [x] 19.2 Proof coverage completion for current language surface.
  - [x] 19.2.1 Close unsigned/bitwise SMT gaps (`U128`/`U256`, shifts, masks) or downgrade affected checks to explicit assumptions. (`docs/design/phase-19.2.1-unsigned-bitwise-gap-closure.md`, updated `bitwise.uninterpreted` downgrade coverage for bitwise-sensitive `std::u64` intrinsics)
  - [x] 19.2.2 Add deterministic modeling policy for crypto intrinsics with per-intrinsic assurance level in outputs. (`docs/design/phase-19.2.2-crypto-intrinsic-modeling-policy.md`, `assumptions.items[].intrinsic_levels` in VC JSON and proof-section outputs)
  - [x] 19.2.3 Complete refinement ergonomics needed for production proofs (inline param/return refinements + generic refinement aliases if sound). (`docs/design/phase-19.2.3-refinement-ergonomics-production-proofs.md`, inline refinement normalization + generic alias instantiation support)
- [x] 19.3 Verified-by-construction standard profile.
  - [x] 19.3.1 Define a `strict` language profile that forbids unsupported/deferred constructs and external unchecked calls by default. (`docs/design/phase-19.3.1-strict-language-profile.md`, strict-mode assumed-boundary fail-closed diagnostic `C033`)
  - [x] 19.3.2 Publish a "verified std/core subset" with proof-backed contracts and regression obligations. (`docs/design/phase-19.3.2-verified-std-core-subset.md`, `docs/proofs/verified-std-core-subset.md`, `docs/proofs/verified-std-core-subset.json`, regression test `crates/cli/tests/verified_std_core_subset.rs`)
  - [x] 19.3.3 Add CI gate: no profile regression if a change lowers assurance level for existing fixtures. (`docs/design/phase-19.3.3-profile-regression-ci-gate.md`, fixture manifest `docs/proofs/verified-profile-fixtures.json`, CI gate `cargo test -p clg-cli --test profile_regression_gate`)
- [x] 19.4 Usability-first proof workflow (the "easier" part).
  - [x] 19.4.1 Add diagnostic hints that suggest the minimal contract/invariant needed to discharge each failed VC. (`docs/design/phase-19.4.1-vc-diagnostic-hints.md`, VC JSON `diagnostics.repair_hints`, CLI IT coverage in `crates/cli/tests/cli_it/vc_outputs.rs`)
  - [x] 19.4.2 Add proof-failure slicing/counterexample reporting that maps directly to user source spans. (`docs/design/phase-19.4.2-proof-failure-slicing-counterexample-reporting.md`, VC JSON `diagnostics.failure_slice` + `diagnostics.counterexample`, CLI IT/snapshot coverage)
  - [x] 19.4.3 Provide AI-oriented machine-readable proof context bundle (`VC`, assumptions, model snippet, span map). (`docs/design/phase-19.4.3-ai-proof-context-bundle.md`, VC JSON `diagnostics.proof_context`, CLI IT/snapshot coverage)
- [x] 19.5 Explainable assurance artifacts for audits.
  - [x] 19.5.1 Emit a signed assurance manifest per build (levels, assumptions, dependency trust labels, toolchain fingerprint). (`docs/design/phase-19.5.1-signed-assurance-manifest.md`, `--assurance-manifest-out`, diagnostic `C034`)
  - [x] 19.5.2 Add `clg verify --explain` summary output for humans (what is proved, what is assumed, why). (`docs/design/phase-19.5.2-verify-explain-summary.md`, `--explain` on `clg verify`, signing IT coverage)
  - [x] 19.5.3 Add policy checks for release pipelines (reject manifests below required assurance tier). (`docs/design/phase-19.5.3-release-policy-gates.md`, `clg verify --assurance-manifest --release-policy`, diagnostic `V005`)

### 20 Runnable Namespace Baseline + Lock Gates (milestone_2)
- [x] 20.0 Add language comment syntax support (AI-friendly, deterministic).
  - [x] 20.0.1 Parser/lexer: add line comments `// ...` and block comments `/* ... */` with deterministic tokenization and spans.
  - [x] 20.0.2 Define and document one canonical style for generated code/docs (`//` preferred; `/* ... */` only for multi-line notes).
  - [x] 20.0.3 Add parser + CLI tests proving comments are accepted in runnable fixtures and do not affect diagnostics stability.
  - [x] 20.0.4 Numeric literal readability: support `_` as digit separator (`1_000`), keep `,` invalid (`1,000`), and add a targeted diagnostic suggesting `_`.
- [x] 20.1 Publish post-19 std/package architecture lock (precompiled `std::core` + host-backed `std::host` + chain packages).
  - [x] 20.1.0 Bootstrap minimal strict lockfile support for gate preflight (direct dependencies only).
    - [x] 20.1.0.1 Define minimal lockfile v0 schema for strict mode (`name`, `version`, `digest`) covering direct dependencies only. (`docs/design/phase-20.1.0-strict-lockfile-v0.md`)
    - [x] 20.1.0.2 Implement deterministic strict-mode lockfile loader/validator for v0 schema (no transitive solver in 20.1).
    - [x] 20.1.0.3 Emit stable diagnostics for missing/malformed strict lockfile input and document remediation.
    - [x] 20.1.0.4 Define minimal trust-policy v0 for strict gates (trusted signer set + revocation/expiry checks) as a bootstrap before 22.0.4 lifecycle expansion. (`docs/design/phase-20.1.0-trust-policy-v0.md`)
    - [x] 20.1.0.5 Define minimal host-profile v0 schema/capability set consumed by strict preflight before 24.0.2 profile expansion. (`docs/design/phase-20.1.0-host-profile-v0.md`)
    - [x] 20.1.0.6 Implement deterministic loaders/validators for trust-policy v0 and host-profile v0 with stable diagnostics.
    - [x] 20.1.0.7 Define minimal package metadata schema v0 + ABI contract v0 (direct dependencies only) required by strict preflight before 22.0.3 policy expansion. (`docs/design/phase-20.1.0-package-metadata-abi-v0.md`)
    - [x] 20.1.0.8 Implement deterministic metadata/ABI v0 validators and stable diagnostics for schema/ABI mismatches.
  - [x] 20.1.1 Design lock document for long-term final solution (`docs/design/phase-20.0-std-packaging-runtime-linking.md`).
  - [x] 20.1.2 Define deterministic acceptance gates for package trust and runtime linker behavior in strict mode.
    - [x] 20.1.2.1 Strict-mode source-of-truth gate: resolve packages only from lockfile + trusted local store (no implicit network fetch).
    - [x] 20.1.2.2 Artifact identity gate: require exact `(name, version, digest)` match against lockfile entries; digest mismatch fails closed.
    - [x] 20.1.2.3 Trust gate: require valid package signature against configured trust anchors from trust-policy v0; untrusted/revoked/expired signer fails closed.
    - [x] 20.1.2.4 Metadata/schema gate: package metadata schema version must be accepted; unknown/unsupported schema fails with stable diagnostics.
    - [x] 20.1.2.5 ABI/link gate: imported symbols must match expected signature/effect/capability profile exactly; ABI mismatch fails deterministically.
    - [x] 20.1.2.6 Runtime capability gate: required host capabilities for linked imports must be present in selected host-profile v0, otherwise fail closed.
    - [x] 20.1.2.7 Determinism gate: identical inputs (source, lockfile, package store, policy) produce identical resolved direct-dependency import map and diagnostics ordering.
    - [x] 20.1.2.8 CI functional acceptance suite: add positive + tamper negative tests for 20.1.2.1-20.1.2.6 with fixed diagnostic-code assertions.
  - [x] 20.1.3 Implement strict-mode gate evaluator (code path, no resolver expansion yet).
    - [x] 20.1.3.1 Define a single preflight input model (lockfile v0 entries, package metadata, trust-policy v0, host-profile v0).
    - [x] 20.1.3.2 Implement a pure evaluator (`evaluate_strict_gates`) that returns deterministic, stably ordered violations.
    - [x] 20.1.3.3 Wire evaluator into `clg build --compiler-mode strict` before final link/build outputs.
    - [x] 20.1.3.4 Add deterministic ordering rule (gate id -> package id -> symbol id) and snapshot tests.
    - [x] 20.1.3.5 Ensure strict mode fails closed whenever any gate violation exists, emitting a complete deterministic violation list (no permissive fallback path).
    - [x] 20.1.3.6 Emit canonical direct-dependency import-map artifact in strict preflight and assert deterministic serialization/hash across identical inputs.
  - [x] 20.1.4 Publish diagnostic + fixture matrix for 20.1 gates.
    - [x] 20.1.4.1 Publish proposed package/linker strict-gate diagnostics (`C101`-`C108`) in the phase design lock (`docs/design/phase-20.0-std-packaging-runtime-linking.md`).
    - [x] 20.1.4.2 Publish canonical fixture matrix in the phase design lock (positive/trust-fail/digest-fail/schema-fail/abi-fail/capability-fail/determinism for direct dependencies).
    - [x] 20.1.4.3 Promote `C101`-`C108` to canonical diagnostics registry in `docs/diagnostics.md` and keep diagnostics code-table tests green (`crates/cli/tests/diagnostics_codes.rs`).
    - [x] 20.1.4.4 Add CI determinism replay job that runs strict preflight twice with identical inputs and asserts identical diagnostics ordering.
    - [x] 20.1.4.5 In the same replay job, assert strict preflight import-map artifact bytes/hash are identical.
- [x] 20.2 Namespaced runnable baseline first: make `clearlang-tests/16_namespaced_call.clear` runnable (not parse-only).
  - [x] 20.2.1 Replace the unresolved `std::math::add` usage with a valid namespaced callable path in a runnable fixture layout.
  - [x] 20.2.2 Add CLI integration coverage asserting `clg run ...16_namespaced_call.clear` succeeds.
  - [x] 20.2.3 Update sample docs to distinguish parse-only vs runnable namespace examples.

### 21 Precompiled Std Core Packaging
- [x] 21.0 Precompiled `std::core` package pipeline.
  - [x] 21.0.1 Implement the versioned std-core artifact pipeline and metadata model with reproducible hash semantics. (`docs/design/phase-21.0-std-core-package-surface.md`)
  - [x] 21.0.2 Implement import pruning so only used package functions are emitted as imports in app Wasm. (`docs/design/phase-21.0-std-core-package-surface.md`)
  - [x] 21.0.3 Reconcile std-core function surface lock with shipped std metadata.
    - [x] 21.0.3.1 Align `docs/design/phase-21.0-std-core-package-surface.md` and `crates/cli/assets/std-metadata.json` (including `std::list::remove_take` and `std::map::{insert_take,remove_take}`) with one canonical source of truth.
    - [x] 21.0.3.2 Add CI drift gate that fails when locked std surface, emitted std metadata, typer std call-check surface (builtins + specialized std call check modules), and codegen std binding map (intrinsic vs package-import routing) diverge.
    - [x] 21.0.3.3 Emit deterministic std binding-map artifact in CI so drift checks do not depend on internal codegen implementation details.
  - [x] 21.0.4 Lock and align `std::host` capability surface for production profile readiness.
    - [x] 21.0.4.1 Define canonical v1 ownership for host-facing capabilities (`time`, `random`, `chain_id`, storage, events) across `std::host` vs chain wrappers, with explicit non-goals.
    - [x] 21.0.4.2 Lock strict-mode deterministic policy for `std::env::{time,random}` (allow/deny profile matrix and diagnostics) and align with runtime host-import docs.
    - [x] 21.0.4.3 Align strict host-profile capability allowlist and strict-gate checks with the canonical v1 `std::host` surface.
    - [x] 21.0.4.4 Publish machine-readable host capability policy artifact per profile (`contract_static`, `shared_app`) and assert deterministic serialization/hash in CI.
    - [x] 21.0.4.5 Add CI profile-conformance fixtures for `std::env::{time,random,chain_id}` and expected strict diagnostics on disallowed capabilities.
  - [x] 21.0.5 Add reproducible std-core artifact CI evidence gates (validation of 21.0.1 behavior).
    - [x] 21.0.5.1 Emit versioned `std::core` artifact + metadata + digest as CI artifacts with deterministic naming.
    - [x] 21.0.5.2 Add replay determinism assertion: identical inputs produce byte-identical std-core artifact and identical digest.
  - [x] 21.0.6 Harden import-pruning as an explicit CI acceptance gate (validation of 21.0.2 behavior).
    - [x] 21.0.6.1 Add Wasm import-section assertions proving only used external/package symbols are emitted.
    - [x] 21.0.6.2 Add negative coverage proving unused symbols declared in metadata/ABI never appear in emitted app Wasm imports.
  - [x] 21.0.7 Add non-vacuous precompiled-link proof gate for std-core.
    - [x] 21.0.7.0 Define and lock precompiled std-core activation contract (explicit build/profile switch and fallback policy) before enforcing non-vacuous link behavior.
    - [x] 21.0.7.1 Add CI fixture proving at least one locked `std::core` symbol is linked via package ABI/import (not satisfied by local intrinsic lowering only).
    - [x] 21.0.7.2 Add failure coverage that rejects fallback-to-intrinsic behavior when precompiled std-core mode is enabled for that symbol set and emits deterministic diagnostics.
  - [x] 21.0.8 Replace transitional synthetic strict fixtures with locked std-core symbols.
    - [x] 21.0.8.1 Migrate strict acceptance fixtures from synthetic `std::core::math::*` symbols to canonical locked std-core symbols.
    - [x] 21.0.8.2 Keep deterministic diagnostics ordering/hash assertions green after fixture migration, with no semantic regression in the existing `C101`-`C108` strict acceptance suite.

### 22 Package Trust + Dependency Resolution
- [x] 22.0 Package metadata trust hardening.
  - [x] 22.0.0 Publish and lock Phase 22 design documents (metadata migration, metadata v1, lockfile v1, resolver/solver determinism, advisory policy, diagnostics). (`docs/design/phase-22.0.0-package-trust-resolution-design-lock.md`, `docs/design/phase-22.0.1-canonical-package-metadata-migration.md`, `docs/design/phase-22.0.2-package-metadata-v1.md`, `docs/design/phase-22.0.3-lockfile-v1.md`, `docs/design/phase-22.1.0-resolver-semver-determinism.md`, `docs/design/phase-22.1.3-vulnerability-response-policy.md`, `docs/design/phase-22.0.6-phase22-diagnostics-reservation.md`)
  - [x] 22.0.1 Unify package metadata to one canonical production model and define migration/deprecation from legacy `clg-packages.json`. (`docs/design/phase-22.0.1-canonical-package-metadata-migration.md`, deterministic coexistence conflict gate in `crates/cli/src/commands/modules/package_metadata.rs`, IT coverage `legacy_and_canonical_package_metadata_conflict_reports_c027`)
  - [x] 22.0.2 Extend package metadata with artifact digest/signature/trust-anchor fields and strict validation. (strict preflight metadata schema v1 support + validation in `crates/cli/src/commands/build/strict_package_contract.rs`, trust-gate linkage checks in `crates/cli/src/commands/build/strict_package_signatures.rs`, regression coverage in strict unit/CLI IT suites)
  - [x] 22.0.3 Expand lockfile flow from 20.1 v0 bootstrap to full deterministic workflow.
    - [x] 22.0.3.1 Add CLI lockfile generation/update commands for exact package versions + digests.
    - [x] 22.0.3.2 Define canonical lockfile serialization/hash rules (stable ordering + deterministic writes).
    - [x] 22.0.3.3 Add replay tests proving byte-identical lockfiles from identical inputs.
  - [x] 22.0.4 Define package metadata/ABI compatibility policy (schema evolution, deprecation windows, migration guarantees) with regression tests. (`docs/design/phase-22.0.4-package-metadata-abi-compatibility-policy.md`, `crates/cli/tests/schema_compatibility_policy.rs`)
  - [x] 22.0.5 Expand signer lifecycle policy from 20.1 trust-policy v0 bootstrap to full production policy (rotation, revocation, expiry, emergency compromise handling). (`docs/design/phase-22.0.5-signer-lifecycle-policy.md`, `crates/cli/src/commands/build/strict_trust_policy.rs` tests, `crates/cli/src/commands/build/strict_package_signatures.rs` tests)
  - [x] 22.0.6 Reserve and register Phase 22 diagnostics for resolver/solver/advisory flows before implementation (stable JSON code contracts). (`docs/diagnostics.md` `C109`-`C119`, `crates/cli/tests/phase22_diagnostics_reservation.rs`)
  - [x] 22.0.7 Refactor shared strict validators (semver/digest/schema/id parsing) into a single module to prevent rule drift. (`docs/design/phase-22.0.7-strict-validator-consolidation.md`, `crates/cli/src/commands/build/strict_validation.rs`)
- [x] 22.1 Dependency resolution completion gates for milestone_2.
  - [x] 22.1.0 Lock deterministic resolver + semver solver policy (tie-break rules, conflict precedence, diagnostics ordering). (`docs/design/phase-22.1.0-resolver-policy.lock.json`, `crates/cli/tests/resolver_policy_lock.rs`)
  - [x] 22.1.1 Add transitive dependency resolution for compiled packages (deterministic graph + cycle diagnostics). (`crates/cli/src/commands/pkg.rs`, `crates/cli/tests/cli_it/pkg_lock.rs`)
  - [x] 22.1.2 Add deterministic semver solver with lockfile generation/update flow. (`crates/cli/src/commands/pkg.rs`, `crates/cli/tests/cli_it/pkg_lock.rs`)
  - [x] 22.1.3 Add package vulnerability response flow (advisory ingestion, denylist/yank policy, forced-upgrade semantics, deterministic diagnostics). (`crates/cli/src/commands/pkg.rs`, `crates/cli/tests/cli_it/pkg_lock.rs`)
    - [x] 22.1.3.a Strict advisory trust gate: require signed advisory envelope + trust-policy signer verification (trusted, non-revoked, valid signer window).
    - [x] 22.1.3.b Deterministic advisory-time evaluation: support `--advisory-as-of` and lock strict-mode requirement for replay-stable advisory applicability windows.
  - [x] 22.1.4 Add CI determinism replay gates for resolver/solver outputs (resolved graph artifact, lockfile bytes/hash, diagnostics ordering). (`.github/workflows/ci.yml`, `crates/cli/tests/ci_workflow.rs`, `crates/cli/tests/cli_it/pkg_lock.rs`, `crates/cli/src/commands/pkg.rs`)
  - [x] 22.1.5 Add build/run resolution parity policy so package trust/resolution behavior is explicit for both `clg build` and `clg run`.
    - [x] 22.1.5.1 Migrate standard/permissive compiled-package import indexing from legacy `clg-packages.json` to canonical metadata/ABI inputs and retire legacy loader behavior. (`crates/cli/src/commands/modules/package_metadata.rs`, `crates/cli/tests/cli_it/imports.rs`, `crates/cli/tests/cli_it/vc_outputs.rs`, `docs/design/phase-22.0.1-canonical-package-metadata-migration.md`, `docs/diagnostics.md`)
    - [x] 22.1.5.2 Lock and document strict vs non-strict trust semantics for metadata v1 and proof-claim boundaries (strict-only release/audit trust claims). (`docs/design/phase-22.0.2-package-metadata-v1.md`)
    - [x] 22.1.5.3 Emit an explicit non-strict assurance-claim marker in CLI/artifact outputs to prevent interpreting non-strict proofs as release-grade trust evidence.

### 23 Runtime Package Loader + Linker
- [x] 23.0 Runtime linker for compiled packages.
  - [x] 23.0.0 Publish and lock Phase 23 runtime loader/linker design before implementation. (`docs/design/phase-23.0-runtime-loader-linker-design-lock.md`)
    - [x] 23.0.0.1 Lock runtime source-of-truth inputs (lockfile/import-map/trust-policy/host-profile), canonical artifact locator model, deterministic resolution ordering, and canonical runtime link artifact contract (`clg.runtime-link.json` + hash).
    - [x] 23.0.0.2 Reserve and register Phase 23 runtime-loader diagnostics (`R012`-`R017`) before wiring implementation, and pin with reservation tests. (`docs/diagnostics.md`, `crates/cli/tests/phase23_diagnostics_reservation.rs`)
    - [x] 23.0.0.3 Lock deterministic replay contract and runtime signer-time semantics (signed_at-anchored, no ambient wall-clock dependence) for runtime loader outputs/diagnostics.
  - [x] 23.0.1 Implement host-side package loader core from trusted local store/index with explicit no-implicit-network default. (`crates/cli/src/commands/run/package_loader.rs`, `crates/cli/src/commands/run/mod.rs`, `crates/cli/tests/run_smoke.rs`)
  - [x] 23.0.2 Enforce fail-closed runtime trust gates (digest/signature/policy + lock/import-map consistency) before linking any package artifact. (`crates/cli/src/commands/run/package_loader.rs`, `crates/cli/tests/run_smoke.rs`)
  - [x] 23.0.3 Implement deterministic runtime linker/import binding path and deterministic runtime diagnostics for missing/mismatched/untrusted package artifacts. (`crates/cli/src/commands/run/mod.rs`, `crates/cli/src/commands/run/package_loader.rs`, `crates/cli/tests/run_smoke.rs`)
  - [x] 23.0.4 Add artifact availability/resilience policy (mirrors/cache/offline mode/retry/failure behavior) and operational runbook coverage. (`crates/cli/src/commands/run/package_loader.rs`, `crates/cli/tests/run_smoke.rs`, `docs/runtime/runtime-loader-resilience-runbook.md`)
- [x] 23.1 Runtime loading completion gate for milestone_2.
  - [x] 23.1.1 Enable automatic runtime package loader/linker path in runtime hosts (`clg run` and production host integrations) without manual import wiring. (`crates/cli/src/lib.rs`, `crates/cli/src/main.rs`, `crates/cli/src/commands/run/mod.rs`, `docs/runtime/host-integration-api.md`)
  - [x] 23.1.2 Add CI tamper + determinism replay matrix for runtime loading/linking (missing/mismatch/untrusted artifacts, diagnostics ordering, replay stability). (`crates/cli/tests/run_smoke.rs`, `.github/workflows/ci.yml`, `crates/cli/tests/ci_workflow.rs`)
  - [x] 23.1.3 Keep fail-closed runtime trust checks mandatory in all production profiles (no permissive fallback for runtime package loading). (`crates/cli/src/commands/run/package_loader.rs`, `crates/cli/tests/run_smoke.rs`)
  - [x] 23.1.4 Add staged rollout/canary + rollback criteria and release gate evidence for runtime loader enablement in production hosts. (`docs/runtime/runtime-loader-rollout-gate.md`, `crates/cli/tests/runtime_loader_rollout_gate.rs`)

### 24 Host Profiles + Milestone_2 Exit
- [x] 24.0 Host capability profile alignment.
  - [x] 24.0.1 Keep `std::crypto`/`std::env`/`std::wasi` host-backed with explicit determinism policies. (`docs/runtime/host-backed-determinism-policy.md`, `docs/runtime/host-imports.md`, `crates/cli/tests/cli_it/imports.rs`, `crates/cli/tests/cli_it/runtime_env.rs`, `crates/cli/tests/cli_it/crypto.rs`)
  - [x] 24.0.2 Expand host-profile docs from 20.1 host-profile v0 bootstrap to full static/contract vs shared/app production policy. (`docs/runtime/host-profiles-production-policy.md`, `docs/runtime/host-imports.md`, `docs/design/phase-20.1.0-host-profile-v0.md`, `crates/cli/tests/host_profile_policy_docs.rs`)
  - [x] 24.0.3 Add host conformance certification suite for deterministic std-host capability behavior across supported runtimes. (`crates/cli/tests/host_conformance_certification.rs`, `docs/runtime/host-profiles-production-policy.md`)
- [x] 24.1 Milestone_2 release gate.
  - [x] 24.1.1 Verify Phases 20-24 completion without regressions to Phase 19 strict/profile guarantees. (`.github/workflows/ci.yml`, `crates/cli/tests/ci_workflow.rs`, `crates/cli/tests/host_conformance_certification.rs`)
  - [x] 24.1.2 Publish `release_notes/milestone_2.md` once gates are green. (`release_notes/milestone_2.md`, `crates/cli/tests/milestone2_release_notes.rs`)
- [x] 24.2 Go-live checklist (must be green before milestone_2 tag).
  - [x] 24.2.1 Runnable baseline: `clg run clearlang-tests/16_namespaced_call.clear` passes in CI and docs clearly mark runnable vs parse-only fixtures. (`.github/workflows/ci.yml`, `crates/cli/tests/ci_workflow.rs`, `clearlang-tests/README.md`)
  - [x] 24.2.2 Precompiled std-core: CI emits versioned artifact + metadata with reproducible hash and import-pruning coverage. (`.github/workflows/ci.yml`, `crates/cli/tests/ci_workflow.rs`, `crates/cli/tests/cli_it/imports.rs`)
  - [x] 24.2.3 Package trust: digest/signature/trust-anchor metadata validation is enforced; malformed/untrusted metadata fails deterministically. (`.github/workflows/ci.yml`, `crates/cli/tests/cli_it/diagnostics.rs`, `crates/cli/tests/phase22_diagnostics_reservation.rs`)
  - [x] 24.2.4 Dependency resolution: transitive resolver + deterministic semver solver + lockfile enforcement are active in CI. (`.github/workflows/ci.yml`, `crates/cli/tests/cli_it/pkg_lock.rs`, `crates/cli/tests/resolver_policy_lock.rs`)
  - [x] 24.2.5 Runtime loader/linker: automatic package loading works and fails closed on missing/mismatch/untrusted artifacts with stable diagnostics. (`.github/workflows/ci.yml`, `crates/cli/tests/run_smoke.rs`, `docs/runtime/runtime-loader-rollout-gate.md`)
  - [x] 24.2.6 Host profiles: static/contract and shared/app profile behavior is documented and covered by integration tests. (`docs/runtime/host-profiles-production-policy.md`, `crates/cli/tests/host_profile_policy_docs.rs`, `crates/cli/tests/host_conformance_certification.rs`)
  - [x] 24.2.7 Assurance/regression gates: Phase 19 strict/profile tests remain green with no assurance-tier regression on protected fixtures. (`.github/workflows/ci.yml`, `crates/cli/tests/profile_regression_gate.rs`, `crates/cli/tests/phase19_design_principles.rs`)
  - [x] 24.2.8 Release readiness: security review, runbooks, signed artifacts, and `release_notes/milestone_2.md` are complete. (`docs/evidence/milestone_2-readiness.md`, `crates/cli/tests/milestone2_readiness_evidence.rs`, `release_notes/milestone_2.md`)
  - [x] 24.2.9 Production SLO/performance gates: package resolution/link latency, startup overhead, and memory/CPU budgets are measured and within defined thresholds. (`xtask/src/main/milestone2_gates.rs`, `.github/workflows/ci.yml`, `docs/evidence/milestone_2-performance.md`, `crates/cli/tests/milestone2_performance_evidence.rs`)
  - [x] 24.2.10 Supply-chain compliance gates: SBOM/license checks for shipped package artifacts and runtime dependencies are green. (`xtask/src/main/milestone2_gates.rs`, `.github/workflows/ci.yml`, `docs/evidence/milestone_2-supply-chain.md`, `crates/cli/tests/milestone2_supply_chain_evidence.rs`)
- [x] 24.3 Milestone_2 delivery governance (execution risk controls).
  - [x] 24.3.1 Assign an explicit owner/DRI for each Phase 20-24 parent task and record it in TODO/DEVPLAN. (`docs/rollout/milestone_2-governance.md`, `docs/rollout/DEVPLAN.md`)
  - [x] 24.3.2 Add target dates (planned start/end) for each Phase 20-24 parent task and mark critical-path dependencies. (`docs/rollout/milestone_2-governance.md`)
  - [x] 24.3.3 Maintain a milestone_2 risk register (top risks, mitigations, rollback owners) and review weekly. (`docs/rollout/milestone_2-governance.md`)
  - [x] 24.3.4 Add a release-train gate: do not tag milestone_2 unless 24.2.x is fully green and evidence links are attached. (`.github/workflows/ci.yml`, `crates/cli/tests/milestone2_release_train_gate.rs`, `crates/cli/tests/ci_workflow.rs`)

### 25 Milestone_3 Roadmap

Execution order for Milestone 3: **policy lock -> proof closure -> release UX -> quality gates -> distribution**.

- [x] 25.0 Release assurance policy (`release == proved`) [Gate A]
  - [x] 25.0.1 Publish milestone_3 design lock with explicit proof completion thresholds and fail-closed policy. (`docs/design/phase-25.0.1-milestone-3-design-lock.md`)
  - [x] 25.0.2 Define theorem-grade proof certification policy (`proved_all`) requiring pure functions, all VCs `proved`, zero assumptions, and strict-mode verification. (`docs/design/phase-25.0.2-theorem-grade-certification-policy.md`)
  - [x] 25.0.3 Lock assurance policy decision: `release == proved`; allow non-proved compile only in non-release/dev workflows. (`docs/design/phase-25.0.3-release-equals-proved-policy.md`)
  - [x] 25.0.4 Enforce release fail-closed policy: block release on any `failed|unknown|timeout|assumed` proof outcome. (`docs/design/phase-25.0.4-fail-closed-release-enforcement.md`)
  - [x] 25.0.5 Emit theorem-grade status in assurance artifacts/signature payload (`proof_status: proved_all|not_proved_all`) with deterministic serialization. (`docs/design/phase-25.0.5-proof-status-emission.md`)
  - [x] 25.0.6 Add verifier policy gate (`clg verify --require-assurance proved_all`) and wire it into release workflows. (`docs/design/phase-25.0.6-verify-require-assurance-gate.md`)
  - [x] 25.0.7 Add release compile profile behavior so production compile/publish fails unless theorem-grade policy passes. (`docs/design/phase-25.0.7-production-release-profile-gate.md`)
  - [x] 25.0.8 Add CI/release gate that blocks publish unless required proof matrix and evidence artifacts are complete. (`docs/design/phase-25.0.8-ci-release-proof-matrix-gate.md`)
  - [x] 25.0.9 Add CI gate asserting release bundles contain zero assumption boundaries (`unsigned.int_model`, `bitwise.uninterpreted`, `crypto.uninterpreted`). (`docs/design/phase-25.0.9-zero-assumption-boundary-release-gate.md`)
  - [x] 25.0.10 Add release-target proof parity gate: current release target (Windows) must produce deterministic proof outcomes and assurance status across identical runs; expand platform set after self-contained solver matrix lands. (`docs/design/phase-25.0.10-cross-platform-proof-parity-gate.md`)
  - [x] 25.0.11 Wire release gate to std proof matrix allowlist: release bundles may include only APIs/symbols marked `proved` in the canonical matrix. (`docs/design/phase-25.0.11-release-symbol-allowlist-gate.md`)
  - [x] 25.0.12 Add strict release-surface policy that only permits features/intrinsics with fully proved semantics (unproved surfaces remain non-release/dev-only). (`docs/design/phase-25.0.12-strict-release-surface-policy.md`)
  - [x] 25.0.13 Enforce crypto proof boundary policy in strict release gates: any remaining `crypto.uninterpreted` boundary blocks theorem-grade/release-grade claims. (`docs/design/phase-25.0.13-crypto-proof-boundary-release-gate.md`)
  - [x] 25.0.14 Align README/docs assurance claims with profile reality (avoid implying `standard` compile is theorem-grade proved). (`docs/design/phase-25.0.14-docs-assurance-profile-alignment.md`)
  - [x] 25.0.15 Keep language surface minimal: defer `theorem` keyword and treat theorem-grade as certification status (not syntax) for milestone_3. (`docs/design/phase-25.0.15-defer-theorem-keyword.md`)

- [x] 25.1 Proof engine and solver closure [Gate B]
  - [x] 25.1.1 Publish Gate B design lock (policy, non-goals, deterministic inputs, and exit criteria) before solver implementation lands. (`docs/design/phase-25.1.1-gate-b-design-lock.md`)
  - [x] 25.1.2 Reserve and document solver-era diagnostics for proof execution/artifact failures (`solver unavailable`, `timeout`, `artifact mismatch`, deterministic replay mismatch). (`docs/design/phase-25.1.2-solver-diagnostics-reservation.md`)
  - [x] 25.1.3 Define `--emit-proof` artifact schema/version and canonical serialization rules (including backward/forward compatibility policy). (`docs/design/phase-25.1.3-emit-proof-schema-v1.md`)
  - [x] 25.1.4 Lock deterministic solver profile as explicit strict input (version/options/timeouts) and include it in release evidence/signature claims. (`docs/design/phase-25.1.4-deterministic-solver-profile-lock.md`)
  - [x] 25.1.5 Version VC/proof schemas for solver-era statuses and add backward/forward compatibility gates for `status` + counterexample fields. (`docs/design/phase-25.1.5-vc-proof-schema-compatibility-gates.md`)
  - [x] 25.1.6 Add solver runtime safety/isolation policy (resource limits, timeout/kill semantics, crash handling) with fail-closed diagnostics mapping. (`docs/design/phase-25.1.6-solver-runtime-safety-isolation.md`)
  - [x] 25.1.7 Close VC soundness gaps so emitted obligations are solver-ready (no placeholder/unconstrained local symbols). (`docs/design/phase-25.1.7-vc-solver-ready-symbol-closure.md`)
  - [x] 25.1.8 Integrate theorem-prover execution path (Z3 baseline) and record deterministic per-VC outcomes (`proved|failed|unknown|timeout`). (`docs/design/phase-25.1.8-z3-vc-outcome-integration.md`)
  - [x] 25.1.9 Add proof artifact emission (`--emit-proof`) and verification wiring in `clg verify`. (`docs/design/phase-25.1.9-proof-artifact-emit-verify-wiring.md`)
  - [x] 25.1.10 Bind proof artifact hash and solver-profile hash into signed payload/manifest and enforce verification consistency in `clg verify`. (`docs/design/phase-25.1.10-proof-solver-hash-binding.md`)
  - [x] 25.1.11 Add deterministic `--emit-proof` artifact reproducibility gate (byte/hash stability across identical inputs and runs). (`docs/design/phase-25.1.11-emit-proof-reproducibility-gate.md`)
  - [x] 25.1.12 Harden solver determinism contract: pin solver version/options/timeouts and add replay-stability CI checks. (`docs/design/phase-25.1.12-solver-determinism-contract-and-replay.md`)
  - [x] 25.1.13 Add CI replay gates for deterministic solver outcomes (`proved|failed|unknown|timeout`) across identical runs and release target platforms (Windows-only for current milestone scope). (`docs/design/phase-25.1.13-ci-replay-gates-release-target-platforms.md`)
  - [x] 25.1.14 Make solver integration self-contained by default (no external system install required) by shipping a platform bundle or Rust-managed vendor path. (`docs/design/phase-25.1.14-self-contained-solver-vendor-path.md`)
  - [x] 25.1.15 Define required self-contained solver support matrix (OS/arch coverage, unsupported-target policy, and CI validation strategy). (`docs/design/phase-25.1.15-solver-support-matrix.md`)
  - [x] 25.1.16 Add solver supply-chain/security gates: pinned version, checksum/signature verification, license/notice inclusion, CVE update policy, and rollback procedure. (`docs/design/phase-25.1.16-solver-supply-chain-security-gates.md`)
  - [x] 25.1.17 Add bitvector proof encoding for covered unsigned paths (starting with `U64`) to retire `unsigned.int_model` assumptions on those paths. (`docs/design/phase-25.1.17-u64-bitvector-bridge-and-unsigned-boundary-retirement.md`)
  - [x] 25.1.18 Add bitwise SMT encoding for covered operators/intrinsics to retire `bitwise.uninterpreted` assumptions on those paths. (`docs/design/phase-25.1.18-u64-bitwise-bv64-encoding.md`)
  - [x] 25.1.19 Close crypto proof-model gaps required for `release == proved` by replacing `crypto.uninterpreted` for release-enabled intrinsic surfaces. (`docs/design/phase-25.1.19-crypto-release-closure.md`)
  - [x] 25.1.20 Define strict-production cutover policy from `generated`/`solver_unavailable` placeholders to solver-required fail-closed behavior. (`docs/design/phase-25.1.20-production-cutover-fail-closed-policy.md`)
  - [x] 25.1.21 Define Gate B completion metric as a machine-checkable closure rule for release-enabled surfaces (explicit zero-boundary + deterministic proof-evidence thresholds). (`docs/design/phase-25.1.21-gate-b-closure-metric.md`)

- [ ] 25.2 Release workflow and proved-only UX simplification [Gate C]
  - [x] 25.2.1 Record Gate C discussion conclusions as policy lock: (a) simplified command options with proved-only release assumptions, (b) one-command release path, (c) built-in Z3 migration path, and (d) IDE/VSCode-first CLI contracts. (`docs/design/phase-25.2.1-gate-c-policy-lock.md`, `docs/design/phase-25.2.2-release-ux-design-lock.md`, `docs/design/phase-25.2.3-release-command-orchestration.md`, `docs/design/phase-25.2.4-release-proved-only-default.md`)
  - [x] 25.2.2 Publish Gate C UX design lock: default policy is `release == proved` (`proved_all` required), with minimal primary command surface (`clg check`, `clg test`, `clg release`) and advanced commands retained for expert/debug workflows only. (`docs/design/phase-25.2.2-release-ux-design-lock.md`, `crates/cli/src/main.rs`, `crates/cli/src/commands/release.rs`, `crates/cli/tests/cli_it/basic.rs`)
  - [x] 25.2.3 Add one-command `clg release` orchestration (lock -> build/prove -> sign -> verify -> release bundle) with fail-closed behavior and minimal required flags. (`docs/design/phase-25.2.3-release-command-orchestration.md`, `crates/cli/src/commands/release.rs`, `crates/cli/tests/cli_it/diagnostics/release_command.rs`)
  - [x] 25.2.4 Make `clg release` enforce theorem-grade proof by default (no optional downgrade path for release artifacts). (`docs/design/phase-25.2.4-release-proved-only-default.md`, `crates/cli/src/commands/release.rs`, `crates/cli/tests/cli_it/basic.rs`, `crates/cli/tests/cli_it/diagnostics/release_command.rs`)
  - [x] 25.2.5 Add `clg strict init <root>` to generate/validate strict preflight inputs (`clg.project.json`/`clg.lock.json` + trust/profile files) and prefill release defaults to reduce CLI parameters. (`docs/design/phase-25.2.5-strict-init-bootstrap.md`, `crates/cli/src/main.rs`, `crates/cli/src/commands/strict.rs`, `crates/cli/src/commands/release.rs`, `crates/cli/tests/cli_it/basic.rs`, `crates/cli/tests/cli_it/diagnostics/release_command.rs`)
  - [x] 25.2.6 Add CLI simplification/deprecation pass: hide and remove flag-heavy legacy release path from primary docs/help and emit migration guidance to `clg release` (no compatibility aliases required pre-production). (`docs/design/phase-25.2.6-release-migration-guidance.md`, `crates/cli/src/main.rs`, `docs/release-process.md`, `README.md`, `crates/cli/tests/cli_it/basic.rs`)
  - [ ] 25.2.7 Add `clg check` as a fast deterministic preflight command for local iteration (non-release), aligned with release policy inputs.
  - [ ] 25.2.8 Add IDE integration contract for VSCode plugin support: stable machine-readable outputs (`--json-errors`, structured stage/progress events, deterministic exit-code mapping) for `check|test|release`.
  - [ ] 25.2.9 Add non-interactive mode guarantees for all primary commands (no prompts, deterministic stdout/stderr separation, plugin-safe logs).
  - [ ] 25.2.10 Add VSCode plugin-facing command profile docs (recommended invocations, expected JSON schema/versioning, cancellation/timeout behavior).
  - [ ] 25.2.11 Add CI contract tests for IDE-facing CLI behavior to prevent breaking plugin integrations across releases.
  - [ ] 25.2.12 Add release-precheck gating so `fmt` + `lint` + tests must pass before strict signed publish flow (local and CI).
  - [ ] 25.2.13 Add `clg fmt` for `.clear` sources with deterministic formatting output.
  - [ ] 25.2.14 Add `clg lint` for `.clear` sources (quality/safety checks) with stable diagnostics and `--deny-warnings` support.
  - [ ] 25.2.15 Upgrade solver bundle `.sig` verification from integrity-metadata mode to cryptographic publisher-authenticity verification (pinned vendor key/cert + rotation policy).
  - [ ] 25.2.16 Add solver backend abstraction (`external-z3-cli` and `rust-z3-lib`) with deterministic backend selection policy.
  - [ ] 25.2.17 Implement `rust-z3-lib` backend behind a feature/cutover flag while keeping `external-z3-cli` as temporary fallback.
  - [ ] 25.2.18 Add determinism parity gate between backends (same VC status outcomes and proof artifact hash for identical strict inputs).
  - [ ] 25.2.19 Add release packaging gate to remove runtime dependency on `tools/proof/z3` for supported release targets once `rust-z3-lib` is cut over.

- [ ] 25.3 Unit testing and test runner [Gate D]
  - [ ] 25.3.1 Define canonical unit-test layout under `tests/` and function naming convention `test_*` (no annotation syntax).
  - [ ] 25.3.2 Add `clg test` command that discovers/runs ClearLang unit tests and returns non-zero on failures.
  - [ ] 25.3.3 Add deterministic test reports (`human|json|junit`) with stable failure diagnostics.
  - [ ] 25.3.4 Add CI coverage and release-gate integration for `clg test` in strict production workflows.
  - [ ] 25.3.5 Lock `clg test` proof-mode policy for CI/release (`standard` vs `strict`) and require deterministic, documented mode selection.
  - [ ] 25.3.6 Add deterministic migration/cutover from `clearlang-tests/` fixtures to canonical `tests/` layout (or document one source-of-truth alias model) and enforce it in CI.

- [ ] 25.4 Clear project dependency manifests (`json`) [Gate E]
  - [ ] 25.4.1 Define `clg.project.json` as the user-authored dependency manifest (declared packages/version ranges/source policy).
  - [ ] 25.4.2 Keep `clg.lock.json` as the tool-generated deterministic lockfile (exact versions, digests, and resolved graph identity).
  - [ ] 25.4.3 Add resolver flow: `clg pkg lock --generate|--update` reads `clg.project.json` and writes canonical lock outputs.
  - [ ] 25.4.4 Ensure imports in `.clear` remain version-free (logical module/package paths only); versions live only in project/lock JSON.
  - [ ] 25.4.5 Add schema docs, migration notes, and CI drift gates that fail on manifest/lock inconsistency.
  - [ ] 25.4.6 Define migration/coexistence policy from canonical package metadata/ABI inputs to `clg.project.json` + `clg.lock.json`, with deterministic conflict diagnostics.
  - [ ] 25.4.7 Add backward-compatibility and deprecation timeline for legacy inputs with explicit fail-closed cutover milestone.
  - [ ] 25.4.8 Add migration tooling command/docs (`clg pkg migrate-manifest`) to generate `clg.project.json` from existing canonical metadata inputs.

- [ ] 25.5 Literal ergonomics for low-level/crypto code
  - [ ] 25.5.1 Add integer literal support for `0x...` (hex) and `0b...` (binary) with deterministic parsing, underscore rules, and diagnostics.
  - [ ] 25.5.2 Define typing/inference rules for new literals (`Int` default, unsigned expected-type coercion/casts) with deterministic diagnostics.
  - [ ] 25.5.3 Add VC/proof regression coverage for hex/binary literals in bitwise/unsigned paths to ensure no proof determinism regressions.

- [ ] 25.6 First usable binary releases
  - [ ] 25.6.1 Lock GA target matrix and support policy (Windows/Linux/macOS baseline targets + preview targets).
  - [ ] 25.6.2 Produce signed release binaries with reproducible metadata, checksums, and SBOM/license bundles.
  - [ ] 25.6.3 Publish install/upgrade/uninstall/verify docs and add smoke coverage per GA target.
  - [ ] 25.6.4 Add release-train checklist plus rollback/incident runbook for binary distribution failures.
  - [ ] 25.6.5 Publish `release_notes/milestone_3.md` with compatibility matrix, known limitations, and upgrade notes.

- [ ] 25.7 Online testbed (if feasible)
  - [ ] 25.7.1 Add a go/no-go gate (threat model, abuse controls, ops budget, and owner assignment) before implementation.
  - [ ] 25.7.2 If approved, implement a deterministic sandboxed compile/run path with resource/time limits and clear diagnostics.
  - [ ] 25.7.3 Add SLO/runbook/incident playbook and verify readiness before public rollout.
  - [ ] 25.7.4 If gates fail, publish a defer decision and keep local/CLI-first workflow as canonical path.

- [ ] 25.8 Standard library work is tracked in standalone Phase 26 (critical path).

### 26 Standalone Standard Library Phase (Critical)

Execution order for std proof coverage: **scope lock -> core coverage -> set subset -> set ops -> cardinality -> list/map completion**.

- [ ] 26.0 Std scope and governance [Std Gate A]
  - [ ] 26.0.1 Define and lock v1 std scope as `must-have` vs `stretch` symbols (`std::core`, `std::host`, package contracts).
  - [ ] 26.0.2 Publish explicit defer list for unresolved `stretch` symbols with follow-up phase assignment.
  - [ ] 26.0.3 Add std stability policy (compatibility guarantees, deprecation windows, and versioning policy) before public GA.

- [ ] 26.1 Std implementation and release gates [Std Gate B]
  - [ ] 26.1.1 Implement all `must-have` std functions/types with deterministic typing/lowering/runtime behavior.
  - [ ] 26.1.2 Add full std coverage matrix (`typed|runtime|proved` per symbol) with CI drift gates.
  - [ ] 26.1.3 Keep strict package metadata/ABI/import-pruning/trust gates green for expanded std surface.

- [ ] 26.2 Finite-set proof roadmap (ordered execution) [Std Gate C]
  - [ ] 26.2.1 Add `std::set::subset(a, b) -> Bool` API with typing/lowering/runtime coverage and regression tests.
  - [ ] 26.2.2 Add finite-set VC/SMT reasoning for membership + subset and enforce strict no-assumption gate for theorem-grade claims over set properties.
  - [ ] 26.2.3 Sequence rule: complete subset proof support first (API + VC/SMT + tests) before expanding other set operators.
  - [ ] 26.2.4 Sequence rule: add proof support for `union`/`intersect`/`diff` after subset is complete and stable.
  - [ ] 26.2.5 Sequence rule: add cardinality-heavy proofs last (`len`, bounds, set-size relations) with solver performance guardrails.

- [ ] 26.3 List proof roadmap [Std Gate D]
  - [ ] 26.3.1 Define/lock list proof contracts for core APIs (`len`, `get`, `push`, `insert`, `remove`, `remove_take`, `pop`).
  - [ ] 26.3.2 Add VC/SMT reasoning for list index bounds and shape-preservation invariants.
  - [ ] 26.3.3 Add theorem-grade gates for list proofs (no assumption boundaries on release-enabled list surfaces).

- [ ] 26.4 Map proof roadmap [Std Gate E]
  - [ ] 26.4.1 Define/lock map proof contracts for core APIs (`len`, `contains`, `get`, `insert`, `insert_take`, `remove`, `remove_take`).
  - [ ] 26.4.2 Add VC/SMT reasoning for key-membership/value-consistency invariants.
  - [ ] 26.4.3 Add theorem-grade gates for map proofs (no assumption boundaries on release-enabled map surfaces).
