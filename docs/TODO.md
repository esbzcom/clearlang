# ClearLang TODO

A focused, actionable checklist to move from Phase 11 and beyond.

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

Current focus: Phase 11 - proof-carrying Wasm verification.

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
    - [x] 16.7.1 Draft design note: registry schema, payload format, and verification flow.
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
- [x] 17.2 Add generics and trait/interface abstractions beyond built-in ADTs.
  - [x] 17.2.1 Design: generics syntax, trait bounds, coherence rules, monomorphization strategy, and canonical instantiation mangling for debug/proof metadata.
  - [x] 17.2.2 Parser/AST: type params, bounds, and impl blocks. (Scaffolding only; no typechecking/trait resolution yet.)
  - [x] 17.2.3 Typer: inference, trait resolution, and error diagnostics.
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
  - [x] 17.3.8 Implement structural equality for non-primitive `Map`/`Set` keys (Option/Result/structs/enums/tuples/arrays) and add runtime coverage.
  - [x] 17.3.9 Add runtime tests for `list::get`/`list::pop`, `set::contains`, `map::contains`/`map::get`, and no-op remove cases.
  - [x] 17.3.10 Docs refresh: update `docs/collections.md` + `docs/design/phase-17.3-collections-runtime.md` to reflect `new()` inference and current `can_mut` behavior.
  - [x] 17.3.11 Clarify invalid-handle behavior for collections and align trap code/docs (R002 vs new code) or add explicit pointer validation.
  - [x] 17.3.12 Use unsigned bounds checks in collection handle validation to avoid signed i32 overflow in pointer comparisons.
- [ ] 17.4 Add general array/slice types with indexing semantics and bounds checks.
  - [ ] 17.4.1 Spec: array/slice syntax, indexing semantics, and bounds behavior.
  - [ ] 17.4.2 Parser/AST + typer support for arrays/slices and indexing.
  - [ ] 17.4.3 Lowering/codegen for layout + bounds checks.
  - [ ] 17.4.4 Runtime helpers (if needed) + tests.
  - Tech debt: general arrays/slices with indexing semantics remain pending; fixed-size arrays/tuples use pointer layout for nested elements with runtime guards for dynamic indices.
- [ ] 17.5 Add module/import system with visibility controls for libraries.
  - [ ] 17.5.1 Design: file layout, module paths, visibility keywords, and re-exports.
  - [ ] 17.5.2 Parser/AST for `mod`/`use` and namespace nodes.
  - [ ] 17.5.3 Typer: module graph resolution + name shadowing diagnostics.
  - [ ] 17.5.4 Build system updates (module discovery, caching, error spans).
  - [ ] 17.5.5 Tests + docs walkthrough.
- [ ] 17.6 Linear-aware collections: design note, effects/VC plan, and phased prototype.
  - [ ] 17.6.1 Design: ownership rules for collections of resources.
  - [ ] 17.6.2 Typer rules + diagnostics for linear-aware collection ops.
  - [ ] 17.6.3 VC/effect integration plan + prototype tests.
- [ ] 17.7 First-class functions and closures (if ClearLang is to be general-purpose).
  - [ ] 17.7.1 Design: function types, capture semantics, and effect annotations.
  - [ ] 17.7.2 Parser/AST: lambdas, capture lists (if any), and type annotations.
  - [ ] 17.7.3 Typer: closure typing, lifetime/capture checks, and effect compatibility.
  - [ ] 17.7.4 Lowering/codegen: closure environment layout + call ABI.
  - [ ] 17.7.5 Tests + docs examples.
 - [ ] 17.8 Traits follow-ups (post-17.2).
   - [ ] 17.8.1 Default trait method bodies with explicit effect checking and override rules.
   - [ ] 17.8.2 Optional explicit impl selection syntax (only if coherence is relaxed).
   - [ ] 17.8.3 Proof/debug name shortening (optional hash suffix for long mangled names).

### 18 Production hardening + attestation
- [ ] 18.1 Attestation registry hardening (authz, key rotation, revocation, schema versioning).
- [ ] 18.2 Data availability policy (pinning/backup/retention) for attestation payloads.
- [ ] 18.3 Security review + fuzzing for attestation contract and payload validation.

