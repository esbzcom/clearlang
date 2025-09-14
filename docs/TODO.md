# Lumi TODO

A focused, actionable checklist to move from Phase 2 → Phase 3 and beyond.

## Phase 0 — Workspace & Toolchain (Done)

- [x] Set up Rust toolchain and Cargo workspace.
- [x] Add crates: `cli`, `parser`, `ast`, `typer`, `ir`, `codegen-wasm`.
- [x] Install and use Wasmtime and `wasm-tools` locally.

## Phase 1 — Hello WASM (Done)

- [x] Implement `emit_trivial_main` producing `main() -> i32` returning 42.
- [x] Add CLI `emit-hello` subcommand writing `hello.wasm` (creates parent dirs).
- [x] Validate with `wasm-tools validate` and run with Wasmtime.
- [x] Add tests to check Wasm header/export and `i32.const 42`.

## Phase 2 — Parser (Done)

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
- [x] lumi-tests samples for positive and negative cases.

2.5 Error Coverage (Optional)
- [x] Improve parse error messages/spans where useful.
- [x] More negative tests (unknown idents, reserved keywords).

## Phase 3 — Typer & IR (Done)

→ Current focus: Phase 4.3–4.7 — Std Collections (type-only) and 4.8 — DX; next: Phase 5 — Codegen/Strings runtime.

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
- [x] Lower AST → IR guided by typer; preserve minimal names/locals.

3.5 Integration
- [x] CLI `build`: parse → type-check → lower to IR → emit Wasm.
- [x] Codegen replaces const-eval; keep const-eval behind a feature flag (optional).
 - [x] Add `--validate` flag to run `wasm-tools validate` on outputs.
 

3.6 Tests
- [x] Typer errors: unknown function, arity mismatch, type mismatch.
- [x] IR/codegen e2e for `01_hello`, `02_arith`, `03_nested_calls`, `04_multiline_call`, `05_trailing_param_comma`.
- [x] Keep negative parse case `06_trailing_call_comma`.

3.7 Docs
- [x] Add `docs/typing.md` (rules/spec) and `docs/ir.md` (IR shape).
- [x] Update README “Current Status” to mark Phase 3 in progress.

3.8 Diagnostics & Spans
- [x] Attach source spans in AST via chumsky for identifiers and expressions.
- [x] Propagate spans into typer errors (unknown var/fn, arity, return/type mismatch).
- [x] Add tests that assert span presence/format in error messages.
- [x] Gate Phase 4 switch (IR→Wasm as default) on basic span coverage to avoid tech debt.

## Phase 4 - Namespacing + Strings (Parse/Type) + Std Collections stubs + Small Optimizations (Done)

- 4.1 Namespacing (no tech debt)
  - [x] Add namespaced call syntax (paths): `std::str::len(s)`, `std::list::push(l,x)`, `std::map::get(m,k)`, `std::set::contains(s,x)`.
  - [x] Typer: accept namespaced callees as exact names (parser done; built-ins via 4.3–4.7).

- 4.2 Strings (parse/type only)
  - [x] Add `Type::String` and `Expr::String` with escapes and multi-line string literal support.
  - [x] Typer: `std::str` built-ins (len/concat/eq) for strings.
  - [x] Typer: make `String` first-class in params/returns and literals.
  - [x] Tests: single-line, multi-line, escapes, span diagnostics.
  - [x] DX: Split parser into modules to unblock strings and path growth (`tokens.rs`, `types.rs`, `literals.rs`, `path.rs`, `expr.rs`, `func.rs`, `program.rs`; `lib.rs` wires them).
  - [x] Syntax: adopt `function` keyword exclusively (remove `fn`); update parser, tests, and docs for clarity and readability.
  - [x] CLI/Diagnostics: add `--json-errors` with short error codes (e.g., P001, T003) to support AI repair loops.
  - [x] Rename Str → String: one-shot rename across AST/parser/typer/tests/docs; keep semantics unchanged.

- 4.3 Collections Strict Errors (Option A)
  - [x] Typer: detect `std::list/*`, `std::set/*`, `std::map/*` and emit one clear, spanful error
        (code `T101` "collections require generics/ADTs; planned in later slices").
  - [x] Tests: calls parse; type errors return stable JSON (`--json-errors`) with code and spans.
- 4.4 Parametric Types + Minimal ADTs + `match` (built-ins only)
  - [x] Parser: accept `Option<T>` and `Result<T,E>` types; add minimal `match` for these two ADTs.
  - [x] Tests: constructors and simple matches parse.
- 4.5 Option/Result Typing Rules
  - [x] Typer: rules for `Option`/`Result` `match` (exhaustiveness, binders, arm type unification); partial constructors support (`Some(expr)`); no codegen/runtime yet.
  - [x] Tests: typer-only tests for `Option`/`Result` matches and errors (T201–T205); Option constructors (`Some`) and identity.
