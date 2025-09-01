# Lumi Compiler Roadmap (Condensed)

Lumi is a dependently-typed, proof-oriented language targeting WebAssembly.  
Goal: **prevent bugs at compile time, enforce invariants at runtime, and run safely anywhere via WASM.**

---

## Phase Overview

1. **Setup (Phase 0–1)**  
   - Rust workspace, toolchain, Wasmtime, wasm-tools.  
   - Emit minimal WASM module (`main = 42`).  

2. **Core Language (Phase 2–4)**  
   - Parser + AST.  
   - Type checker + SSA-like IR.  
   - Codegen IR → WASM (programs run in Wasmtime).  

3. **Safety Foundations (Phase 5–6)**  
   - Contracts (`require`/`ensure`) → runtime traps.  
   - Effect system (`pure|mut|io`) enforced at type and bytecode level.  
   - Arrays + loops with invariants, totality checks.  

4. **Interop (Phase 7)**  
   - WASI integration (`print`, I/O).  

5. **Verification & Proofs (Phase 8–9)**  
   - Emit SMT-LIB2 conditions for contracts and invariants.  
   - Optional solver integration (Z3/Why3).  
   - Proof-carrying WASM: custom sections (`lumi.contracts`, `lumi.proofs`, …).  
   - `lumiverify` tool validates WASM + contracts before execution.  

6. **Domain Adoption (Phase 10)**  
   - Libraries/templates for financial contracts, safe web/mobile backends.  
   - Ready-to-use Lumi modules to accelerate adoption.  

---

## Deliverables

- **lumic**: compiler from `.lumi` → `.wasm` + proof artifacts.  
- **lumiverify**: verifier that checks contracts, purity, and proofs pre-execution.  
- **Templates**: reference implementations for finance, web, and smart contracts.  

---

## Vision

**Simple like Python. Safe like SPARK Ada. Portable via WASM.**  
A platform for **provably correct apps and smart contracts**.
