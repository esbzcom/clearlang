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
