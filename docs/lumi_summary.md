# Lumi Language Discussion — Summary & Plan

This document summarizes the full discussion about designing the **Lumi** language, its goals, relation to existing languages, and the step-by-step plan to implement it.

---

## 1. Motivation & Concept

- The user wanted a **bug-proof language**, inspired by dependently-typed languages (Coq, Agda, Idris, F*).
- Goal: a language with **pure mathematical reasoning**, preventing entire classes of bugs at compile time.
- Proposed language name: **Lumi** (light, clarity, purity).

---

## 2. Core Ideas of Lumi

- **Dependent / refinement types**  
  Types can carry logical properties (e.g., `Nat = Int where n >= 0`).

- **Contracts (`require` / `ensure`)**  
  Preconditions and postconditions must be provable, else code won’t compile.

- **Effect system** (`pure`, `mut`, `io`)  
  Separates pure math, local mutation, and I/O effects.

- **Totality**  
  Pure functions must terminate; loops need invariants or bounds.

- **Resource/linear types**  
  Useful for crypto contracts (ensures no double-spend).

---

## 3. Comparison with Existing Languages

### Dependently Typed Languages
- **Coq, Agda, Idris**: strong proofs, but academic/niche.
- **F***: industrial use (crypto proofs, Project Everest).
- **SPARK Ada**: widely used in aerospace/rail/defense for certified software.

### Practical Mainstream
- **Rust**: prevents memory/data-race bugs, but not business logic errors.
- **Elm**: no runtime exceptions, but limited domain.

**Lumi vs Rust**
- Rust: safety for memory & concurrency.
- Lumi: safety for *domain invariants* and *business logic correctness*.

---

## 4. Applications of Lumi

- **Generic apps**: Lumi core for logic, UI in React/Swift/Kotlin via WASM FFI.
- **Smart contracts**: perfect fit, because it can guarantee invariants like total supply conservation, no negative balances, no reentrancy.

---

## 5. Implementation Strategy

- **Target = WebAssembly (WASM)**  
  Chosen for portability (web, mobile, blockchain).

- **Compiler Host = Rust**  
  Rust is chosen to implement the Lumi compiler because:
  - Great ecosystem for WASM tooling (`wasm-encoder`, `wasm-bindgen`).
  - Safety + performance.
  - But Python could be used for rapid prototyping (emit `.wat` and use `wat2wasm`).

---

## 6. Refined Compiler Phases

## Refined Compiler Phases for Lumi

### Phase 0 — Workspace & Toolchain
- **Set up Rust** toolchain, Wasmtime, `wasm-tools`.  
- Scaffold Cargo workspace with crates:  
  - `cli`, `parser`, `ast`, `typer`, `ir`, `codegen-wasm`, later `verifier`.  
- ✅ Output: empty repo, builds cleanly.

### Phase 1 — Minimal WASM (“Hello Lumi”)
- Emit trivial WASM module exporting a `main` that returns `42`.  
- Validate with `wasm-tools`, run with Wasmtime.  
- ✅ Output: `hello.wasm` with correct execution.

### Phase 2 — Parser & AST
- Design minimal syntax (`Program`, `Func`, `Expr`, `BinOp`).  
- Implement parser (e.g., `chumsky`).  
- Add tests for parsing.  
- ✅ Output: AST produced from `.lumi` source.

### Phase 3 — Typer & IR
- Define SSA-like IR (`IConst`, `IAdd`, `Call`, `Ret`).  
- Implement basic type checker (Int/Bool).  
- Lower AST → IR.  
- ✅ Output: IR validated for type safety.

### Phase 4 — Namespacing + Strings (Parse/Type) + Std Collections stubs
- Add namespaced call syntax (e.g., `std::str::len`, `std::list::push`).  
- Parse/type String literals (single + multi-line) and basic `Str` rules.  
- Provide List/Set/Map/Option/Result signatures for type-checking (no runtime yet).  
- ✅ Output: richer front-end with no runtime changes; clean APIs without global prefixes.

### Phase 5 — Codegen IR → WASM
- Translate IR into valid WASM with `wasm-encoder`.  
- Deduplicate function type signatures; calls use callee indices.  
- Introduce linear memory/runtime: Strings as (ptr,len); List<T> with grow/realloc.  
- ✅ Output: `.lumi` programs run as `.wasm` modules with strings/lists supported.

### Phase 6 — Contracts & Effects
- Extend AST with `pure|mut|io` and `require`/`ensure`.  
- Type checker enforces purity/effect rules.  
- Compile contracts into runtime guards (`if … unreachable`).  
- ✅ Output:  
  - Correct traps when contracts are violated.  
  - No memory ops in `pure` functions.  

