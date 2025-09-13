# Lumi Language

## Introduction

**Lumi** is a new programming language designed to make software — especially smart contracts — provably safe.  
Where today’s languages leave room for bugs, hacks, and costly audits, Lumi ensures that unsafe code simply won’t compile.  
It combines **familiar syntax (Python/Java-like)** with **mathematical proofs of correctness**, hides complexity from the developer, and is built to be **AI-friendly**, so both humans and AI tools can generate secure, high-quality code more easily.  
Because it compiles to **WebAssembly (WASM)**, Lumi runs anywhere — web, mobile, or blockchain — making it a practical foundation for trustworthy apps and next-generation crypto systems.

---

## 1. Motivation & Concept

- The user wanted a **bug-proof language**, inspired by dependently-typed languages (Coq, Agda, Idris, F*).
- Goal: a language with **pure mathematical reasoning**, preventing entire classes of bugs at compile time.
- Proposed language name: **Lumi** (light, clarity, purity).
- Another key goal: **simplicity for the end user**.  
  Lumi should hide complex implementation details, leaving a clean, approachable syntax.  
  The learning curve should be **smooth and user-friendly**, so developers can be productive quickly without needing to understand deep theory upfront.
- **Familiar syntax**: Lumi adopts a **Python/Java-like syntax**, making it intuitive for most developers to learn and use without friction.
- **AI-friendly design**: Lumi is built with AI in mind. Its structure and clarity make it easier for AI tools to **generate complete processes directly from human instructions**.  
  Unlike today’s languages, Lumi code is **self-provable**: every generated function or contract can carry its own correctness guarantees, so incorrect or unsafe code cannot compile.

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

## 3. Comparison & Crypto Contracts

Lumi’s motivation comes from gaps and pain points in today’s languages:

### Dependently Typed Research Languages (Coq, Agda, Idris, F*)
- **Strengths**: allow rigorous mathematical proofs.  
- **Pain Points**: steep learning curve, complex syntax, mostly academic, hard to apply in production.  
- **Lumi’s Answer**: bring **dependent/refinement types** to a **Python/Java-like syntax**, making formal reasoning **accessible and user-friendly**, with AI support to guide proofs.

### SPARK Ada
- **Strengths**: proven correctness for mission-critical domains.  
- **Pain Points**: verbose syntax, niche ecosystem, slower onboarding for new developers.  
- **Lumi’s Answer**: keep the **rigor of contracts and proofs**, but hide complexity behind **simple, modern syntax** and automated solver integration.

### Rust
- **Strengths**: memory safety, fearless concurrency, great tooling.  
- **Pain Points**: does not address **business logic correctness** or domain invariants (e.g., balances never negative, workflows always terminate).  
- **Lumi’s Answer**: extend safety to **domain rules and contracts**, not just memory. Resource/linear types ensure **no double-spend**, making it ideal for crypto/finance.

### Elm
- **Strengths**: guarantees no runtime exceptions, great for UI.  
- **Pain Points**: limited in scope, not general-purpose, lacks formal proofs.  
- **Lumi’s Answer**: provide **totality and strong guarantees**, but keep the language **general-purpose** (apps, contracts, backend logic).

### Solidity / Vyper (Blockchain)
- **Strengths**: widely used for smart contracts.  
- **Pain Points**: easy to make critical errors (reentrancy, supply bugs, unchecked balances). Many exploits stem from missing formal guarantees.  
- **Lumi’s Answer**: enforce **compile-time proofs of invariants**, so contracts like token supply cannot compile if unsafe.

### Move (Meta’s blockchain language)
- **Strengths**: resource types to prevent double-spending.  
- **Pain Points**: specialized domain, less emphasis on user simplicity or AI-assisted workflows.  
- **Lumi’s Answer**: combine **resource safety** with a **friendly syntax and AI support**, making high-assurance code easier to write and maintain.

---

### Quick Comparison Table

| Pain Point (Today)                | Example / Cause                                   | Lumi Solution |
|----------------------------------|--------------------------------------------------|---------------|
| **Reentrancy exploits**           | DAO hack: state updated after external call      | Effect system forces state update before I/O; reentrancy unrepresentable |
| **Unchecked invariants**          | Balances can go negative; supply can inflate     | Refinement types + `require`/`ensure` enforce invariants at compile time |
| **Double spending**               | Assets cloned by mistake                         | Resource types guarantee single ownership, prevent duplication |
| **Infinite loops / non-termination** | Contracts hang, wasting gas or freezing logic | Totality check ensures loops must terminate or have invariants |
| **Complex unsafe syntax**         | Solidity low-level quirks, unsafe defaults       | Lumi offers Python/Java-like syntax, AI-friendly and simple |
| **Audit dependency**              | Human review required, costly and fallible       | Mathematical proofs at compile time — unsafe code won’t deploy |

---

### Why Lumi is a Better Fit for Crypto Contracts

