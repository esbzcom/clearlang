# Lumi → WASM Compiler Plan (Using ChatGPT Codex Assistance)

This is a step-by-step roadmap to implement the Lumi language, compile it to WebAssembly, and run it in an existing WASM engine like Wasmtime. The workflow assumes you will use ChatGPT/Codex (e.g., GitHub Copilot Chat in VS Code) to generate much of the code.

---

## Refined Compiler Phases

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

### Phase 4 — Codegen IR → WASM
- Translate IR into valid WASM with `wasm-encoder`.  
- Export functions.  
- ✅ Output: `.lumi` programs run as `.wasm` modules.

### Phase 5 — Contracts & Effects
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


## End Result

- `lumic source.lumi -o out.wasm` produces valid WASM.
- Language supports: `pure|mut|io`, contracts, while-loops, arrays, WASI printing.
- Path to solver integration later.

---

## Safety Model Checklist

- Compile Time
  - Type/effect checks reject unsafe programs.
  - Contracts lower to guards early; later, emit verification conditions and proofs.
- Load Time
  - Validate Wasm bytes (`wasm-tools validate`, Wasmtime module validation).
  - Optional `lumiverify` verifies proof/metadata custom sections.
- Runtime
  - Contract guards trap deterministically on violation.
  - Wasm sandboxing; enable fuel/epoch/memory limits in host.
