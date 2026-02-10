# Arrays, Slices, and Tuples Layout (Phase 17.4)

This document specifies the in-memory layout for dynamic arrays, slices, and tuples.
Values are represented as 32-bit pointers to heap-allocated storage in linear memory.

## Scope
- Dynamic arrays: `Array<T>`.
- Slices: `Slice<T>`.
- Fixed-size sugar: `[T; N]` is syntax sugar for `Array<T>` with a length contract.
- Tuples: `(T1, T2, ...)` with arity >= 2.

## Value Representation
Array, slice, and tuple values are passed as `i32` pointers to the layouts
described below.

## Element Layout Basics
Elements are laid out using each type's runtime representation.

| Type | Size | Align | Representation |
| --- | --- | --- | --- |
| `Int` | 4 | 4 | i32 |
| `Bool` | 4 | 4 | i32 (0/1) |
| `U8` | 1 | 1 | u8 |
| `U64` | 8 | 8 | i64 |
| `U128` | 4 | 4 | i32 pointer to limb buffer |
| `U256` | 4 | 4 | i32 pointer to limb buffer |
| `String` | 4 | 4 | i32 pointer to `[u32 len][u8 bytes]` |
| `Bytes` | 4 | 4 | i32 pointer to `[u32 len][u8 bytes]` |
| `Option<T>` | 4 | 4 | i32 pointer to 16-byte variant layout |
| `Result<T,E>` | 4 | 4 | i32 pointer to 16-byte variant layout |
| `Struct` | 4 | 4 | i32 pointer to struct allocation (tuple layout) |
| `Enum` | 4 | 4 | i32 pointer to 16-byte variant layout |
| `List/Set/Map` | 4 | 4 | i32 pointer to collection header (see `docs/design/phase-17.3-collections-runtime.md`) |
| `Resource` | 4 | 4 | i32 handle (runtime-defined) |
| `Array<T>` | 4 | 4 | i32 pointer to array header |
| `Slice<T>` | 4 | 4 | i32 pointer to slice header |
| `Tuple` | 4 | 4 | i32 pointer to tuple allocation |

Notes:
- Arrays/slices containing resources are currently unsupported (enforced by the typer).
- Tuples may contain resources; such tuple values are linear-owned by the same transitive ownership rule used for other wrappers.
- `U128`/`U256` use limb buffers defined in `docs/design/phase-15.1-unsigned-ints.md`.
- Enums use the variant layout; for multi-field variants, `payload_lo` points to a tuple allocation that follows the same layout rules.

## Array and Slice Layout

Both arrays and slices use the same header layout:
```
struct SliceHeader {
    u32 len;
    u32 data_ptr;  // pointer to the first element
}
```

Element layout:
- Element size = `size_of(T)`
- Element alignment = `align_of(T)`
- Stride = `align_up(size_of(T), align_of(T))`
- Data size = `len * stride`

Elements are stored contiguously starting at `data_ptr`.

Note: `[T; N]` does not change the runtime layout; it is enforced as a length
constraint at the type level.

Example: `Array<U8>` with `len = 4`
```
header:
  len = 4
  data_ptr -> element buffer

element buffer:
  offset 0: u8
  offset 1: u8
  offset 2: u8
  offset 3: u8
```

Example: `Array<U64>` with `len = 2` (stride 8)
```
header:
  len = 2
  data_ptr -> element buffer

element buffer:
  offset 0: i64
  offset 8: i64
```

## Tuple Layout
Tuples are laid out in order with natural alignment and padding.

Algorithm:
1. Start at offset 0.
2. For each field, align the offset up to the field's alignment.
3. Place the field and advance by its size.
4. Final size is rounded up to the maximum field alignment.

Example: `(U64, U8, Bool)`
```
offset 0:  i64 (U64)
offset 8:  u8  (U8)
offset 9:  padding to align Bool (align 4)
offset 12: i32 (Bool)
total size: 16 (aligned to 8)
```

## Implementation Status
Tuples use this layout today. Dynamic arrays/slices are planned for Phase 17.4.
