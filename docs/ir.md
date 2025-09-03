# Lumi IR (Phase 3.3–3.5)

Overview
- Minimal SSA-like IR used between the typer and Wasm codegen.
- Single block per function in Phase 3.x; control flow is linear.

Core Types
- `IrType`: `Int`, `Bool` (both lower to Wasm `i32`; Bool uses 0/1).
- `Value(u32)`: SSA value id.

Instructions
- `IConst { dst: Value, ty: IrType, n: i64 }` — integer/boolean constant (Bool as 0/1).
- `IBin { dst: Value, op: Add|Sub|Mul|Div, lhs: Value, rhs: Value }` — integer binary op.
- `Call { dst: Option<Value>, callee: String, args: Vec<Value> }` — function call (currently returns a value; `dst: Some(v)`).
- `Ret { val: Value }` — function return.

Containers
- `Function { name: String, params: Vec<IrType>, ret: Option<IrType>, body: Vec<Instr> }`.
- `Module { funcs: Vec<Function> }`.

SSA / Value Assignment
- Parameter values occupy `Value(0..P-1)` in parameter order.
- New temporaries allocate monotonically increasing ids starting at `P`.

Lowering (AST → IR)
- Literals: `Int(n)` → `IConst(Int, n)`; `Bool(b)` → `IConst(Bool, 1 or 0)`.
- Vars: read the parameter/temporary id from the local map.
- Binops: lower both sides, then `IBin(op, lhs, rhs)` to a fresh `dst`.
- Calls: lower arg values, emit `Call { dst: Some(fresh), callee, args }`.
- Return: emit a final `Ret { val }` with the last computed value.

IR → Wasm (Phase 3.5 subset)
- Parameters map to Wasm params; temporaries become locals (one i32 local per used SSA beyond params).
- Encoding:
  - `IConst` → `i32.const n; local.set v`
  - `IBin(Add|Sub|Mul|Div)` → `local.get lhs; local.get rhs; i32.{add|sub|mul|div_s}; local.set dst`
  - `Call` → `local.get args*; call idx; local.set dst?`
  - `Ret` → `local.get v` (implicit function return at end)
- Exports: `main` is exported when present.

Planned Extensions
- Void-returning functions, dropping unused call results.
- Multi-block/control flow in later phases.