Smart contracts are unforgiving: a single bug can permanently lock or steal millions. Existing languages (Solidity, Vyper, even Move) leave room for errors because they rely heavily on tests and audits, not mathematical guarantees.

**Crypto pain points today**:
- Reentrancy exploits draining funds.  
- Invariants like *total supply* or *balance ≥ 0* not enforced by compilers.  
- Double-spending or resource misuse due to unsafe abstractions.  
- Audits are expensive and still miss subtle bugs.  
- Developers face steep complexity or unsafe defaults.  

**How Lumi addresses them**:
- **Dependent/refinement types** ensure balances and supplies always respect constraints.  
- **Contracts (`require`/`ensure`)** act as provable guardrails — unsafe code won’t compile.  
- **Resource types** model tokens and assets directly, preventing double-spend by design.  
- **Totality guarantees** prevent non-terminating or unpredictable contract logic.  
- **AI-friendly syntax** lowers the barrier: both humans and AI can generate verifiable, safe contracts without requiring deep theorem-prover expertise.  
- **WASM portability** allows Lumi contracts to run across multiple chains and environments with consistent guarantees.  

---

#### Example 1: Token Transfer in Lumi

```lumi
// A natural number (no negatives)
type Nat = Int where n >= 0

// Resource type ensures ownership cannot be duplicated
resource Token { balance: Nat }

pure fn transfer(from: &mut Token, to: &mut Token, amount: Nat)
    require amount <= from.balance
    ensure from.balance' + amount == from.balance + to.balance
{
    from.balance = from.balance - amount
    to.balance   = to.balance + amount
}
```

- `require amount <= from.balance`: prevents overdraft.  
- `ensure` clause: enforces conservation of total supply at compile time.  
- `resource Token`: prevents duplication of balances (no double-spend).  
- The compiler and SMT solver together **prove the contract safe** before it can be deployed.  

---

#### Example 2: Preventing Reentrancy (DAO Hack)

**Solidity Vulnerable Code (simplified):**

```solidity
function withdraw(uint amount) public {
    require(balances[msg.sender] >= amount);
    (bool success, ) = msg.sender.call{value: amount}("");
    require(success);
    balances[msg.sender] -= amount;  // ⚠ State update happens AFTER external call
}
```

- **Bug**: attacker re-enters `withdraw` before `balances` is updated → drains funds.  

**Lumi Equivalent (safe by design):**

```lumi
mut fn withdraw(user: &mut Token, amount: Nat)
    require amount <= user.balance
    ensure user.balance' == user.balance - amount
{
    // Update state first
    user.balance = user.balance - amount

    // External call (I/O) is separated and sequenced after state mutation
    io fn sendFunds(to: Address, amount: Nat)
}
```

- **State update enforced before I/O**: effect system (`mut` vs `io`) prevents interleaving.  
- Compiler proves that `user.balance'` is always consistent.  
- Reentrancy exploit is **unrepresentable** in Lumi — the code won’t compile otherwise.  

---

### AI Instruction → Lumi Code Example (General App)

Because Lumi is **AI-first**, non-crypto workflows can also be generated safely from plain instructions.

**Instruction (natural language):**  
> “Compute the average of a list of integers. If the list is empty, don’t divide by zero; instead return nothing. Otherwise return an exact ratio equal to sum/length.”

**Generated Lumi code (self-provable):**

```lumi
type PosInt = Int where n > 0

struct Ratio { num: Int, den: PosInt }

// sum is a spec-level function; compiled code will use a loop with invariants
pure fn average(xs: Array[Int]) -> Option[Ratio]
    // No precondition: empty lists allowed
    ensure match result {
        None      => xs.length == 0,
        Some(r)   => r.den == xs.length && r.num == sum(xs)
    }
{
    if xs.length == 0 {
        None
    } else {
        Some(Ratio { num: sum(xs), den: xs.length })
    }
}
```

- **No divide-by-zero**: empty input returns `None` by construction.  
- **Self-provable contract**: the `ensure` clause guarantees the result equals the exact mathematical average (as a ratio), not a rounded value.  
- **AI-friendly**: the invariant is expressed declaratively; the compiler enforces it or rejects the code.

---

In short: **Lumi makes unsafe programs un-compilable**, shifting safety from “hope and audits” to **provable correctness by design**.

---

## 4. Vision

Lumi aims to be:
- **Simple like Python/Java/Javascript/Rust**, but **safe like SPARK Ada**.  
- **Portable** (runs anywhere via WASM).  
- **Proof-oriented** (bugs prevented at compile time).  
- **AI-first**: designed so that processes can be generated directly from instructions, while remaining  **self-provable**.  
- **Familiar syntax, user-friendly, and reliable by design**.  
- Suitable for **web apps, mobile apps, and smart contracts**.

---

## 5. Project Status

- Current focus: Phase 4.3–4.7 — Std Collections (type-only) and 4.8 — DX; next: Phase 5 — Codegen/Strings runtime.
- Full roadmap and checklist: see `docs/TODO.md`.
