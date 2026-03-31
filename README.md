# Clear Language



## Introduction



Branding: **Clear Language** (project shorthand: **ClearLang**)  
Website: **https://clearlang.net**

**Clear Language** is a new programming language designed to make software - especially smart contracts - provably safe.  

Where today's languages leave room for bugs, hacks, and costly audits, ClearLang ensures that unsafe code simply won't compile.  

It combines **familiar syntax (Python/Java-like)** with **mathematical proofs of correctness**, hides complexity from the developer, and is built to be **AI-friendly**, so both humans and AI tools can generate secure, high-quality code more easily.  

Because it compiles to **WebAssembly (WASM)**, ClearLang runs anywhere - web, mobile, or blockchain - making it a practical foundation for trustworthy apps and next-generation crypto systems.



---

## Design Principles

- **Simple for users**: keep syntax familiar and minimize boilerplate so developers can be productive quickly.
- **AI-friendly**: predictable structure and diagnostics so tools can generate and repair code safely.
- **Provably correct**: contracts, types, and proofs make unsafe programs uncompilable.
- **Crypto-focused**: prioritize determinism, resource safety, and audit-grade guarantees for smart contracts.

---

## Separation of Concerns

- **Core language**: syntax, types, proofs, and effects (pure/mut/io).
- **Runtime**: deterministic host interface (storage, crypto syscalls, logging, gas, ABI entrypoints).
- **Chain packages**: chain-specific types and helpers (e.g., `std::eth::Address`) built on runtime capabilities.

This keeps the core small and stable while allowing chain packages to evolve as crypto tech adds new types or services.

Contract logic itself is modeled as a pure state transition (`apply(state, msg) -> Bytes`), while
the runtime handles ABI entrypoints and canonical serialization (see `docs/runtime/abi.md`).

---

## 1. Motivation & Concept



- The user wanted a **bug-proof language**, inspired by dependently-typed languages (Coq, Agda, Idris, F*).

- Goal: a language with **pure mathematical reasoning**, preventing entire classes of bugs at compile time.

- Proposed language name: **ClearLang** (light, clarity, purity).

- Another key goal: **simplicity for the end user**.  

  ClearLang should hide complex implementation details, leaving a clean, approachable syntax.  

  The learning curve should be **smooth and user-friendly**, so developers can be productive quickly without needing to understand deep theory upfront.

- **Familiar syntax**: ClearLang adopts a **Python/Java-like syntax**, making it intuitive for most developers to learn and use without friction.

- **AI-friendly design**: ClearLang is built with AI in mind. Its structure and clarity make it easier for AI tools to **generate complete processes directly from human instructions**.  

  Unlike today's languages, ClearLang code is **self-provable**: every generated function or contract can carry its own correctness guarantees, so incorrect or unsafe code cannot compile.



---



## 2. Core Ideas of ClearLang



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



## 3. Comparison & Crypto Contracts



ClearLang's motivation comes from gaps and pain points in today's languages:



### Dependently Typed Research Languages (Coq, Agda, Idris, F*)

- **Strengths**: allow rigorous mathematical proofs.  

- **Pain Points**: steep learning curve, complex syntax, mostly academic, hard to apply in production.  

- **ClearLang's Answer**: bring **dependent/refinement types** to a **Python/Java-like syntax**, making formal reasoning **accessible and user-friendly**, with AI support to guide proofs.



### SPARK Ada

- **Strengths**: proven correctness for mission-critical domains.  

- **Pain Points**: verbose syntax, niche ecosystem, slower onboarding for new developers.  

- **ClearLang's Answer**: keep the **rigor of contracts and proofs**, but hide complexity behind **simple, modern syntax** and automated solver integration.



### Rust

- **Strengths**: memory safety, fearless concurrency, great tooling.  

- **Pain Points**: does not address **business logic correctness** or domain invariants (e.g., balances never negative, workflows always terminate).  

- **ClearLang's Answer**: extend safety to **domain rules and contracts**, not just memory. Resource/linear types ensure **no double-spend**, making it ideal for crypto/finance.



