# Phase 15.1 - Unsigned Integers (Design Notes)

This note captures the policy for unsigned integers in ClearLang. U64/U128/U256 support is implemented with limb-backed helpers for U128/U256.

## Goals
- Provide `U64`, `U128`, `U256` as core numeric types for crypto workloads.
- Make overflow behavior explicit and safe by default.
- Keep proofs and runtime diagnostics deterministic and AI-friendly.

## Overflow policy
- Default arithmetic is **checked**. On overflow, execution traps with a stable runtime code.
- Explicit opt-in operations provide wrap and saturating behavior.

U64 intrinsics:
- `std::u64::add_wrap(a, b)`, `std::u64::sub_wrap(a, b)`, `std::u64::mul_wrap(a, b)`
- `std::u64::add_sat(a, b)`, `std::u64::sub_sat(a, b)`, `std::u64::mul_sat(a, b)`
- `std::u64::add_checked(a, b) -> Option<U64>` (optional helper for proofs/tooling; not implemented yet)

U128/U256 helper intrinsics:
- `std::u128::from_limbs(lo, hi) -> U128`, `std::u128::{lo,hi}(value) -> U64`
- `std::u256::from_limbs(limb0, limb1, limb2, limb3) -> U256`
- `std::u256::limb0/limb1/limb2/limb3(value) -> U64`

Overflow traps use `R005` ("numeric overflow").

## Literal typing
- Contextual typing: a bare literal becomes `U64`/`U128`/`U256` when the expected type is that unsigned type.
- Explicit cast fallback: use `U64(42)`, `U128(42)`, or `U256(42)` when there is no context.
- Unsuffixed integer literals remain `Int` when no unsigned context exists.

## Representation
- `U64` lowers to Wasm `i64` with unsigned ops.
- `U128`/`U256` are stored in linear memory as fixed limb buffers (`2 x U64`, `4 x U64`)
  and are passed around as pointers (i32).

## Implementation status (Phase 15.1)
- `U64` end-to-end (parser, typer, IR, codegen, tests).
- `U128`/`U256` limb helpers and layout support (no arithmetic yet; use T110 for unsupported ops).
