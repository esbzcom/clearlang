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

→ Next focus: 3.5 Integration (replace const‑eval in the build path).

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
- [ ] CLI `build`: parse → type‑check → lower to IR → emit Wasm.
- [ ] Codegen replaces const‑eval; keep const‑eval behind a feature flag (optional).
 - [ ] Add `--validate` flag to run `wasm-tools validate` on outputs.

3.6 Tests
- [ ] Typer errors: unknown function, arity mismatch, type mismatch.
- [ ] IR/codegen e2e for `01_hello`, `02_arith`, `03_nested_calls`, `04_multiline_call`, `05_trailing_param_comma`.
- [ ] Keep negative parse case `06_trailing_call_comma`.

3.7 Docs
- [ ] Add `docs/typing.md` (rules/spec) and `docs/ir.md` (IR shape).
- [ ] Update README “Current Status” to mark Phase 3 in progress.

3.8 Diagnostics & Spans
- [ ] Attach source spans in AST via chumsky (`map_with_span`) for identifiers and expressions.
- [ ] Propagate spans into typer errors (unknown var/fn, arity, return/type mismatch).
- [ ] Add tests that assert span presence/format in error messages.
- [ ] Gate Phase 4 switch (IR→Wasm as default) on basic span coverage to avoid tech debt.

## Phase 4 — Codegen IR → Wasm

4.1 Module & Signatures
- [ ] Types, function indices, and exports.

4.2 Locals & Stack
- [ ] Local allocation for temps; map IR values to stack ops.

4.3 Ops & Calls
- [ ] Encode `IConst`, `IBin`, and `Call` to Wasm; verify results in IT.

4.4 Replace Const‑Eval
- [ ] Switch CLI build to IR→Wasm path by default; keep const‑eval for quick checks.
 - [ ] Remove const‑eval fallback once IR path is stable (optional cleanup).

4.5 Tests
- [ ] Extend e2e tests to verify outputs across samples via Wasmtime.

## Phase 5 — Contracts & Effects (Safety)

5.1 Syntax
- [ ] `require { expr }` / `ensure { expr }`, effects `pure|mut|io`.

5.2 Typer Rules
- [ ] Enforce effects and basic purity constraints.
- [ ] Generate placeholders for verification conditions (VCs).

5.3 Runtime Guards
- [ ] Lower contracts to guards that trap on violation.

5.4 Metadata & Proofs
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

- [ ] WASI `print` intrinsic (Phase 7) for observable output.
- [ ] Arrays + while loops with invariants (Phase 6).
- [ ] Proof‑carrying Wasm prototype (`lumiverify`) (Phase 9).
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

## Phase 3 — Typer & IR (Next)

→ Next focus: 3.5 Integration (replace const-eval in the build path).

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
- [ ] CLI `build`: parse + type-check + lower to IR + emit Wasm.
- [ ] Codegen replaces const-eval; keep const-eval behind a feature flag (optional).
 - [ ] Add `--validate` flag to run `wasm-tools validate` on outputs.

3.6 Tests
- [ ] Typer errors: unknown function, arity mismatch, type mismatch.
- [ ] IR/codegen e2e for `01_hello`, `02_arith`, `03_nested_calls`, `04_multiline_call`, `05_trailing_param_comma`.
- [ ] Keep negative parse case `06_trailing_call_comma`.

3.7 Docs
- [ ] Add `docs/typing.md` (rules/spec) and `docs/ir.md` (IR shape).
- [ ] Update README "Current Status" to mark Phase 3 in progress.

3.8 Diagnostics & Spans
- [ ] Attach source spans in AST via chumsky (`map_with_span`) for identifiers and expressions.
- [ ] Propagate spans into typer errors (unknown var/fn, arity, return/type mismatch).
- [ ] Add tests that assert span presence/format in error messages.
- [ ] Gate Phase 4 switch (IR→Wasm as default) on basic span coverage to avoid tech debt.

## Phase 4 — Codegen IR → Wasm

4.1 Module & Signatures
- [ ] Types, function indices, and exports.

4.2 Locals & Stack
- [ ] Local allocation for temps; map IR values to stack ops.

4.3 Ops & Calls
- [ ] Encode `IConst`, `IBin`, and `Call` to Wasm; verify results in IT.

4.4 Replace Const-Eval
- [ ] Switch CLI build to IR→Wasm path by default; keep const-eval for quick checks.
 - [ ] Remove const-eval fallback once IR path is stable (optional cleanup).

4.5 Tests
- [ ] Extend e2e tests to verify outputs across samples via Wasmtime.

## Phase 5 — Contracts & Effects (Safety)

5.1 Syntax
- [ ] `require { expr }` / `ensure { expr }`, effects `pure|mut|io`.

5.2 Typer Rules
- [ ] Enforce effects and basic purity constraints.
- [ ] Generate placeholders for verification conditions (VCs).

5.3 Runtime Guards
- [ ] Lower contracts to guards that trap on violation.

5.4 Metadata & Proofs
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

## Nice-to-Have (Backlog)

- [ ] WASI `print` intrinsic (Phase 7) for observable output.
- [ ] Arrays + while loops with invariants (Phase 6).
- [ ] Proof-carrying Wasm prototype (`lumiverify`) (Phase 9).
