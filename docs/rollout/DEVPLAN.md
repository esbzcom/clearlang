# Phase 3.5 -" Integration Plan (IR+'Wasm)



Goals

- Wire CLI `build` to: parse +' type-check +' lower to IR +' codegen Wasm.

- Adapt codegen to accept `clg_ir::Module` instead of AST.

- Add `--validate` flag to run `wasm-tools validate` on outputs.



IR+'Wasm Mapping

- Functions: assign indices; export `main` when present.

- Types: `IrType::Int|Bool` +' Wasm `i32`; Bool uses 0/1.

- Params/locals: IR params map to Wasm params; allocate locals for SSA temps.

- Instrs: `IConst(n) +' i32.const`; `IBin(Add|Sub|Mul|Div) +' i32 ops`; `Call +' call` by index; `Ret +' return`.



CLI Changes

- Replace current AST codegen call with IR pipeline; print simple stage logs.

- `--validate`: if set, shell out to `wasm-tools validate` (optional in CI).



Testing

- Add e2e for samples: `01_hello`, `02_arith`, `03_nested_calls`, `04_multiline_call`, `05_trailing_param_comma`.

- Keep negative parse case `06_trailing_call_comma`.



Next Actions

- [done] Implement IR+'Wasm encoder entry accepting `clg_ir::Module`.

- [done] Update `clg build` to use typer output (IR) and call new encoder.

- [done] Add `--validate` and gate stage logs behind `--verbose`.



---



# Next Steps (Post 3.5)



Testing (Phase 3.6)

- Add IR pipeline tests for samples: `02_arith`, `03_nested_calls`, `04_multiline_call`, `05_trailing_param_comma`.

- Keep `06_trailing_call_comma` as a negative parse case (no IR/codegen).



Diagnostics & Spans (Phase 3.8)

- Attach source spans in AST (identifiers, expressions) using chumsky `map_with_span`.

- Propagate spans into typer errors: unknown var/fn, arity, return/type mismatch.

- Add tests asserting span presence/format in error messages.



---



# Phase 3.8 -" Diagnostics & Spans Plan



Goals

- Add basic source spans for identifiers and expressions in AST and surface them in type errors.



Parser

- Update AST nodes to carry spans where useful (Expr variants, function/param identifiers).

- Use chumsky `map_with_span` to capture spans during parsing.



Typer

- Extend error paths to include span data; adjust error messages to include `line:col` when available.

- Keep rules as-is; only enrich diagnostics.



Tests

- Add negative typer tests that assert span presence (not exact numbers yet, but format and non-empty).

- Include cases: unknown var, unknown function, arity mismatch, return mismatch, binop operand type mismatch.



CLI

- Optionally print spans in errors (e.g., `in file:line:col: message`).



---



# Phase 4 -" Namespacing + Strings (Parse/Type) + Std Collections stubs + Small DX



Goals

- Introduce namespaced call syntax (std::...) to avoid global prefixes without a full module system.

- Add String literal parsing (incl. multi-line) and basic `String` typing; no runtime yet.

- Provide List/Set/Map signatures (type-checking only) to unblock user code; defer runtime to Phase 5.

- Apply small DX optimizations that don't change semantics.



Work Items

- Namespacing: parser/typer accept path calls: `std::str::len`, `std::list::push`, `std::map::get`, `std::set::contains`.

- Strings: `Type::String`, `Expr::String` with escapes and multi-line; typer rules for eq/concat in `std::str`; tests (escapes/spans).

- Collections: define `List<T>`, `Set<T>`, `Map<K,V>`, `Option<T>`, `Result<T,E>`; expose minimal `std::list`, `std::set`, `std::map` APIs to typer.

- DX: preallocation in parser/typer; shared Wasmtime Engine in tests; release profile tuning; verbose-gated logs.

- DX (parser split): extract `tokens.rs`, `types.rs`, `literals.rs`, `path.rs`, `expr.rs`, `func.rs`, `program.rs`; wire via `lib.rs` (do before/with Strings).

   - Status: completed.

- DX (typer split): separate typing rules (`check.rs`) from IR lowering (`lower.rs`) to prep for `String` and collections.

 - DX (CLI refactor): when adding `--verbose`, split subcommands into `commands/{emit_hello,parse,build,run}.rs` and small helpers.



Out of Scope (moved to Phase 5)

- Any IR/codegen changes (type dedup, callee indices, memory/runtime).



---



# Phase 5 -" Codegen IR+'Wasm Plan (Types/Indices/Ops/Memory)



Goals

- Polish IR+'Wasm codegen and introduce linear memory/runtime for Strings and List; prepare for Set/Map.



