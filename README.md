# lumi-lang

Lumi is a dependently-typed programming language that compiles to WebAssembly.  
It aims to be *simple like Python* but *safe like SPARK Ada*, with refinement types, contracts, and an effect system to eliminate entire classes of bugs at compile time.

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

- Parser prototype is working (can parse function calls and nested expressions).  
- Tests cover arithmetic, call expressions, and error cases.  
- Next step: implement **typer** and IR lowering.  

---

## 📖 Vision

Lumi aspires to be:

- **Simple** — approachable syntax like Python.  
- **Safe** — formal guarantees like SPARK Ada.  
- **Portable** — runs anywhere via WebAssembly.  
- **Proof-oriented** — bugs prevented at compile time, not found at runtime.  

---

## 🔒 License

Currently **private, all rights reserved**.  
A suitable open-source license (e.g., MIT or Apache-2.0) may be applied when Lumi is released publicly.
