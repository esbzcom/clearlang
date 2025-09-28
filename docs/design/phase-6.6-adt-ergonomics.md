# Phase 6.6 - ADT Ergonomics Design Note

## Overview
- **Goal**: provide lightweight surface sugar for Option/Result ergonomics (if let, ??, ?) while keeping compilation proofs and diagnostics predictable.
- **Scope**: parser, typer, VC generation, CLI tooling. Codegen remains experimental until lowering for Expr::Try and variant data is implemented.
- **Flagging**: all sugars remain behind the Phase 6 experimental gate until lowering and VC coverage are production ready.

## Surface Syntax & Desugars
| Sugar | Desugar | Notes |
| --- | --- | --- |
| if let Some(x) = opt { a } else { b } | match opt { Some(x) => a, None => b } | Supports Some, Ok, Err. Parser enforces explicit else. |
| lhs ?? rhs | match lhs { Some(v) => v, None => rhs } | Chains left-associatively. Uses unique temporary binders to avoid collisions. |
| expr? | Option: unwrap or early return None. Result: unwrap Ok or return Err. | Typing ensures operand/result alignment; lowering currently unimplemented. |

## Typing Rules
- if let uses existing match typing (T201-T205). Binder names are checked against scope to prevent shadowing (T205).
- ?? requires Option<T> left operand and yields T; right operand must also type to T. Failure triggers T603 style mismatch.
- expr?:
  - Operand must be Option<T> or Result<T,E> (T602).
  - Surrounding function must return Option<T> or Result<T,E> respectively (T601/T604).
  - Inner types must match (T603/T605).
  - Constructors (None, Ok, Err) now validate return context (T607-T613).
- $return binding seeded in typer environments allows Expr::Try to validate the enclosing return type.

## Effect & Purity Considerations
- Sugars are pure; max_effect treats Expr::Try as the effect of the operand.
- if let/?? inherit operand effects; existing guard enforcement covers branches.
- No additional runtime guard requirements introduced.

## VC Generation
- generate_vcs traverses Expr::Try and match to avoid panics.
- expr_to_source renders `try` expressions as <expr>? for readability.
- expr_to_smt2 currently emits ; unsupported try ... comments for unlowered constructs, keeping output stable while signalling missing semantics.
- Future work: model Option/Result in SMT (e.g., algebraic datatypes or tagged pairs) once lowering/codegen is ready.

## Testing Strategy
- Parser/typer suites cover positive and negative cases for each sugar.
- VC tests assert stable generation with sugar in bodies to prevent regressions before SMT encoding is complete.
- CLI integration tests remain gated until lowering exists.

## Worked Examples

### `if let` / `??`
```cl
pure function default_or_zero(opt: Option<Int>) -> Int
    ensure { result >= 0 }
{ if let Some(v) = opt { v } else { 0 } }
```
`generate_vcs` currently emits:
```
(=> true ; unsupported expr If { ... })
```
The placeholder comment keeps SMT output stable until match lowering is encoded.

### `?` propagation
```cl
pure function bump_when_positive(opt: Option<Int>) -> Option<Int>
    ensure { result == result }
{ if opt? > 0 { Some(opt? + 1) } else { None } }
```
VC output contains the expression tree inside the existing placeholder:
```
(=> true (= ; unsupported expr If { ... Try { ... } } ; unsupported expr If { ... Try { ... } }))
```
These snapshots are verified by `crates/typer/tests/vc.rs`.

## Codegen / Lowering Plan
1. **Representation**: define a canonical in-memory encoding for Option<T>/Result<T,E> ({tag: i32, payload...}) in IR/Wasm.
2. **Constructors**: lower None/Some/Ok/Err into the representation; update builtins returning Option/Result to emit the same encoding.
3. **Expr::Try Lowering**: translate to conditional logic that inspects the tag; on None/Err, synthesize the corresponding return value and emit an Instr::Ret.
4. **Proof Section**: extend proof packaging schema if new runtime intrinsics are required.
5. **Validation**: add Wasm smoke tests exercising the runtime behaviour of ? under both success and early-return paths.
- Until the above is implemented, the experimental flag should remain.

## Open Follow-Ups
- SMT encoding for match/try (avoid placeholder comments in VCs).
- VC snapshots exercising if let, ??, and ?.
- IR lowering/codegen implementation for Expr::Try and constructors.
- CLI docs explaining experimental flag usage and runtime guarantees once lowering ships.