- 4.6 Collections Signatures (type-only)
  - [x] Provide namespaced APIs using Option/Result (type-check only):
        `std::list::{len,push,pop}` (pop -> Option<T>); `new` present but requires inference → emits T206.
        `std::set::{len,insert,remove,contains}`.
        `std::map::{len,insert,remove,get (-> Option<V>), contains}`.
  - [x] Parser: add `List<T>`, `Set<T>`, `Map<K,V>` types.
  - [x] Typer: enforce element/key/value types; dedicated errors T206–T208.
  - [x] Tests: typer-only tests for positive and negative cases.
- 4.7 Collections Docs/DX
  - [x] Document naming (modules lower-case: `std::list`; types PascalCase: `List<T>`),
        dual-API guidance (precondition vs Option/Result), and JSON error codes
        (include T206 “new requires inference” and T207 “expected collection kind”).
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
  - [x] Tests: add `lumi-tests/18_return_simple.lumi` and include in CLI IT.

- 4.10 Conditionals (expr-form if/else chain)
  - [x] Parser: `if cond { ... } (else if cond { ... })* else { ... }` (no `elif` alias).
  - [x] Typer: `cond: Bool` per arm; all branches unify to a single result type.
  - [x] Tests: multi-branch chains; require final `else` in expression form.
  - [x] Diagnostics: reserve codes P010 (MissingElseForExprIf) and T301 (BranchTypeMismatch);
        include spans in JSON and add tests.

 
## Phase 5 — Codegen IR → Wasm

5.0 Codegen Layout & DX
- [ ] Gate build stage logs behind `--verbose` (keep default quieter).
- [ ] Split `codegen-wasm` into `trivial.rs` (emit_trivial_main), `const_eval.rs` (emit_from_ast + eval), and `ir.rs` (IR→Wasm encoder); re-export from `lib.rs`.
 - [ ] Refactor CLI by splitting subcommands into `commands/{emit_hello,parse,build,run}.rs` and small helpers.

- [ ] Refactor CLI by splitting subcommands into `commands/{emit_hello,parse,build,run}.rs` and small helpers.
5.1 Module & Signatures
- [ ] Types, function indices, and exports.
- [ ] Deduplicate function signatures in the Wasm Type section (reuse type indices).

5.2 Locals & Stack
- [ ] Local allocation for temps; map IR values to stack ops.

5.3 Ops & Calls
- [ ] Encode `IConst`, `IBin`, and `Call` to Wasm; verify results in IT.
- [ ] Switch calls to use callee indices; update IR lowering accordingly (resolve names to indices).

5.4 Replace Const-Eval
- [ ] Switch CLI build to IR→Wasm path by default; keep const-eval for quick checks.
- [ ] Remove const-eval fallback once IR path is stable (optional cleanup).

5.5 Tests
- [ ] Extend e2e tests to verify outputs across samples via Wasmtime.
- [ ] Reuse a shared Wasmtime `Engine` in tests (e.g., `once_cell`) to speed up instantiation.

5.6 Strings Runtime
- [ ] Define String memory model (ptr + len; utf-8 bytes).
- [ ] Implement `std::str::{len, concat, eq}` via intrinsics or a small runtime.
- [ ] Provide a minimal bump allocator or reuse host env for concat.
- [ ] Add e2e tests for string ops; document runtime model.

5.7 ADT Ergonomics (sugar)
- [ ] `if let` for `Some/Ok` single-variant handling; parser + desugar to 2-arm match.
- [ ] `??` coalescing for Option (sugar for `unwrap_or`).
- [ ] `?` try operator for Option/Result (propagate early) - design behind a flag.

## Phase 6 — Contracts & Effects (Safety)

6.1 Syntax
- [ ] `require { expr }` / `ensure { expr }`, effects `pure|mut|io`.

6.2 Typer Rules
- [ ] Enforce effects and basic purity constraints.
- [ ] Allow multiple `require`/`ensure` (conjoined semantics).
- [ ] Generate Verification Conditions (VCs) for pure, expression-bodied functions.
- [ ] CLI: `--emit-vcs` outputs stable JSON (function, vc_id, pre, post, smt2, status).

6.3 Mutable Collections (effects)
- [ ] Add effect-gated mutable variants for collections (initial sketch, no runtime yet):
  - `std::list::{push_mut(l: List<T>, x: T) -> Unit, insert_mut(l: List<T>, x: T, i: Int) -> Unit, remove_mut(l: List<T>, i: Int) -> Option<T>, pop_mut(l: List<T>) -> Option<T>, clear_mut(l: List<T>) -> Unit}`.
  - `std::set::{insert_mut(s: Set<T>, x: T) -> Bool, remove_mut(s: Set<T>, x: T) -> Bool, clear_mut(s: Set<T>) -> Unit}`.
  - `std::map::{insert_mut(m: Map<K,V>, k: K, v: V) -> Option<V>, remove_mut(m: Map<K,V>, k: K) -> Option<V>, clear_mut(m: Map<K,V>) -> Unit}`.