Work Items

- [done] DX (codegen split): organize into `trivial.rs` (emit_trivial_main), `intrinsics/strings.rs` (string intrinsics), and `ir.rs` (IR encoder); re-export in `lib.rs`.

- [done] Module & signatures: deduplicate function type signatures in Wasm Type section.

- [done] Calls: switch to callee indices (resolve names during lowering); remove name+'index lookups in codegen.

- [done] Strings runtime basics: linear memory, string data segments, and global `heap_ptr` (bump allocator).

- [done] Intrinsics: `std::str::{len, eq, concat}` (robust eq; concat via memory.copy); tests added.

- Ops: continue `IConst`, `IBin`, `Call`, `Ret`; add void-return and drop unused call results where applicable.

- Validation: maintain `--validate`; consider CI integration.



Tests

- E2E IR/codegen tests covering strings len/eq/concat and samples; CLI smoke remains green.



---



# Phase 4.1 -" Namespacing Progress



Status

- [x] Parser: accept namespaced call syntax `seg::seg::name(args...)`.

- [ ] Typer: wire namespaced calls to built-in signatures (tracked under Phase 4.3).



Notes

- Variables remain simple identifiers (no `::`).

- Bare paths like `a::b` without `(...)` are rejected (call syntax only).



---



# Phase 4.2 -" Strings Progress



Status

- [x] AST/Parser: `Type::String` and `Expr::String` with escapes and multi-line literals.

- [x] Typer: `String` is first-class (params/returns, literals type to `String`).

- [x] Built-ins: `std::str::{len, concat, eq}` (type stubs) wired in typer.

- [x] Tests: parser (escapes, multi-line, invalid escape) and typer (String echo, spanful mismatch).

- [x] Syntax cleanup: `function` keyword only (removed `fn`).

- [x] Parser UX: hint when `:` is used for return types (suggest `->`).

- [x] CLI/Diagnostics: add `--json-errors` with stable codes (P001, T001-"T006, C001-"C002) and spans.

- [x] Structured errors: introduce `ParserError` and `TyperError` with codes/spans; CLI emits JSON directly.

- [x] DX: Split `clg-typer` into modules (`errors`, `builtins`, `check`, `lower`) keeping `check()` public.



Notes

- Codegen/runtime for `String` deferred to Phase 5; lowering uses a placeholder.



---



# Phase 4.3 -" Collections (Type Stubs) Plan



Goals

- Prepare collections in the typer while keeping the language simple; avoid committing to generics prematurely.



Scope (type-only)

- Design minimal signatures for `std::list`, `std::set`, `std::map` to enable type-checking in examples.

- Defer `Option<T>`/`Result<T,E>` and full generics until ADTs + `match` land (simplicity over partial features).



Work Items

- Draft a brief design note for parametric types and `match` (timing and shape).

- Option A (strict): keep collections deferred; add friendly error stubs explaining "collections require generics; planned in Phase X".

- Option B (demo-only): add monomorphic preview signatures (e.g., `std::str::split(String) -> ListString`) for early demos; clearly marked temporary.

- Add typer tests validating unknown-collection calls produce helpful errors (if Option A).



---



# Phase 6 -" Contracts, Effects (Proof-Ready) -" DEVPLAN Slice



Goals

- Introduce a minimal contract system to enable machine-checked mathematical proofs of simple properties for pure functions.

- Generate verification conditions (VCs) and optionally emit proof artifacts suitable for external proof checkers.

- Keep the design simple, testable, and AI-friendly.



Syntax (initial)

- Function contracts attach to definitions; only `pure` functions participate initially.

- Grammar sketch (single-expression bodies for now):



  ```

  pure function inc(x: Int) -> Int

    require { x >= 0 }

    ensure  { result >= x }

  {

    x + 1

  }

  ```



  - `require { expr }`: Boolean precondition over params.

  - `ensure { expr }`: Boolean postcondition; `result` names the return value.

  - Multiple `require`/`ensure` blocks are allowed; they are conjoined.

  - Non-`pure` effects: contracts parsed but not used for proofs in Phase 6 (emit a clear diagnostic if attempted).



Initial VC Rules (pure, expression-bodied)

- For `function f(p) -> r { e }` with pre P and post Q:

  - VC: P => Q[result := e]. For multiple requires/ensures, conjoin respectively.

- For calls `g(a)` inside `e` (Phase 6 limited case):

  - Obligation: current context must imply `Pg[a/params]` (callee pre). Do not yet inline/post-strengthen with `Qg` (defer to later slice).

