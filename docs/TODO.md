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

## Phase 2 — Parser (Current)

- [x] Parse functions, params (allow trailing), Int/Bool, literals, calls.
- [x] Operator precedence: `* /` > `+ -`.
- [x] Tests for valid/invalid calls (missing/trailing commas).
- [x] CLI `parse` subcommand to pretty‑print AST.
- [x] End‑to‑end smoke via const‑eval build path (temporary).

Optional niceties
- [ ] Improve parse error messages/spans where useful.
- [ ] Add more negative tests (unknown idents, reserved keywords).

## Phase 3 — Typer & IR (Next)

Typer
- [ ] Function environment: collect signatures (name, params, ret, effect).
- [ ] Type rules for `Int`, `Bool`, `+ - * /`, variables, and calls.
- [ ] Arity/return checks; unknown function errors with spans.
- [ ] Effects (stub): accept `Effect::None|Pure`; reject `Bool` arithmetic.

IR
- [ ] Define minimal SSA‑like IR in `crates/ir` (Function, Block, Instr, Type).
- [ ] Instrs: `IConst`, `IBin(Add|Sub|Mul|Div)`, `Call`, `Ret`.
- [ ] Lower AST → IR (guided by typer results), preserve names minimally.

Integration
- [ ] CLI `build`: parse → type‑check → lower to IR → emit Wasm.
- [ ] Codegen (replace const‑eval): map IR to Wasm (integers + calls first).
- [ ] Keep const‑eval behind a feature flag (optional) for quick tests.

Tests
- [ ] Typer error tests: unknown function, arity mismatch, type mismatch.
- [ ] IR/codegen e2e: expect outputs for `01_hello`, `02_arith`, `03_nested_calls`, `04_multiline_call`, `05_trailing_param_comma`.
- [ ] Keep negative parse case `06_trailing_call_comma`.

Docs
- [ ] Add `docs/typing.md` (rules/spec) and `docs/ir.md` (IR shape).
- [ ] Update README “Current Status” to mark Phase 3 once scaffold lands.

## Phase 4 — Codegen IR → Wasm

- [ ] Export functions, correct type signatures.
- [ ] Encode integer ops/calls; add simple local allocation for temps.
- [ ] Extend tests to compare outputs with expected values (Wasmtime).

## Phase 5 — Contracts & Effects (Safety)

- [ ] Syntax: `require { expr }` / `ensure { expr }`, effects `pure|mut|io`.
- [ ] Typer: enforce effects; generate VCs for contracts (placeholder).
- [ ] Runtime: lower contracts to guards that trap on violation.
- [ ] Optionally emit verification metadata (custom sections) for future proofs.

## Safety & Tooling

- [ ] Always validate generated Wasm: `wasm-tools validate` in CI.
- [ ] Add CI to run `cargo test` across workspace; run pipeline tests.
- [ ] Document Wasmtime fuel/epoch/memory limits for runtime safety.
- [ ] Add `--contracts=runtime|hybrid|static` flag design (future).

## Developer Experience

- [ ] Simple tracing/logging for pipeline stages in CLI.
- [ ] Add `cargo xtask` or Makefile for common flows (build/validate/run).
- [ ] Consistent error types and messages across crates.

## Nice‑to‑Have (Backlog)

- [ ] WASI `print` intrinsic (Phase 7) for observable output.
- [ ] Arrays + while loops with invariants (Phase 6).
- [ ] Proof‑carrying Wasm prototype (`lumiverify`) (Phase 9).
