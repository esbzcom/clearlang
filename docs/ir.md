# ClearLang IR (Phase 3.3–3.5)

Overview
- Minimal SSA-like IR used between the typer and Wasm codegen.
- Single block per function in Phase 3.x; control flow is linear.
 - Deterministic mapping to Wasm enables translation validation and future proofs.

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

Verification Notes
- Instruction-by-instruction simulation: each IR op corresponds to a small Wasm sequence.
- Translation validation: run an IR interpreter and the Wasm instance on the same inputs to check for equal results (testing aid ahead of full proofs).

Planned Extensions
- Void-returning functions, dropping unused call results.
- Multi-block/control flow in later phases.

Match Lowering Plan (Phase 5+)
- Goal: Support `match` over `Option`/`Result` after introducing basic control flow.
- IR additions (minimal):
  - `Br(u32)`/`BrIf(u32)` and block annotations, or a simple structured form:
    - `Block { label, body }`, `If { cond, then_body, else_body }`.
  - Temporary assignment across branches (SSA join): introduce an explicit `Phi { dst, a: Value, b: Value }` or, simpler, use a pre-allocated `dst` with both branches `local.set dst` then fallthrough uses `dst`.
- Lowering sketch for Option:
  - Evaluate scrutinee to a tuple-like runtime form (placeholder: i32 tag + i32 payload pointer/value when available). In Phase 5, use a stub: Option encoded as i32 (0 = None, 1 = Some) with a separate value (future work for non-int payloads).
  - Emit `If` on tag: in `then` (Some), bind payload and lower arm; in `else` (None), lower other arm.
  - Store both arm results to the same destination value; continue with that value.
- Lowering sketch for Result:
  - Similar to Option; use tag (0 = Ok, 1 = Err) + payload slot; branch and assign a single destination.
- Wasm mapping:
  - Map `If`/`Block` to Wasm structured control flow (`if`, `else`, `end`).
  - Use function locals for branch results; emit `local.set dst` in both branches and `local.get dst` at join.
- Constraints:
  - Strings and non-i32 payloads require a well-defined runtime representation (arrives with Strings Runtime). Until then, restrict match lowering to scrutinees/arms producing `Int`/`Bool`.
- Testing:
  - Add small e2e cases for `Option<Int>` and `Result<Int,Int>` returning `Int`.