### Elm

- **Strengths**: guarantees no runtime exceptions, great for UI.  

- **Pain Points**: limited in scope, not general-purpose, lacks formal proofs.  

- **ClearLang's Answer**: provide **totality and strong guarantees**, but keep the language **general-purpose** (apps, contracts, backend logic).



### Solidity / Vyper (Blockchain)

- **Strengths**: widely used for smart contracts.  

- **Pain Points**: easy to make critical errors (reentrancy, supply bugs, unchecked balances). Many exploits stem from missing formal guarantees.  

- **ClearLang's Answer**: enforce **compile-time proofs of invariants**, so contracts like token supply cannot compile if unsafe.



### Move (Meta's blockchain language)

- **Strengths**: resource types to prevent double-spending.  

- **Pain Points**: specialized domain, less emphasis on user simplicity or AI-assisted workflows.  

- **ClearLang's Answer**: combine **resource safety** with a **friendly syntax and AI support**, making high-assurance code easier to write and maintain.



---



### Quick Comparison Table



| Pain Point (Today)                | Example / Cause                                   | ClearLang Solution |

|----------------------------------|--------------------------------------------------|---------------|

| **Reentrancy exploits**           | DAO hack: state updated after external call      | Effect system forces state update before I/O; reentrancy unrepresentable |

| **Unchecked invariants**          | Balances can go negative; supply can inflate     | Refinement types + `require`/`ensure` enforce invariants at compile time |

| **Double spending**               | Assets cloned by mistake                         | Resource types guarantee single ownership, prevent duplication |

| **Infinite loops / non-termination** | Contracts hang, wasting gas or freezing logic | Totality check ensures loops must terminate or have invariants |

| **Complex unsafe syntax**         | Solidity low-level quirks, unsafe defaults       | ClearLang offers Python/Java-like syntax, AI-friendly and simple |

| **Audit dependency**              | Human review required, costly and fallible       | Mathematical proofs at compile time - unsafe code won't deploy |



---



### Why ClearLang is a Better Fit for Crypto Contracts



Smart contracts are unforgiving: a single bug can permanently lock or steal millions. Existing languages (Solidity, Vyper, even Move) leave room for errors because they rely heavily on tests and audits, not mathematical guarantees.



**Crypto pain points today**:

- Reentrancy exploits draining funds.  

- Invariants like *total supply* or *balance %Y 0* not enforced by compilers.  

- Double-spending or resource misuse due to unsafe abstractions.  

- Audits are expensive and still miss subtle bugs.  

- Developers face steep complexity or unsafe defaults.  



**How ClearLang addresses them**:

- **Dependent/refinement types** ensure balances and supplies always respect constraints.  

- **Contracts (`require`/`ensure`)** act as provable guardrails - unsafe code won't compile.  

- **Resource types** model tokens and assets directly, preventing double-spend by design.  

- **Totality guarantees** prevent non-terminating or unpredictable contract logic.  

- **AI-friendly syntax** lowers the barrier: both humans and AI can generate verifiable, safe contracts without requiring deep theorem-prover expertise.  

- **WASM portability** allows ClearLang contracts to run across multiple chains and environments with consistent guarantees.  



---



#### Example 1: Token Transfer in ClearLang



```clearlang

// A natural number (no negatives)

type Nat = Int where n >= 0;



// Resource type ensures ownership cannot be duplicated

resource Token {
    balance: Nat;
    drop { }
}



mut function transfer(from: Token, to: Token, amount: Nat) -> Nat
    require { amount <= token::balance(from) }
    ensure { result == old(token::balance(from) + token::balance(to)) }
{

    token::set_balance(from, token::balance(from) - amount);
    token::set_balance(to, token::balance(to) + amount);
    token::balance(from) + token::balance(to)

}

```



- `require { amount <= token::balance(from) }`: prevents overdraft.  

- `ensure { result == old(...) }`: enforces conservation of total supply; `old(...)` snapshots pre-state.  

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

    balances[msg.sender] -= amount;  // S  State update happens AFTER external call

}

