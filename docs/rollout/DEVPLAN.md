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
- Update `lumi-cli build` to use typer output (IR) and call new encoder.
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

Out of Scope (moved to Phase 5)
- Any IR/codegen changes (type dedup, callee indices, memory/runtime).

---

# Phase 5 — Codegen IR→Wasm Plan (Types/Indices/Ops/Memory)

Goals
- Polish IR→Wasm codegen and introduce linear memory/runtime for Strings and List; prepare for Set/Map.

Work Items
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
