# Package: `std::int`

## Purpose
Deterministic integer models and arithmetic policy surfaces for safe numeric logic.

## Key Types
- `U64`
- `U128`
- `U256`
- `Checked<T>`
- `IntError`

## Class/Method Draft

### `U64`
Methods:
- `add_checked(self, rhs: U64) -> Result<U64, IntError>`
- `sub_checked(self, rhs: U64) -> Result<U64, IntError>`
- `mul_checked(self, rhs: U64) -> Result<U64, IntError>`
- `add_wrapping(self, rhs: U64) -> U64`
- `sub_wrapping(self, rhs: U64) -> U64`
- `mul_wrapping(self, rhs: U64) -> U64`
- `add_saturating(self, rhs: U64) -> U64`
- `sub_saturating(self, rhs: U64) -> U64`
- `mul_saturating(self, rhs: U64) -> U64`
- `to_bytes_le(self) -> Bytes`
- `to_bytes_be(self) -> Bytes`
- `from_bytes_le(input: Bytes) -> Result<U64, IntError>`
- `from_bytes_be(input: Bytes) -> Result<U64, IntError>`

### `U128`
Methods:
- `from_limbs(lo: U64, hi: U64) -> U128`
- `lo(self) -> U64`
- `hi(self) -> U64`
- `add_checked(self, rhs: U128) -> Result<U128, IntError>`
- `sub_checked(self, rhs: U128) -> Result<U128, IntError>`

### `U256`
Methods:
- `from_limbs(l0: U64, l1: U64, l2: U64, l3: U64) -> U256`
- `limb0(self) -> U64`
- `limb1(self) -> U64`
- `limb2(self) -> U64`
- `limb3(self) -> U64`
- `add_checked(self, rhs: U256) -> Result<U256, IntError>`
- `sub_checked(self, rhs: U256) -> Result<U256, IntError>`

### `Checked<T>`
Methods:
- `ok(value: T) -> Checked<T>`
- `err(error: IntError) -> Checked<T>`
- `to_result(self) -> Result<T, IntError>`

### `IntError`
Methods:
- `code(self) -> ErrorCode`
- `equals(self, other: IntError) -> Bool`

## First-Production Cut (recommended)
- Keep `U64` checked/wrapping/saturating arithmetic and byte conversions.
- Keep `U128`/`U256` limb constructors/accessors plus checked `add/sub`.
- Keep `IntError` and `Checked<T>::to_result`.
- Defer broader wide-int API until proof/model coverage is complete.

## Notes
- Numeric semantics must be deterministic and proof-compatible.
- Overflow behavior must be explicit; no implicit runtime coercion.

## Summary
- Exposes checked, saturating, and wrapping arithmetic APIs with explicit behavior.
- Supports crypto and finance workflows requiring fixed-width integer guarantees.
- Must keep overflow semantics stable across platforms and toolchain versions.