- Types supported in VCs: Int, Bool; operators: +, -, *, /, comparisons, equality, Boolean ops.

- Logic fragment: quantifier-free linear integer arithmetic (QF_LIA) + Bool.



CLI Changes

- `clg build <file> [--emit-vcs <OUT>] [--emit-proof <OUT>]`:

  - `--emit-vcs`: writes a JSON array of VCs with stable schema:

    - `{ function, pre, post, vc_id, smt2, status }`

    - `status`: "generated"|"proved"|"failed` (if solver integrated later)

  - `--emit-proof`: when solver integration is enabled later, also write a proof certificate (e.g., Alethe) per `vc_id` next to OUT.

- Default: do not solve; only generate VCs and write them if `--emit-vcs` is provided.



Proof-Carrying Wasm (PCW) Section Layout (reserved)

- Custom section name: `clearlang.proof` (versioned):

  - `version`: u32 (start at 1)

  - `functions`: [

      { `name`, `pre` (AST string), `post` (AST string), `vcs`: [ { `vc_id`, `smt2` } ], `proofs`: optional [ { `vc_id`, `format`, `bytes` } ] }

    ]

  - `generated_by`: tool/version metadata

- In Phase 6, optionally embed VC texts (no proofs) when `--emit-vcs` is used with `--debug-names`.



Testing & Acceptance Criteria

- Parser: contracts parse; errors on malformed `require/ensure` are spanful.

- Typer: `pure` contract-bearing functions type-check; non-pure emit a clear message that proof is limited to pure in Phase 6.

- VC Gen: for expression-bodied pure functions, `--emit-vcs` outputs one VC per function; content matches P => Q[e/result].

- Snapshot tests: JSON schema validated; examples for inc/add and a failing ensure.



Out of Scope (Phase 6 follow-ups)

- Loops/arrays/invariants, effectful VCs, interprocedural postcondition propagation, solver integration and proof re-checker.



---



# Phase 6 -" Proof Signatures & Anchoring (Design)



Principles

- Simple Is Best: start with offline, file-level signatures; defer blockchain anchoring.

- Prove Correct: signatures cover the exact bytes of VCs/proofs and the Wasm module; verification re-checks proofs, then signatures.

- AI-Friendly: canonical JSON for payloads, stable schema and codes.



Signing Scope & Payloads

- Scopes: `proofs` (all VC+proof artifacts), `module` (Wasm bytes), or `both`.

- Canonical payload (JSON, JCS canonicalization):

  - `{ module_hash, proofs_hash, toolchain, timestamp, scope }`

  - Hashes: SHA-256 over exact byte sequences; `module_hash` over `.wasm` bytes; `proofs_hash` over concatenated VC/proof artifacts in lexicographic `vc_id` order.

- Signature object:

  - `{ alg: "Ed25519", key_id, payload_hash, sig, signer_meta }`.



CLI Additions

- Build/sign:

  - `clg build file.clear --emit-vcs vcs.json [--emit-proof proofs/] --sign --key key.pem --key-id KEY --sign-scope proofs|module|both --sig-out sig.json`

- Verify:

  - `clg verify out.wasm --sig sig.json --pubkey pub.pem [--vcs vcs.json] [--proofs proofs/]`

  - Steps: validate Wasm +' optionally re-check proofs +' verify signature over declared scope.



PCW Section (extend)

- `clearlang.proof` v1 adds `signatures: [{ alg, key_id, payload_hash, sig, scope }]` and `module_hash` fields.

- Keep embeddings optional and gated behind flags; default remains off.



Anchoring (Optional, later)

- Registry: post `sha256(signature_object)` to an on-chain registry (EVM suggested) with a URI to artifacts (IPFS/https). Not in Phase 6 scope.



Acceptance

- Deterministic `module_hash` reproducible across machines.

- `clg verify` validates signature and (when artifacts are provided) re-checks proofs before accepting.



---



# Near-Term Next Steps (Post 6.5)

## Focus 1 – Phase 6.6 ADT Ergonomics

- Draft a design note covering `if let`, `??`, and `?`, including effect-awareness and proof obligations.
- Implement `if let` by desugaring to two-arm `match`; update parser/typer/tests and reuse existing diagnostics.
- Prototype `??` (Option coalesce) and `?` (Option/Result propagation) behind an experimental flag with typer checks enforcing purity/effect compatibility.
- Document representative samples and the VCs they emit to keep outputs AI-friendly.

## Focus 2 – Phase 7 Resource / Linear Types