```



- **Bug**: attacker re-enters `withdraw` before `balances` is updated +' drains funds.  



**ClearLang Equivalent (safe by design):**



```clearlang

io function withdraw(user: Token, to: std::eth::Address, amount: Nat) -> Nat
    require { amount <= token::balance(user) }
    ensure { result == old(token::balance(user)) - amount }
{

    // Update state first

    token::set_balance(user, token::balance(user) - amount);



    // External call (I/O) is separated and sequenced after state mutation
    // Chain packages provide `std::<chain>::io` helpers.
    std::eth::io::send_funds(to, amount);
    token::balance(user)

}

```



- **State update enforced before I/O**: effect system (`mut` vs `io`) prevents interleaving.  

- Compiler proves that the post-state balance matches the `ensure` contract.  

- Reentrancy exploit is **unrepresentable** in ClearLang - the code won't compile otherwise.  

- Note: chain packages (e.g., `std::eth::Address` and `std::eth::io`) are provided by the target environment, not the core language.



---



### AI Instruction +' ClearLang Code Example (General App)



Because ClearLang is **AI-first**, non-crypto workflows can also be generated safely from plain instructions.



**Instruction (natural language):**  

> "Compute the average of a list of integers. If the list is empty, don't divide by zero; instead return nothing. Otherwise return an exact ratio equal to sum/length."



**Generated ClearLang code (self-provable):**



```clearlang

type PosInt = Int where n > 0;



struct Ratio { num: Int, den: PosInt }



// sum is a spec-level function; compiled code will use a loop with invariants

pure function average(xs: Array[Int]) -> Option[Ratio]

    // No precondition: empty lists allowed

    ensure { match result {

        None      => xs.length == 0,

        Some(r)   => r.den == xs.length && r.num == sum(xs)

    } }

{

    if xs.length == 0 {

        None

    } else {

        Some(Ratio { num: sum(xs), den: xs.length })

    }

}

```

- Note: struct and Array[...] are planned core language features (see Phase 15).




- **No divide-by-zero**: empty input returns `None` by construction.  

- **Self-provable contract**: the `ensure` clause guarantees the result equals the exact mathematical average (as a ratio), not a rounded value.  

- **AI-friendly**: the invariant is expressed declaratively; the compiler enforces it or rejects the code.



---



In short: **ClearLang makes unsafe programs un-compilable**, shifting safety from "hope and audits" to **provable correctness by design**.



---



## 4. Vision



ClearLang aims to be:

- **Simple like Python/Java/Javascript/Rust**, but **safe like SPARK Ada**.  

- **Portable** (runs anywhere via WASM).  

- **Proof-oriented** (bugs prevented at compile time).  

- **AI-first**: designed so that processes can be generated directly from instructions, while remaining  **self-provable**.  

- **Familiar syntax, user-friendly, and reliable by design**.  

- Suitable for **web apps, mobile apps, and smart contracts**.



---



## 5. Project Status

- Current status: Phases `0` through `19` are complete in `docs/TODO.md` (including `19.5` assurance-manifest and release-policy gates).
- Next focus: define the post-19 roadmap slice (Phase 20+ planning and acceptance gates).
- Language scope note: several surfaces remain intentionally deferred/disallowed for v1 and are enforced with deterministic diagnostics (see `docs/TODO.md` and `docs/typing.md`).
- Full roadmap and checklist: see `docs/TODO.md` and the resource overview in `docs/resource-guide.md`.

---

## 6. Current Capabilities (Subset)

- Parsing: Int/Bool/String literals, namespaced calls, Option/Result with `match`, expression-form `if/else`, contract clauses, resource declarations with drop blocks, and consume params.

- Typing: Effect lattice (`pure`/`mut`), Option/Result pattern typing plus `if let`/`??`/postfix `?`, collection APIs with structured errors, span-rich diagnostics (Txxx codes), and linear/resource tracking (`T801`-`T804`) with collection rejection (`T806`).

- Codegen & runtime: IR->Wasm pipeline with string allocator/runtime traps (`R000`-`R002`), optional debug names, and `wasm-tools validate`.

- CLI & tooling: primary UX surface is `check`/`test`/`release` (Gate C lock), with advanced expert/debug commands `parse`/`build`/`run`/`verify` retained; supports `--json-errors`, `--emit-vcs`, and Wasmtime-backed `run`.

---

## 7. Signing and Verifying Proofs

ClearLang can embed proof metadata in a Wasm module and sign a canonical payload for offline verification.
For the end-to-end strict production command flow, see `docs/release-process.md`.
Gate C one-command release flow:
```
clg strict init examples/projects/generic
clg check examples/projects/generic/main.clear --root examples/projects/generic
clg release examples/projects/generic/main.clear \
  --key keys/signing.json --pubkey keys/public.json \
  --root examples/projects/generic