- [ ] Type system: restrict usage of `*_mut` to functions with `mut` effect; pure code continues using immutable APIs.
- [ ] Tests: typer-only coverage that `*_mut` require `mut` effect and have correct signatures.

6.3 Runtime Guards
- [ ] Lower contracts to guards that trap on violation.

6.4 Metadata & Proofs
- [ ] Emit `lumi.proof` custom section v1 (contracts/effects/VCs; optional proofs).
- [ ] Add offline signatures (Ed25519):
      - CLI build flags: `--sign --key --key-id --sign-scope proofs|module|both --sig-out`.
      - CLI verify: `lumi verify --sig --pubkey [--vcs --proofs]`.
- [ ] Canonical payloads (JCS) with SHA-256 hashes: `module_hash`, `proofs_hash`.
- [ ] Plan `lumiverify` tool (re-check proofs + signatures) — keep in backlog until Phase 10.

## Phase 7 — Resource/Linear Types

7.1 Syntax & Semantics
- [ ] Add `resource` type declarations and usage guidelines.
- [ ] Define move-only semantics: no implicit copies; explicit `move`/`drop` as needed.

7.2 Typing Rules
- [ ] Linear usage checking: every resource is consumed exactly once; no double-use.
- [ ] Function signatures express resource flow (in/out/borrow) as needed.

7.3 Aliasing & In-Place Updates
- [ ] Ownership/aliasing rules to guarantee unique access for in-place updates.
- [ ] Freeze/thaw design sketch (optional): safe conversion between immutable and uniquely-owned mutable states.
- [ ] Update mutable collection ops to leverage unique ownership (no hidden aliasing).

7.3 Tests & Docs
- [ ] Unit tests for moves, drops, and invalid double-use.
- [ ] Docs explaining how resource types prevent double-spend patterns.

## Phase 8 — Totality & Loops with Invariants

8.1 Syntax
- [ ] Introduce `while` loops with required loop invariants and optional variants/measures for termination.

8.2 Totality
- [ ] Enforce totality for `pure` functions: require structural recursion or a decreasing measure.
- [ ] Provide diagnostics with spans for missing or non-decreasing measures.

8.3 Typing & Checks
- [ ] Type rules for loop invariants; ensure invariants are well-typed and refer to in-scope variables.
- [ ] Guardrail: allow opting out behind a flag initially to ease migration.

8.4 Tests & Docs
- [ ] Positive/negative tests for loops and recursion with measures.
- [ ] Document totality policy and examples in `docs/typing.md`.

## Phase 9 — Refinement Types

9.1 Syntax
- [ ] `type Nat = Int where n >= 0` and similar `where`-refined aliases.

9.2 Typing & Constraints
- [ ] Propagate refinement constraints through expressions and function boundaries.
- [ ] Interop with `require`/`ensure` from Phase 6; generate VCs for refinements.

9.3 Tooling & Tests
- [ ] `--emit-vcs` includes refinements in generated obligations; stable JSON.
- [ ] Tests for typical refinements (non-negative, bounded ranges, simple equalities).

## Safety & Tooling

- [ ] Always validate generated Wasm: `wasm-tools validate` in CI.
- [ ] Add CI to run `cargo test` across workspace; run pipeline tests.
- [ ] Document Wasmtime fuel/epoch/memory limits for runtime safety.
- [ ] Add `--contracts=runtime|hybrid|static` flag design (future).
- [ ] Enable Wasmtime fuel/epoch limits in IT tests for runtime bounding.
 - [x] Document pre-commit hook usage in README; provide skip toggles.

## Developer Experience

- [ ] Simple tracing/logging for pipeline stages in CLI.
- [ ] Add `cargo xtask` or Makefile for common flows (build/validate/run).
- [ ] Consistent error types and messages across crates.
- [ ] Preallocate HashMaps/Vecs in typer/parser where sizes are known (e.g., builtins+funcs, params per function).
- [ ] Release profile tuning in top-level Cargo.toml: `lto = "thin"`, `codegen-units = 1` (optionally `strip = "symbols"`).

## Nice-to-Have (Backlog)

- [ ] WASI `print` intrinsic for observable output.
- [ ] Proof-carrying Wasm prototype (`lumiverify`).
- [ ] On-chain attestation (anchoring) for signatures (EVM registry + IPFS URIs).