- Stabilize syntax for `resource` types and enforce move-only semantics in the typer.
- Extend lowering/runtime with explicit drop/move tracking to prevent double-use.
- Add tests and docs that explain how resource types block aliasing and double-spend patterns.

## Focus 3 – Phase 8 Totality & Loops

- Introduce loop invariants/variants syntax and ensure typer validation for `while` constructs.
- Enforce totality checks for `pure` functions (structural recursion or decreasing measures).
- Capture diagnostic strategy and examples in `docs/typing.md`; add positive/negative regression tests.

## Focus 4 – Safety & DX Hardening (Phases 11–12)

- Wire `wasm-tools validate` and full workspace `cargo test` into CI; document Wasmtime fuel/epoch limits.
- Design the `--contracts=runtime|hybrid|static` flag surface and experiment with fuel/epoch limits in integration tests.
- Introduce CLI tracing hooks and a `cargo xtask` (or Makefile) for repeatable workflows.
- Unify error types/messages across crates and tune release profile defaults (`lto=thin`, `codegen-units=1`).



---



# Phase DEVPLAN -" Next Steps (2025-09-14)



Scope

- Consolidate immediate work: 5.0 Codegen Layout & DX, 5.1-"5.5 IR +' Wasm, 5.6 Strings runtime.



4.7 Collections Docs/DX

- Docs: document naming (modules lower-case: `std::list`; types PascalCase: `List<T>`), dual-API guidance (precondition vs `Option`/`Result`), and stable error codes T206 "new requires inference" and T207 "expected collection kind".

- DX: confirm `clg-typer` split (`errors`, `builtins`, `check`, `lower`); keep `lib.rs` re-exports minimal and stable.

- Deliverables: update `docs/typing.md`; consider adding `docs/collections.md` with API/diagnostics overview.



4.10 Conditionals (expr-form if/else)

- Status: Implemented. Parser requires final `else` (no `elif` alias). Typer enforces `Bool` cond and branch unification; T301 added.



Phase 5 IR +' Wasm

