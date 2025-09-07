# Phase 3.5 — Integration Plan (IR→Wasm)

Goals
- Wire CLI `build` to: parse → type-check → lower to IR → codegen Wasm.
- Adapt codegen to accept `lumi_ir::Module` instead of AST.
- Add `--validate` flag to run `wasm-tools validate` on outputs.

IR→Wasm Mapping
- Functions: assign indices; export `main` when present.
- Types: `IrType::Int|Bool` → Wasm `i32`; Bool uses 0/1.
- Params/locals: IR params map to Wasm params; allocate locals for SSA temps.
- Instrs: `IConst(n) → i32.const`; `IBin(Add|Sub|Mul|Div) → i32 ops`; `Call → call` by index; `Ret → return`.

CLI Changes
- Replace current AST codegen call with IR pipeline; print simple stage logs.
- `--validate`: if set, shell out to `wasm-tools validate` (optional in CI).

Testing
- Add e2e for samples: `01_hello`, `02_arith`, `03_nested_calls`, `04_multiline_call`, `05_trailing_param_comma`.
- Keep negative parse case `06_trailing_call_comma`.

Next Actions
- Implement IR→Wasm encoder entry accepting `lumi_ir::Module`.
- Update `lumi build` to use typer output (IR) and call new encoder.
- Add `--validate` and basic logging.

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

# Phase 3.8 — Diagnostics & Spans Plan

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

# Phase 4 — Namespacing + Strings (Parse/Type) + Std Collections stubs + Small DX

Goals
- Introduce namespaced call syntax (std::…) to avoid global prefixes without a full module system.
- Add String literal parsing (incl. multi-line) and basic `Str` typing; no runtime yet.
- Provide List/Set/Map signatures (type-checking only) to unblock user code; defer runtime to Phase 5.
- Apply small DX optimizations that don’t change semantics.

Work Items
- Namespacing: parser/typer accept path calls: `std::str::len`, `std::list::push`, `std::map::get`, `std::set::contains`.
- Strings: `Type::Str`, `Expr::Str` with escapes and multi-line; typer rules for eq/concat in `std::str`; tests (escapes/spans).
- Collections: define `List<T>`, `Set<T>`, `Map<K,V>`, `Option<T>`, `Result<T,E>`; expose minimal `std::list`, `std::set`, `std::map` APIs to typer.
- DX: preallocation in parser/typer; shared Wasmtime Engine in tests; release profile tuning; verbose-gated logs.
- DX (parser split): extract `tokens.rs`, `types.rs`, `literals.rs`, `path.rs`, `expr.rs`, `func.rs`, `program.rs`; wire via `lib.rs` (do before/with Strings).
   - Status: completed.
 - DX (typer split): separate typing rules (`check.rs`) from IR lowering (`lower.rs`) to prep for `Str` and collections.
 - DX (CLI refactor): when adding `--verbose`, split subcommands into `commands/{emit_hello,parse,build,run}.rs` and small helpers.

Out of Scope (moved to Phase 5)
- Any IR/codegen changes (type dedup, callee indices, memory/runtime).

---

# Phase 5 — Codegen IR→Wasm Plan (Types/Indices/Ops/Memory)

Goals
- Polish IR→Wasm codegen and introduce linear memory/runtime for Strings and List; prepare for Set/Map.

Work Items
- DX (codegen split): organize into `trivial.rs` (emit_trivial_main), `const_eval.rs` (emit_from_ast + evaluator), and `ir.rs` (IR encoder); re-export in `lib.rs`.
- Module & signatures: deduplicate function type signatures in Wasm Type section.
- Calls: switch to callee indices (resolve names during lowering); remove name→index lookups in codegen.
- Memory/runtime: add a minimal allocator (bump/realloc). Strings as (ptr,len) with data segments; List<T> with grow/realloc.
- Ops: continue `IConst`, `IBin`, `Call`, `Ret`; add void-return and drop unused call results where applicable.
- Validation: maintain `--validate`; consider CI integration.

Tests
- E2E IR/codegen tests covering added memory/runtime behaviors; CLI smoke remains green.

---

# Phase 4.1 — Namespacing Progress

Status
- [x] Parser: accept namespaced call syntax `seg::seg::name(args...)`.
- [ ] Typer: wire namespaced calls to built-in signatures (tracked under Phase 4.3).

Notes
- Variables remain simple identifiers (no `::`).
- Bare paths like `a::b` without `(...)` are rejected (call syntax only).

---

# Phase 4.2 — Strings Progress

Status
- [x] AST/Parser: `Type::Str` and `Expr::Str` with escapes and multi-line literals.
- [x] Typer: `Str` is first-class (params/returns, literals type to `Str`).
- [x] Built-ins: `std::str::{len, concat, eq}` (type stubs) wired in typer.
- [x] Tests: parser (escapes, multi-line, invalid escape) and typer (Str echo, spanful mismatch).
- [x] Syntax cleanup: `function` keyword only (removed `fn`).
- [x] Parser UX: hint when `:` is used for return types (suggest `->`).

Notes
- Codegen/runtime for `Str` deferred to Phase 5; lowering uses a placeholder.

---

# Phase 4.3 — Collections (Type Stubs) Plan

Goals
- Prepare collections in the typer while keeping the language simple; avoid committing to generics prematurely.

Scope (type-only)
- Design minimal signatures for `std::list`, `std::set`, `std::map` to enable type-checking in examples.
- Defer `Option<T>`/`Result<T,E>` and full generics until ADTs + `match` land (simplicity over partial features).

Work Items
- Draft a brief design note for parametric types and `match` (timing and shape).
- Option A (strict): keep collections deferred; add friendly error stubs explaining “collections require generics; planned in Phase X”.
- Option B (demo-only): add monomorphic preview signatures (e.g., `std::str::split(Str) -> ListStr`) for early demos; clearly marked temporary.
- Add typer tests validating unknown-collection calls produce helpful errors (if Option A).

---

# Phase 6 — Contracts, Effects (Proof-Ready) — DEVPLAN Slice

Goals
- Introduce a minimal contract system to enable machine-checked mathematical proofs of simple properties for pure functions.
- Generate verification conditions (VCs) and optionally emit proof artifacts suitable for external proof checkers.
- Keep the design simple, testable, and AI-friendly.

Syntax (initial)
- Function contracts attach to definitions; only `pure` functions participate initially.
- Grammar sketch (single-expression bodies for now):

  ```
  function inc(x: Int) -> Int pure
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
- `lumi build <file> [--emit-vcs <OUT>] [--emit-proof <OUT>]`:
  - `--emit-vcs`: writes a JSON array of VCs with stable schema:
    - `{ function, pre, post, vc_id, smt2, status }`
    - `status`: "generated"|"proved"|"failed` (if solver integrated later)
  - `--emit-proof`: when solver integration is enabled later, also write a proof certificate (e.g., Alethe) per `vc_id` next to OUT.
- Default: do not solve; only generate VCs and write them if `--emit-vcs` is provided.

Proof-Carrying Wasm (PCW) Section Layout (reserved)
- Custom section name: `lumi.proof` (versioned):
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