```
This executes `lock -> build/prove -> sign -> verify -> bundle` and emits
`<stem>.release-bundle.json` with deterministic artifact hashes and stage status.
`clg release` reads `advisory_as_of` and `key_id` defaults from `clg.project.json` (generated by `clg strict init`), with explicit override flags still available:

```
clg release examples/projects/generic/main.clear --advisory-as-of 2026-03-31T00:00:00Z \
  --key keys/signing.json --key-id release-2026q2 --pubkey keys/public.json \
  --root examples/projects/generic --out-dir out/release \
  --trust-policy examples/projects/generic/trust-policy.json
```

Assurance profile reality:
- `--compiler-mode permissive` and `--compiler-mode standard` are dev/evidence workflows and do **not** imply theorem-grade status (`proved_all`).
- Non-strict modes are transitional migration workflows and are not accepted for production release artifacts.
- Pre-production direction: remove compatibility debt rather than preserving legacy release paths once strict-first replacements exist.
- Theorem-grade/release-grade claims require strict release gates (`--release-profile production`) plus verification policy gates (`--require-assurance proved_all`).
- Production publish policy is `release == proved`; non-proved outputs remain non-release/dev artifacts.
- Milestone 3 intentionally has no `theorem` language keyword; theorem-grade is a certification outcome from build/verify policy gates.

Advanced expert/debug flows (legacy release-like path; CLI now emits migration guidance to `clg release`):

Build and sign:
```
clg build examples/contract.clear -o out.wasm --emit-vcs out.vc.json \
  --sign --key keys/signing.json --key-id demo --scope both --sig-out out.sig.json
```
This also emits a signed assurance manifest by default at `out.assurance.json`
(derived from `--sig-out`). Override with `--assurance-manifest-out <FILE>`.

Build and sign with pinned compile-time trust anchors:
```
clg build examples/contract.clear -o out.wasm --emit-vcs out.vc.json \
  --sign --key keys/signing.json --key-id demo --scope both --sig-out out.sig.json \
  --lean-checker-version 4.14.0 --coq-checker-version 8.19.2
```

Verify a signed module:
```
clg verify --module out.wasm --sig out.sig.json --pubkey keys/public.json
```

Explain verification assurance summary (what is checked vs assumed, and why):
```
clg verify --module out.wasm --sig out.sig.json --pubkey keys/public.json --explain
```

Release policy gate using signed assurance manifest:
```
clg verify --module out.wasm --sig out.sig.json --pubkey keys/public.json \
  --assurance-manifest out.assurance.json --release-policy release-policy.json
```

Compile-time verify mode with trust policy:
```
clg verify --module out.wasm --sig out.sig.json --pubkey keys/public.json \
  --verify-mode compile-time --trust-policy trust-policy.json
```

Trust policy JSON shape:
```
{
  "schema_version": 1,
  "trust_anchors": {
    "lean_checker": "4.14.0",
    "coq_checker": "8.19.2"
  }
}
```

Release policy JSON shape:
```
{
  "schema_version": 1,
  "minimum_assurance_tier": "L1"
}
```

Key files are JSON:
```
{"scheme":"ed25519","private_key":"<hex>","public_key":"<hex>"}
{"scheme":"ed25519","public_key":"<hex>"}
```
