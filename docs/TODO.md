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

## Phase 4 — Namespacing + Strings (Parse/Type) + Std Collections stubs + Small Optimizations

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
  - [ ] Typer: rules for `Option`/`Result` constructors and `match`; no codegen/runtime yet.
  - [ ] Tests: functions returning `Option<String>` / `Result<Int,E>`; spanful diagnostics.
- 4.6 Collections Signatures (type-only)
  - [ ] Provide namespaced APIs using Option/Result:
        `std::list::{new,len,push,pop} (pop -> Option<T>)`;
        `std::set::{new,len,insert,remove,contains}`;
        `std::map::{new,len,insert,remove,get (-> Option<V>), contains}`.
- 4.7 Collections Docs/DX
  - [ ] Document naming (modules lower-case: `std::list`; types PascalCase: `List<T>`),
        dual-API guidance (precondition vs Option/Result), and JSON error codes.
  - [ ] DX: Split typer — move rules to `check.rs` and IR lowering to `lower.rs`; keep `lib.rs` as public entry.

- 4.8 Small Optimizations & DX
  - [x] Optional Wasm name section for function names (`--debug-names`).
  - [ ] Preallocate HashMaps/Vecs in typer/parser based on known capacities.
  - [ ] Reuse a shared Wasmtime `Engine` in tests (e.g., `once_cell`) to speed up instantiation.
  - [ ] Add `[profile.release]` tuning (e.g., `lto = "thin"`, `codegen-units = 1`).
  - [ ] Gate build stage logs behind `--verbose` (keep default quieter).
  - [ ] DX: When adding `--verbose`, refactor CLI by splitting subcommands into `commands/{emit_hello,parse,build,run}.rs` and small helpers.
  

## Phase 5 — Codegen IR → Wasm

DX Prep
- [ ] Split `codegen-wasm` into `trivial.rs` (emit_trivial_main), `const_eval.rs` (emit_from_ast + eval), and `ir.rs` (IR→Wasm encoder); re-export from `lib.rs`.

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

5.6 Strings Runtime
- [ ] Define String memory model (ptr + len; utf-8 bytes).
- [ ] Implement `std::str::{len, concat, eq}` via intrinsics or a small runtime.
- [ ] Provide a minimal bump allocator or reuse host env for concat.
- [ ] Add e2e tests for string ops; document runtime model.

## Phase 6 — Contracts & Effects (Safety)

6.1 Syntax
- [ ] `require { expr }` / `ensure { expr }`, effects `pure|mut|io`.

6.2 Typer Rules
- [ ] Enforce effects and basic purity constraints.
- [ ] Allow multiple `require`/`ensure` (conjoined semantics).
- [ ] Generate Verification Conditions (VCs) for pure, expression-bodied functions.
- [ ] CLI: `--emit-vcs` outputs stable JSON (function, vc_id, pre, post, smt2, status).

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

## Nice-to-Have (Backlog)

- [ ] WASI `print` intrinsic for observable output.
- [ ] Proof-carrying Wasm prototype (`lumiverify`).
- [ ] On-chain attestation (anchoring) for signatures (EVM registry + IPFS URIs).
