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

# Phase 3.9 — Polish & Low‑impact Optimizations

Goals
- Improve performance and ergonomics without changing language surface.

Codegen (IR → Wasm)
- Deduplicate function signatures in the type section (build a map of (params, ret) → type index).
- Switch call encoding to use callee indices from IR to avoid name→index lookups during encoding.
- Add optional name section emission (done) controlled by a CLI flag (`--debug-names`).

IR
- Change `Instr::Call { callee: String }` to use `callee: u32` (function index) once a stable index ordering is established.
- Provide a lowering pass that resolves names to indices based on the module’s function table.

Parser/Typer micro‑polish
- Preallocate HashMaps/Vecs using known capacities (params.len(), funcs.len()).
- Simplify identifier construction with direct `collect::<String>()` where possible.

Tests & CI
- Reuse a shared Wasmtime `Engine` in tests via `once_cell` to speed up module compilation.
- Keep IR pipeline and CLI smoke tests; expand only as needed.

Build Profiles & DX
- Add `[profile.release]` tuning (e.g., `lto = "thin"`, `codegen-units = 1`).
- Gate build stage logs behind `--verbose` to reduce default noise.


Codegen Extensions (Phase 4)
- Functions & exports: function index mapping; export `main` (done), extend for more exports.
- Locals/stack: refine local allocation strategy if/when multi-block IR arrives.
- Ops: maintain `IConst`, `IBin`, `Call`, `Ret`; add void-return support and drop unused call results.
- Validation: keep `--validate` path; consider adding CI validation step.
