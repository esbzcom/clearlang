# Namespace: `std::int`

## Purpose
Deterministic integer models and arithmetic policy surfaces for safe numeric logic.

## Import Paths
- This file is an umbrella spec for concrete integer modules:
  - `std::u64`
  - `std::u128`
  - `std::u256`

## Sub-Namespaces
- `u64`
- `u128`
- `u256`
- `checked`
- `int_error`

## Types
- `U64` (built-in)
- `U128` (built-in)
- `U256` (built-in)
- `Checked<T>`
- `IntError`

## Type/Function Draft

### `u64`
Functions:
- `add_checked(lhs: U64, rhs: U64) -> Result<U64, IntError>`
- `sub_checked(lhs: U64, rhs: U64) -> Result<U64, IntError>`
- `mul_checked(lhs: U64, rhs: U64) -> Result<U64, IntError>`
- `div_checked(lhs: U64, rhs: U64) -> Result<U64, IntError>`
- `mod_checked(lhs: U64, rhs: U64) -> Result<U64, IntError>`
- `add_wrapping(lhs: U64, rhs: U64) -> U64`
- `sub_wrapping(lhs: U64, rhs: U64) -> U64`
- `mul_wrapping(lhs: U64, rhs: U64) -> U64`
- `add_saturating(lhs: U64, rhs: U64) -> U64`
- `sub_saturating(lhs: U64, rhs: U64) -> U64`
- `mul_saturating(lhs: U64, rhs: U64) -> U64`
- `bit_and(lhs: U64, rhs: U64) -> U64`
- `bit_or(lhs: U64, rhs: U64) -> U64`
- `bit_xor(lhs: U64, rhs: U64) -> U64`
- `bit_not(value: U64) -> U64`
- `shl_checked(lhs: U64, rhs: U64) -> Result<U64, IntError>`
- `shr_checked(lhs: U64, rhs: U64) -> Result<U64, IntError>`
- `rotl(lhs: U64, rhs: U64) -> U64`
- `rotr(lhs: U64, rhs: U64) -> U64`
- `to_bytes_le(value: U64) -> Bytes`
- `to_bytes_be(value: U64) -> Bytes`
- `from_bytes_le(input: Bytes) -> Result<U64, IntError>`
- `from_bytes_be(input: Bytes) -> Result<U64, IntError>`

### `u128`
Functions:
- `from_limbs(lo: U64, hi: U64) -> U128`
- `lo(value: U128) -> U64`
- `hi(value: U128) -> U64`
- `add_checked(lhs: U128, rhs: U128) -> Result<U128, IntError>`
- `sub_checked(lhs: U128, rhs: U128) -> Result<U128, IntError>`
- `mul_checked(lhs: U128, rhs: U128) -> Result<U128, IntError>`
- `div_checked(lhs: U128, rhs: U128) -> Result<U128, IntError>`
- `mod_checked(lhs: U128, rhs: U128) -> Result<U128, IntError>`
- `bit_and(lhs: U128, rhs: U128) -> U128`
- `bit_or(lhs: U128, rhs: U128) -> U128`
- `bit_xor(lhs: U128, rhs: U128) -> U128`
- `bit_not(value: U128) -> U128`
- `shl_checked(lhs: U128, rhs: U64) -> Result<U128, IntError>`
- `shr_checked(lhs: U128, rhs: U64) -> Result<U128, IntError>`
- `rotl(lhs: U128, rhs: U64) -> U128`
- `rotr(lhs: U128, rhs: U64) -> U128`

### `u256`
Functions:
- `from_limbs(l0: U64, l1: U64, l2: U64, l3: U64) -> U256`
- `limb0(value: U256) -> U64`
- `limb1(value: U256) -> U64`
- `limb2(value: U256) -> U64`
- `limb3(value: U256) -> U64`
- `add_checked(lhs: U256, rhs: U256) -> Result<U256, IntError>`
- `sub_checked(lhs: U256, rhs: U256) -> Result<U256, IntError>`
- `mul_checked(lhs: U256, rhs: U256) -> Result<U256, IntError>`
- `div_checked(lhs: U256, rhs: U256) -> Result<U256, IntError>`
- `mod_checked(lhs: U256, rhs: U256) -> Result<U256, IntError>`
- `bit_and(lhs: U256, rhs: U256) -> U256`
- `bit_or(lhs: U256, rhs: U256) -> U256`
- `bit_xor(lhs: U256, rhs: U256) -> U256`
- `bit_not(value: U256) -> U256`
- `shl_checked(lhs: U256, rhs: U64) -> Result<U256, IntError>`
- `shr_checked(lhs: U256, rhs: U64) -> Result<U256, IntError>`
- `rotl(lhs: U256, rhs: U64) -> U256`
- `rotr(lhs: U256, rhs: U64) -> U256`

### `checked`
Functions:
- `ok(value: T) -> Checked<T>`
- `err(error: IntError) -> Checked<T>`
- `to_result(value: Checked<T>) -> Result<T, IntError>`

### `int_error`
Functions:
- `code(err: IntError) -> ErrorCode`
- `equals(a: IntError, other: IntError) -> Bool`

## First-Production Cut (recommended)
- Keep `U64` checked/wrapping/saturating arithmetic, checked `div/mod`, bitwise/shift/rotate, and byte conversions.
- Keep `U128`/`U256` limb constructors/accessors plus checked `add/sub/mul/div/mod` and bitwise/shift/rotate.
- Keep `IntError` and `Checked<T>::to_result`.
- Defer broader wide-int API until proof/model coverage is complete.

## Notes
- Numeric semantics MUST be deterministic and proof-compatible.
- Overflow behavior MUST be explicit; implicit runtime coercion MUST NOT occur.
- Checked `div/mod` MUST fail deterministically on zero divisor.
- Checked shifts MUST fail deterministically when shift amount is out of range.
- Endianness for byte conversions MUST be stable and unambiguous (`*_le` little-endian, `*_be` big-endian).

## Security Considerations
- Crypto workflows SHOULD prefer checked operations in consensus/security-critical code paths.
- Wrapping arithmetic SHOULD be used only where modular arithmetic is explicitly intended.
- Shift and rotate contracts MUST remain architecture-independent.

## Contract Conformance Checklist
- All checked ops MUST map each failure class to deterministic `IntError` codes.
- Wrapping/saturating behavior MUST be bit-for-bit stable across targets.
- Limb constructors/accessors (`U128`/`U256`) MUST preserve canonical limb ordering.
- Byte conversion round-trips MUST be deterministic for all valid inputs.

## Summary
- Exposes checked, saturating, and wrapping arithmetic APIs with explicit behavior.
- Supports crypto and finance workflows requiring fixed-width integer guarantees.
- Must keep overflow semantics stable across platforms and toolchain versions.


