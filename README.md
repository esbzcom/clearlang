# lumi-lang

Lumi is a dependently-typed programming language that compiles to WebAssembly.
It aims to be simple like Python and safe like SPARK Ada, with refinement types,
contracts, and an effect system to eliminate entire classes of bugs at compile time.

---

## Core Ideas

- Dependent/refinement types: encode logical properties in types (e.g., `type Nat = Int where n >= 0`).
- Contracts (`require`/`ensure`): pre/postconditions proved statically or guarded at runtime.
- Effects (`pure`, `mut`, `io`): clearly separate pure computation, local mutation, and I/O.
- Totality: pure functions must terminate; loops require invariants or bounds.
- Resource/linear discipline: avoid misuse (e.g., double-spend) by construction.

---

## Applications

- Safe app backends: Lumi logic to Wasm; UI in React/Swift/Kotlin.
- Smart contracts: invariants like non-negative balances and preserved totals.
- High-assurance systems: finance, medical, aerospace where bugs are unacceptable.

---

## Implementation Strategy

- Target: WebAssembly (portable across web, mobile, blockchain).
- Compiler host: Rust (performance, safety, WASM tooling).
- Parser: chumsky (combinator parser).
- Codegen: wasm-encoder.

---

## Roadmap

1. Phase 0 - Setup workspace (Rust, Wasmtime, wasm-tools).
2. Phase 1 - Hello Wasm (emit trivial `main = 42`).
3. Phase 2 - Parser: functions, expressions, binary ops.
4. Phase 3 - Typer & IR: SSA-like intermediate representation.
5. Phase 4 - Namespacing + Strings (parse/type) + Std Collections stubs + small DX.
6. Phase 5 - Codegen: IR + Wasm (types/indices, ops, memory/runtime).
7. Phase 6 - Effects & contracts (`pure|mut|io`, `require`, `ensure`).
8. Phase 7 - Arrays & while-loops with invariants.
9. Phase 8 - WASI I/O (`print`).
10. Phase 9 (optional) - Proof-carrying Wasm / SMT integration.

---

## Current Status

- Phase 1 complete: minimal Wasm module exporting `main() -> i32` returning `42`.
- Phase 3 complete: typer validates programs and lowers to IR (SSA-like).
- Phase 4 in progress:
  - 4.1 Namespacing: path-call syntax `seg::seg::name(args...)` supported.
  - 4.2 Strings (parse/type): `Str` literals (escapes, multi-line); `std::str::{len, concat, eq}` stubs in typer.
  - Next: 4.3 Collections (type stubs) and 4.4 DX improvements.
- Build path uses IR+Wasm by default. Use `--validate` to run `wasm-tools validate`.

---

## CLI

- Commands:
  - `emit-hello`: emits a trivial Wasm with `main() -> i32` returning 42.
  - `parse <FILE>`: parses and pretty-prints the AST for a Lumi source file.
  - `build <FILE> [--out <PATH>] [--validate] [--debug-names]`: compiles to Wasm via IR.
  - `run <FILE> [--invoke <name>]`: runs a Wasm file (default export: `main`).
- Build pipeline: Parse -> Type-check -> Lower to IR -> Codegen (IR->Wasm) -> write output.
- Flags:
  - `-o, --out <PATH>`: output Wasm path; creates parent directories if needed.
  - `--validate`: run `wasm-tools validate` on the produced Wasm (optional).
  - `--debug-names`: include a Wasm name section with function names.
- Examples:
  - `lumi emit-hello -o tmp/hello.wasm`
  - `lumi parse lumi-tests/01_hello.lumi`
  - `lumi build lumi-tests/02_arith.lumi -o out/arith.wasm --validate`
  - `lumi run out/arith.wasm`

---

## Docs

- Typing rules: `docs/typing.md`
- IR shape and encoding: `docs/ir.md`

---

## Design Philosophy

- Simple Is Best: one obvious way; avoid premature features/complexity.
- Prove Correct: small steps with tests; explicit types and spans.
- AI-Friendly: consistent, structured errors; stable CLI and predictable outputs.

---

## Safety Levels

1) Compile Time
- Types/effects: ill-typed or effect-unsafe programs are rejected.
- Contracts (planned): `require`/`ensure` with verification; runtime guards early on.

2) Load Time
- Wasm validation: `wasm-tools validate out.wasm` and Wasmtime module validation.
- Policy checks: deny disallowed imports; require metadata/custom sections when applicable.

3) Runtime
- Contract guards trap deterministically on violation (when compiled in).
- Sandboxing: Wasm memory isolation by default.
- Host limits: Wasmtime fuel/epoch deadlines, memory/table caps.

---

## How To Run

- Codegen tests: `cargo test -p lumi-codegen-wasm`
- Parser tests: `cargo test -p lumi-parser`
- All tests (workspace): `cargo test --workspace`
- Full pipeline test: `cargo test -p lumi-codegen-wasm --test full_pipeline`
- Build and run samples:
  - `cargo run -p lumi-cli -- build lumi-tests/01_hello.lumi -o tmp/hello_prog.wasm`
  - `wasmtime --invoke main tmp/hello_prog.wasm`

### Windows

- Prereqs: Install Rust (MSVC) and VS Build Tools (C++ workload).
- Build release: `cargo build -p lumi-cli --release` -> `target\release\lumi.exe`
- Install to PATH (optional): `cargo install --path crates/cli --bin lumi`

---

## Git Hooks (Pre-Commit)

- Enable hooks: `git config core.hooksPath .githooks`
- Hook runs: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace`
- Skip toggles: `SKIP_PRECOMMIT=1`, `SKIP_FMT=1`, `SKIP_CLIPPY=1`, `SKIP_TESTS=1`

---

## lumi-tests Samples

- Location: `lumi-tests/`
- Quick parse: `cargo run -p lumi-cli -- parse lumi-tests/01_hello.lumi`
- Includes a negative case: `lumi-tests/06_trailing_call_comma.lumi`

---

## Vision

- Simple - approachable syntax.
- Safe - formal guarantees.
- Portable - runs anywhere via WebAssembly.
- Proof-oriented - prevent bugs before they run.

---

## License

Currently private, all rights reserved.
An open-source license (e.g., MIT or Apache-2.0) may be applied when Lumi is released publicly.

