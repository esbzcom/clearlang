# lumi-lang

Lumi is a dependently-typed programming language that compiles to WebAssembly.
It aims to be simple like Python and safe like SPARK Ada, with refinement types,
contracts, and an effect system to eliminate entire classes of bugs at compile time.

---

## ✨ Core Ideas

- **Dependent / refinement types**  
  Encode logical properties directly in types, e.g.  
  ```lumi
  type Nat = Int where n >= 0
  ```
- **Contracts (`require` / `ensure`)**  
  Preconditions and postconditions that must be provable or enforced at runtime.
- **Effect system** (`pure`, `mut`, `io`)  
  Separates pure math, local mutation, and I/O.
- **Totality**  
  Pure functions must terminate; loops require invariants or bounds.
- **Resource / linear types**  
  Guarantee safe handling of resources (e.g., no double-spend in crypto).

---

## 🚀 Applications

- **Safe app backends** — Lumi logic compiled to WASM, UI in React/Swift/Kotlin.  
- **Smart contracts** — strong invariants (no negative balances, total supply preserved, no reentrancy).  
- **High-assurance systems** — finance, medical, aerospace, where bugs are unacceptable.  

---

## 🔧 Implementation Strategy

- **Target:** WebAssembly (portable across web, mobile, blockchain).  
- **Compiler Host:** Rust (performance, safety, WASM tooling).  
- **Parser:** [chumsky](https://github.com/zesterer/chumsky) combinator parser.  
- **Codegen:** [`wasm-encoder`](https://github.com/bytecodealliance/wasm-tools).  

---

## 📚 Roadmap

1. **Phase 0** — Setup workspace (Rust, Wasmtime, wasm-tools).  
2. **Phase 1** — Hello WASM (emit trivial `main = 42`).  
3. **Phase 2** — Parser: AST with functions, expressions, binary ops.  
4. **Phase 3** — Typer & IR: SSA-like intermediate representation.  
5. **Phase 4** — Codegen: IR → WASM.  
6. **Phase 5** — Effects & contracts (`pure|mut|io`, `require`, `ensure`).  
7. **Phase 6** — Arrays & while-loops with invariants.  
8. **Phase 7** — WASI I/O (`print`).  
9. **Phase 8 (optional)** — SMT solver integration for proofs.  

---

## ✅ Current Status

- Phase 1 complete: emits a minimal Wasm module exporting `main() -> i32` that returns `42`.
- Phase 2 in progress: parser supports functions, parameters, Int/Bool, binary ops, and calls.
- Temporary build path uses const-eval to produce Wasm for Int-returning programs (supports literals, `+ - * /`, variables/params, and function calls).
- CLI subcommands:
  - `emit-hello` — writes a trivial Wasm (`main -> i32 42`).
  - `parse <file>` — parses a Lumi source and prints the AST.
  - `build <file> -o <out.wasm>` — const-eval source→Wasm for Int programs (no Booleans yet).
- Tests cover arithmetic, call expressions, and error cases.

## Safety Levels

Lumi pursues defense-in-depth with three complementary safety layers:

1) Compile Time
- Types/effects: ill-typed or effect-unsafe programs are rejected.
- Contracts: `require`/`ensure` planned with verification conditions; in early phases, compile to runtime checks.
- Proofs (later): generate and optionally discharge SMT obligations; ship proof artifacts.

2) Load Time
- Wasm validation: `wasm-tools validate out.wasm` and Wasmtime module validation.
- Policy checks: deny disallowed imports; require metadata/custom sections when applicable.
- Proof-carrying code (later): `lumiverify` checks proof sections before instantiation.

3) Runtime
- Contract guards trap deterministically on violation (when compiled in).
- Sandboxing: Wasm memory isolation by default.
- Host limits: Wasmtime fuel/epoch deadlines, memory/table caps to prevent hangs/DoS.

## 🏃 How To Run

- Codegen tests: `cargo test -p lumi-codegen-wasm`
- Parser tests: `cargo test -p lumi-parser`
- Emit hello.wasm (Phase 1): `cargo run -p lumi-cli -- emit-hello -o tmp/hello.wasm`
- Parse a Lumi file (Phase 2): `cargo run -p lumi-cli -- parse path/to/file.lumi`
- Build (const-eval: Int-returning programs with `+ - * /` and calls):
  - `cargo run -p lumi-cli -- build lumi-tests/01_hello.lumi -o tmp/hello_prog.wasm`
  - `cargo run -p lumi-cli -- build lumi-tests/02_arith.lumi -o tmp/arith.wasm`
  - Run: `wasmtime --invoke main tmp/arith.wasm`
  - Validate: `wasm-tools validate tmp/arith.wasm`

## lumi-tests Samples

- Location: `lumi-tests/`
- Parse a sample: `cargo run -p lumi-cli -- parse lumi-tests/01_hello.lumi`
- See `lumi-tests/README.md` for a full list and commands (includes a negative case `06_trailing_call_comma.lumi` that should fail to parse).

---

## 📖 Vision

Lumi aspires to be:

- **Simple** — approachable syntax like Python.  
- **Safe** — formal guarantees like SPARK Ada.  
- **Portable** — runs anywhere via WebAssembly.  
- **Proof-oriented** — bugs prevented at compile time, not found at runtime.  

---

## 🔒 License

Currently private, all rights reserved.  
An open-source license (e.g., MIT or Apache-2.0) may be applied when Lumi is released publicly.
