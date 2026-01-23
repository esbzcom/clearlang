# Phase 15.1 - Unsigned Integers (Design Notes)

This note captures the initial policy for unsigned integers in ClearLang. It is a design target; implementation will follow in Phase 15.1.

## Goals
- Provide `U64`, `U128`, `U256` as core numeric types for crypto workloads.
- Make overflow behavior explicit and safe by default.
- Keep proofs and runtime diagnostics deterministic and AI-friendly.

## Overflow policy
- Default arithmetic is **checked**. On overflow, execution traps with a stable runtime code.
- Explicit opt-in operations provide wrap and saturating behavior.

Planned intrinsics (names are placeholders until the API slice lands):
- `std::u64::add_wrap(a, b)`, `std::u64::sub_wrap(a, b)`, `std::u64::mul_wrap(a, b)`
- `std::u64::add_sat(a, b)`, `std::u64::sub_sat(a, b)`, `std::u64::mul_sat(a, b)`
- `std::u64::add_checked(a, b) -> Option<U64>` (optional helper for proofs/tooling)

Overflow traps will reserve `R005` ("numeric overflow") once implemented.

## Literal typing (planned)
- Contextual typing: a bare literal becomes `U64`/`U128`/`U256` when the expected type is that unsigned type.
- Explicit cast fallback: use `U64(42)`, `U128(42)`, or `U256(42)` when there is no context.
- Unsuffixed integer literals remain `Int` when no unsigned context exists.

## Representation (planned)
- `U64` lowers to Wasm `i64` with unsigned ops.
- `U128`/`U256` will use fixed limb layouts (`2 x U64`, `4 x U64`) with helper intrinsics.

## Scope for the first implementation slice
- Enable `U64` end-to-end (parser, typer, IR, codegen, tests).
- Keep `U128`/`U256` gated behind `T110` until limb-based runtime support exists.
