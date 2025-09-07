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
- [x] Function calls with comma‑separated args; missing comma errors.
- [x] Trailing comma policy: allowed in parameter lists, disallowed in call args.

2.3 Precedence & Grouping
- [x] Operator precedence: `* /` > `+ -`; left‑associative.
- [x] Parentheses for grouping, multi‑line formatting.

2.4 Tooling & Samples
- [x] CLI `parse` subcommand pretty‑prints AST.
- [x] lumi-tests samples for positive and negative cases.

2.5 Error Coverage (Optional)
- [x] Improve parse error messages/spans where useful.
- [x] More negative tests (unknown idents, reserved keywords).

## Phase 3 — Typer & IR (Next)

→ Next focus: Phase 4 — Namespacing + Strings (parse/type) + Std Collections stubs; then Phase 5 runtime.

3.1 Typer Core
- [x] Function env: collect signatures (name, params, ret, effect).
- [x] Rules for `Int`/`Bool`, variables, `+ - * /`, and calls.
- [x] Arity/return checks; unknown function errors (spans deferred to 3.8).

3.2 Effects (Stub)
 - [x] Accept `Effect::None|Pure` initially; plan enforcement later.
 - [x] Reject Bool arithmetic; ensure function bodies match return types.

3.3 IR Shape
- [x] Minimal SSA‑like IR in `crates/ir` (Function, Instr, Type; single block for now).
- [x] Instrs: `IConst`, `IBin(Add|Sub|Mul|Div)`, `Call`, `Ret`.
 - [x] Unit tests: construct simple functions and call variants.

3.4 Lowering
- [x] Lower AST → IR guided by typer; preserve minimal names/locals.

3.5 Integration
- [x] CLI `build`: parse → type‑check → lower to IR → emit Wasm.
- [x] Codegen replaces const‑eval; keep const‑eval behind a feature flag (optional).
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

## Phase 4 — Namespacing + Strings (Parse/Type) + Std Collections stubs + Small Optimizations

- 4.1 Namespacing (no tech debt)
  - [x] Add namespaced call syntax (paths): `std::str::len(s)`, `std::list::push(l,x)`, `std::map::get(m,k)`, `std::set::contains(s,x)`.
  - [x] Typer: accept namespaced callees as exact names (parser done; built-ins via 4.3).

- 4.2 Strings (parse/type only)
  - [ ] Add `Type::Str` and `Expr::Str` with escapes and multi‑line string literal support.
  - [ ] Typer: make `Str` first‑class; allow equality and concatenation in `std::str`.
  - [ ] Tests: single‑line, multi‑line, escapes, span diagnostics.

- 4.3 Std Collections (type stubs only)
  - [ ] Introduce core types: `List<T>`, `Set<T>`, `Map<K,V>`, plus `Option<T>`/`Result<T,E>`.
  - [ ] Provide minimal namespaced APIs (signatures for typer):
        `std::list::{new,with_capacity,len,get,set,push,pop,slice}`;
        `std::set::{new,len,insert,remove,contains}`;
        `std::map::{new,len,insert,remove,get,contains}`.
  - [ ] No codegen/runtime yet; only enable type‑checking.

- 4.4 Small Optimizations & DX
  - [x] Optional Wasm name section for function names (`--debug-names`).
  - [ ] Preallocate HashMaps/Vecs in typer/parser based on known capacities.
  - [ ] Reuse a shared Wasmtime `Engine` in tests (e.g., `once_cell`) to speed up instantiation.
  - [ ] Add `[profile.release]` tuning (e.g., `lto = "thin"`, `codegen-units = 1`).
  - [ ] Gate build stage logs behind `--verbose` (keep default quieter).
  

## Phase 5 — Codegen IR → Wasm

5.1 Module & Signatures
- [ ] Types, function indices, and exports.
- [ ] Deduplicate function signatures in the Wasm Type section (reuse type indices).

5.2 Locals & Stack
- [ ] Local allocation for temps; map IR values to stack ops.

5.3 Ops & Calls
- [ ] Encode `IConst`, `IBin`, and `Call` to Wasm; verify results in IT.
- [ ] Switch calls to use callee indices; update IR lowering accordingly (resolve names to indices).

5.4 Replace Const‑Eval
- [ ] Switch CLI build to IR→Wasm path by default; keep const‑eval for quick checks.
 - [ ] Remove const‑eval fallback once IR path is stable (optional cleanup).

5.5 Tests
- [ ] Extend e2e tests to verify outputs across samples via Wasmtime.

## Phase 6 — Contracts & Effects (Safety)

6.1 Syntax
- [ ] `require { expr }` / `ensure { expr }`, effects `pure|mut|io`.

6.2 Typer Rules
- [ ] Enforce effects and basic purity constraints.
- [ ] Generate placeholders for verification conditions (VCs).

6.3 Runtime Guards
- [ ] Lower contracts to guards that trap on violation.

6.4 Metadata & Proofs
- [ ] Emit custom sections for contracts/effects/VCs; plan `lumiverify`.

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

## Nice‑to‑Have (Backlog)

- [ ] WASI `print` intrinsic (Phase 8) for observable output.
- [ ] Arrays + while loops with invariants (Phase 7).
- [ ] Proof‑carrying Wasm prototype (`lumiverify`) (Phase 10).