### Phase 6 — Arrays, Loops & Totality
- Add arrays with bounds checks.  
- Add while-loops with required invariants or bounds.  
- Enforce totality in `pure` functions (termination checks).  
- ✅ Output: safe arrays and loops, proofs or guards for invariants.

### Phase 7 — WASI Interop
- Add intrinsic `print` via WASI `fd_write`.  
- Wire import/export through WASI.  
- ✅ Output: Lumi programs can perform basic I/O safely.

### Phase 8 — Verification Hooks (Proof-Oriented)
- Emit SMT-LIB2 verification conditions for contracts and invariants.  
- Add `--emit-smt` flag to CLI.  
- Optionally integrate solvers (Z3, Why3) for proof discharge.  
- ✅ Output: `.wasm` + proof conditions for static checking.

### Phase 9 — Proof-Carrying WASM (Optional Extension)
- Emit Lumi metadata into custom sections:  
  - `lumi.contracts`, `lumi.effects`, `lumi.resources`, `lumi.proofs`.  
- Implement `lumiverify` tool that:  
  - validates WASM,  
  - checks purity/resource constraints,  
  - validates contracts/proofs.  
- ✅ Output: verifiable Lumi binaries, load-time checking before execution.

### Phase 10 — Domain Templates
- Provide Lumi libraries/templates for:  
  - Safe financial contracts (balances, supply invariants).  
  - Web/Mobile app core logic (safe business rules).  
- ✅ Output: ready-to-use Lumi modules for real-world adoption.


## 7. Advantages of Lumi over Current Tech

- **Compared to Rust**: adds formal proofs of correctness, not just memory safety.
- **Compared to Solidity/Vyper**: stronger invariants, fewer runtime surprises, safer by design.
- **Compared to Move**: similar resource safety, but Lumi is designed to be simpler and more natural syntax-wise.

---

## 8. Next Steps

1. Prototype Lumi → WAT in Python (optional).  
2. Build Lumi → WASM compiler in Rust using the step-by-step plan.  
3. Add WASI interop for web/mobile apps.  
4. Extend with resources & crypto contract templates.  
5. Hook into SMT solvers for proofs.

---

## 9. Vision

Lumi aims to be:
- **Simple like Python**, but **safe like SPARK Ada**.  
- **Portable** (runs anywhere via WASM).  
- **Proof-oriented** (bugs prevented at compile time).  
- Suitable for **web apps, mobile apps, and smart contracts**.

---

## 10. Safety Levels (Defense-in-Depth)

- Compile Time: types/effects and contracts; generate verification conditions; early phases use runtime guards for contracts.
- Load Time: validate Wasm on load; enforce import/metadata policies; optional proof-carrying verification (`lumiverify`).
- Runtime: contract guards trap on violation; Wasm sandboxing + host resource limits (fuel, memory, epoch deadlines).
# Lumi Language Discussion — Summary & Plan

This document summarizes the full discussion about designing the **Lumi** language, its goals, relation to existing languages, and the step-by-step plan to implement it.

---

## 1. Motivation & Concept

- The user wanted a **bug-proof language**, inspired by dependently-typed languages (Coq, Agda, Idris, F*).
- Goal: a language with **pure mathematical reasoning**, preventing entire classes of bugs at compile time.
- Proposed language name: **Lumi** (light, clarity, purity).

---

## 2. Core Ideas of Lumi

- **Dependent / refinement types**  
  Types can carry logical properties (e.g., `Nat = Int where n >= 0`).

- **Contracts (`require` / `ensure`)**  
  Preconditions and postconditions must be provable, else code won't compile.

- **Effect system** (`pure`, `mut`, `io`)  
  Separates pure math, local mutation, and I/O effects.

- **Totality**  
  Pure functions must terminate; loops need invariants or bounds.

- **Resource/linear types**  
  Useful for crypto contracts (ensures no double-spend).

---

## 3. Comparison with Existing Languages

### Dependently Typed Languages
- **Coq, Agda, Idris**: strong proofs, but academic/niche.
- **F***: industrial use (crypto proofs, Project Everest).
- **SPARK Ada**: widely used in aerospace/rail/defense for certified software.

### Practical Mainstream
- **Rust**: prevents memory/data-race bugs, but not business logic errors.
- **Elm**: no runtime exceptions, but limited domain.

**Lumi vs Rust**
- Rust: safety for memory & concurrency.
- Lumi: safety for *domain invariants* and *business logic correctness*.

---

## 4. Applications of Lumi

- **Generic apps**: Lumi core for logic, UI in React/Swift/Kotlin via WASM FFI.
- **Smart contracts**: perfect fit, because it can guarantee invariants like total supply conservation, no negative balances, no reentrancy.

---

## 5. Implementation Strategy

- **Target = WebAssembly (WASM)**  
  Chosen for portability (web, mobile, blockchain).