- 5.0 Codegen Layout & DX

  - Split `codegen-wasm` into `trivial.rs` (emit_trivial_main), `ir.rs` (IR+'Wasm encoder), and `intrinsics/strings.rs` (string intrinsics); re-export from `lib.rs`.

  - Add `--verbose` and gate stage logs; refactor CLI into `commands/{emit_hello,parse,build,run}.rs` with small helpers.

- 5.1 Types/Indices/Exports: define module types, function indices; export `main` if present; deduplicate function signatures.

- 5.2 Locals & Stack: allocate locals for SSA temps; map IR values to stack ops.

- 5.3 Ops & Calls: encode `IConst`, `IBin`, `Call`, `Ret`; switch calls to callee indices.

- 5.4 Replace Const-Eval: make IR+'Wasm default; keep const-eval as an optional flag initially.

- 5.5 Tests: extend e2e; run `wasm-tools validate` and Wasmtime execution; reuse a shared Wasmtime Engine.

- Proof: document small-step simulation for arithmetic/call subset.



5.6 Strings Runtime

- Model: `String = (ptr:i32, len:i32)` UTF-'8; immutability invariant.

- Intrinsics: `std::str::{len, concat, eq}`; define traps and error codes R001 (OOM), R002 (InvalidUtf8, if relevant).

- Allocator: simple bump allocator; monotonic bump invariant.

- Tests: e2e string ops; property tests (concat length, eq properties).

 - Docs: ensure `docs/runtime/strings.md` captures invariants and memory model.



Provability (subset)

- Document a value-preservation proof sketch for the Int/Bool arithmetic and direct call subset: interpreter vs generated Wasm yield identical results.

- Keep the proof AI-friendly: small-step rules, explicit assumptions, and cross-references to tests.



4.8 Optimizations & DX

- Preallocate parser/typer maps/vecs; reuse shared Wasmtime `Engine`; add `[profile.release]` tuning; micro-benchmarks.



Cross-Cutting (Provable + AI-'Friendly)

- Maintain stable JSON diagnostics with codes and spans.

- For each feature, update `docs/typing.md` with formal rules and a brief preservation/progress sketch; link to test IDs.



Prep -" Blocks & Early Return (Post 4.10)

- Design multi-statement blocks: `let`, expr statements, `return;`.

- IR additions for basic structured control flow (`If/Else`, block join via locals).

- Gradually enable early returns inside blocks; keep expression-form compatibility.



---



# Phase DEVPLAN -" Next Steps (2025-09-17)



Focus -" Phase 6 (Contracts & Effects)

- Syntax: parse `pure`, `require { expr }`, `ensure { expr }` (expression-bodied functions first); attach spans.

- VC Gen: for each pure expression-bodied function with pre P and post Q, generate one VC `P #' Q[e/result]` in QF_LIA.

- CLI: `clg build file.clear --emit-vcs out.json` writes an array matching `docs/proofs/vc-schema.md` with `status: "generated"`.

- Tests: snapshot the JSON; include a failing ensure example and a simple inc/add.



Runtime & Diagnostics

- Strings runtime: add explicit OOM guard in bump allocator and map to R001; document R002 (InvalidUtf8) behavior.

- Diagnostics: add unit tests for `C001` (invalid main signature) and `P010` (missing else) to keep codes/doc synced.



Proofs

- Extend value-preservation proof to cover expression-form `if/else` (ISelect lowering to typed Wasm if/else with result).



Design

- Draft blocks + early return design slice (multi-statement blocks, `let`, `return;`), compatible with current expression form.

 - Schedule ADT ergonomics as 6.5: `if let` sugar (Some/Ok) via desugar to 2-arm match; `??` and `?` after effects/contracts, behind a flag.



# Phase DEVPLAN - Next Steps (2025-09-18)



Focus - Contracts Runtime & Effects

- Centralize runtime trap metadata writes so new intrinsics stay in sync with the guard proof story.

- Keep CLI help and docs aligned on `R000`-`R002` semantics; extend with an explicit invalid UTF-8 sample.

- Phase 6.4: introduce `Effect::Mut`, annotate `*_mut` builtins, and surface deterministic `T401` with JSON/span coverage.

- Update `docs/typing.md` and `docs/collections.md` with the new effect table to stay AI-friendly and machine-checkable.

- Add a regression test covering an `R002` invalid UTF-8 failure to complement the existing `R000`/`R001` cases.



# Phase DEVPLAN - Next Steps (2025-09-19)
Status (2025-09-20)
- std::list/set/map::*_mut now require mut effect plus a matching std::<collection>::can_mut guard (T401/T402/T403).
- VC generator emits mut_pre:* obligations so proof packaging can reference guard predicates deterministically.
- Guard predicates are documented as pure contracts; runtime lowering keeps them stubbed until the mutable runtime is implemented.

Next Steps
- Draft the doc updates for docs/typing.md/docs/collections.md (effect lattice table, guard examples, SMT snippets).
- Plan the runtime story for *_mut (either traps or in-place updates) before enabling build outputs that rely on mutable collections.
- Wire guard predicates into future proof packaging so mut_pre VCs link to the guard clause by stable ID.



Focus - Phase 6.4 Mutable Effects and Proof Packaging Prep

- Status (2025-09-19): effect lattice hardened in typer; T401 regression tests cover mut builtins.

- Harden the effect lattice by defining Effect::{Pure,Mut,Io} in the typer, threading it through lowering, and refusing silent downgrades across calls and branches.

- Implement builtin signatures for std::list/set/map::*_mut with precise pre/post conditions so diagnostics stay deterministic (T401) and AI-friendly.

- Extend contract VC generation to require matching require guards before *_mut calls; emit stable vc_id suffixes like mut_pre to unblock proof packaging.



Proof Obligations - Mathematically Provable & Prove Correct

- Frame an effect soundness lemma stating pure functions cannot reach *_mut intrinsics; cross-link the lemma to typer rules and guard tests.

- Update docs/typing.md and docs/collections.md with the effect lattice table, proof sketch, and SMT-friendly worked examples.

- Add regression tests covering accepted mut functions, rejected pure callers, and VC JSON snapshots to keep outputs AI-friendly.



Runway for 6.5 Proof Packaging

- Draft docs/proofs/proof-section.md with header fields, VC linkage, and hashing notes so implementation stays proof-driven.

- Spike a serialization helper in codegen-wasm behind #[cfg(test)] to verify canonical ordering ahead of the CLI flag.

- Capture open questions around key storage and CLI UX to guide the --sign workflow without blocking effect work.

## Phase 6.6 - ADT Ergonomics (Sugar)

Status (2025-09-20)
- Parser surface accepts `if let`, `??`, and `?` behind the Phase 6 experimental flag.
- Typer enforces Option/Result flows, threads `$return`, and emits new diagnostics (T601-T613).
- Parser/typer suites guard positive/negative cases; docs/typing.md documents the feature.

Next Steps
- Draft the design note covering desugars, VC expectations, and remaining debts.
- Add VC snapshot coverage before graduating the sugar from experimental status.
- Implement lowering/codegen support for `Expr::Try` and record the experimental flag policy.
- Extend docs with worked desugar examples for `if let`, `??`, and `?`.
