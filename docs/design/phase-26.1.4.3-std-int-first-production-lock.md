# Phase 26.1.4.3 - `std::int` First-Production API Lock

## Scope
This lock defines the first-production `std::int` surface for Gate B execution.

## First-Production API (Release-Enabled)

`std::u64`:
- `add_wrapping`, `sub_wrapping`, `mul_wrapping`
- `add_saturating`, `sub_saturating`, `mul_saturating`
- `rotl`, `rotr`
- `to_bytes_le`, `to_bytes_be`
- `from_bytes_le`, `from_bytes_be`

`std::u128`:
- `from_limbs`
- `lo`
- `hi`

`std::u256`:
- `from_limbs`
- `limb0`
- `limb1`
- `limb2`
- `limb3`

Compatibility aliases (release-enabled):
- `std::u64::{add_wrap,sub_wrap,mul_wrap,add_sat,sub_sat,mul_sat}` as aliases to wrapping/saturating canonical names.

## Determinism Contracts
- Wrapping and saturating operations MUST be deterministic and architecture-independent.
- Rotate operations MUST be deterministic and architecture-independent.
- Byte conversion endianness MUST remain stable (`*_le` little-endian, `*_be` big-endian).
- Limb constructor/accessor ordering for `U128/U256` MUST remain canonical and stable.

## Deferred (Non-Release) Symbols

`std::u64`:
- `add_checked`, `sub_checked`, `mul_checked`, `div_checked`, `mod_checked`
- `bit_and`, `bit_or`, `bit_xor`, `bit_not`
- `shl_checked`, `shr_checked`

`std::u128`:
- checked arithmetic (`add/sub/mul/div/mod`)
- bitwise (`bit_and/bit_or/bit_xor/bit_not`)
- checked shifts (`shl_checked`, `shr_checked`)
- rotates (`rotl`, `rotr`)

`std::u256`:
- checked arithmetic (`add/sub/mul/div/mod`)
- bitwise (`bit_and/bit_or/bit_xor/bit_not`)
- checked shifts (`shl_checked`, `shr_checked`)
- rotates (`rotl`, `rotr`)

Additional deferred namespaces:
- `std::checked::{ok,err,to_result}`
- `std::int_error::{code,equals}`

## Gate B Exit for `std::int`
`std::int` slice is complete when:
1. API/doc/metadata surface is consistent with this lock.
2. `typed|runtime|proved` statuses are recorded in `docs/std/coverage-matrix.md`.
3. Release profile fails closed for deferred `std::int` symbols.
4. CI has deterministic contract tests for all release-enabled `std::int` symbols.