- **Compiler Host = Rust**  
  Rust is chosen to implement the Lumi compiler because:
  - Great ecosystem for WASM tooling (`wasm-encoder`, `wasm-bindgen`).
  - Safety + performance.
  - But Python could be used for rapid prototyping (emit `.wat` and use `wat2wasm`).

---

## 6. Refined Compiler Phases

## Refined Compiler Phases for Lumi

### Phase 0 — Workspace & Toolchain
- **Set up Rust** toolchain, Wasmtime, `wasm-tools`.  
- Scaffold Cargo workspace with crates:  
  - `cli`, `parser`, `ast`, `typer`, `ir`, `codegen-wasm`, later `verifier`.  
- Output: empty repo, builds cleanly.

### Phase 1 — Minimal WASM ("Hello Lumi")
- Emit trivial WASM module exporting a `main` that returns `42`.  
- Validate with `wasm-tools`, run with Wasmtime.  
- Output: `hello.wasm` with correct execution.

### Phase 2 — Parser & AST
- Design minimal syntax (`Program`, `Func`, `Expr`, `BinOp`).  
- Implement parser (e.g., `chumsky`).  
- Add tests for parsing.  
- Output: AST produced from `.lumi` source.

### Phase 3 — Typer & IR
- Define SSA-like IR (`IConst`, `IAdd`, `Call`, `Ret`).  
- Implement basic type checker (Int/Bool).  
- Lower AST → IR.  
- Output: IR validated for type safety.

### Phase 4 — Codegen IR → WASM
- Translate IR into valid WASM with `wasm-encoder`.  
- Export functions.  
- Output: `.lumi` programs run as `.wasm` modules.

### Phase 5 — Contracts & Effects
- Extend AST with `pure|mut|io` and `require`/`ensure`.  
- Type checker enforces purity/effect rules.  
- Compile contracts into runtime guards (`if (...) unreachable`).  
- Output:  
  - Correct traps when contracts are violated.  
  - No memory ops in `pure` functions.  

### Phase 6 — Arrays, Loops & Totality
- Add arrays with bounds checks.  
- Add while-loops with required invariants or bounds.  
- Enforce totality in `pure` functions (termination checks).  
- Output: safe arrays and loops, proofs or guards for invariants.

### Phase 7 — WASI Interop
- Add intrinsic `print` via WASI `fd_write`.  
- Wire import/export through WASI.  
- Output: Lumi programs can perform basic I/O safely.

### Phase 8 — Verification Hooks (Proof-Oriented)
- Emit SMT-LIB2 verification conditions for contracts and invariants.  
- Add `--emit-smt` flag to CLI.  
- Optionally integrate solvers (Z3, Why3) for proof discharge.  
- Output: `.wasm` + proof conditions for static checking.

### Phase 9 — Proof-Carrying WASM (Optional Extension)
- Emit Lumi metadata into custom sections:  
  - `lumi.contracts`, `lumi.effects`, `lumi.resources`, `lumi.proofs`.  
- Implement `lumiverify` tool that:  
  - validates WASM,  
  - checks purity/resource constraints,  
  - validates contracts/proofs.  
- Output: verifiable Lumi binaries, load-time checking before execution.

### Phase 10 — Domain Templates
- Provide Lumi libraries/templates for:  
  - Safe financial contracts (balances, supply invariants).  
  - Web/Mobile app core logic (safe business rules).  
- Output: ready-to-use Lumi modules for real-world adoption.


## 7. Advantages of Lumi over Current Tech

- **Compared to Rust**: adds formal proofs of correctness, not just memory safety.
- **Compared to Solidity/Vyper**: stronger invariants, fewer runtime surprises, safer by design.
- **Compared to Move**: similar resource safety, but Lumi is designed to be simpler and more natural syntax-wise.

---

## 8. Next Steps

1. Prototype Lumi + WAT in Python (optional).  
2. Build Lumi + WASM compiler in Rust using the step-by-step plan.  
3. Add WASI interop for web/mobile apps.  
4. Extend with resources & crypto contract templates.  
5. Hook into SMT solvers for proofs.

---

## 9. Vision

Lumi aims to be:
- **Simple like Python**, but **safe like SPARK Ada**.  
- **Portable** (runs anywhere via WASM).  
- **Proof-oriented** (bugs prevented at compile time).  
- Suitable for **web apps, mobile apps, and smart contracts**.

---

## 10. Safety Levels (Defense-in-Depth)

- Compile Time: types/effects and contracts; generate verification conditions; early phases use runtime guards for contracts.
- Load Time: validate Wasm on load; enforce import/metadata policies; optional proof-carrying verification (`lumiverify`).
- Runtime: contract guards trap on violation; Wasm sandboxing + host resource limits (fuel, memory, epoch deadlines).
