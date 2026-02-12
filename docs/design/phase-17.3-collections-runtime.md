# Phase 17.3 - Collections Runtime Semantics

## Status
Design note for the implemented Phase 17.3 collections runtime; see `docs/TODO.md` for completion tracking.

## Goals
- Provide deterministic runtime semantics for `List`, `Set`, and `Map`.
- Keep pure operations pure (no host imports); use in-wasm runtime helpers.
- Preserve the existing API surface and the `_mut` + `can_mut` guard model.
- Document a stable layout and error taxonomy so codegen and diagnostics stay aligned.

## Non-Goals
- Hash tables or performance tuning.
- Linear-aware collections (Phase 17.6).
- Module system or stdlib source files (Phase 17.5).
- Iterators or higher-level collection APIs.

## Surface and Semantics

List
- `std::list::new<T>() -> List<T>` (type args inferred from expected type when available; otherwise T206).
- `len(l)` returns the element count.
- `get(l, i)` returns `Some(T)` if `0 <= i < len`, otherwise `None`.
- `push(l, x)` appends and returns the new list.
- `insert(l, x, i)` inserts at index `i` where `0 <= i <= len`; otherwise traps.
- `remove(l, i)` removes index `i` where `0 <= i < len`; otherwise traps.
- `pop(l)` returns `Some(T)` if `len > 0`, otherwise `None`.

Set
- `std::set::new<T>() -> Set<T>` (type args inferred from expected type when available; otherwise T206).
- `len`, `contains`, `insert`, `remove` are defined by equality.
- `insert` is idempotent (no duplicates); `remove` is a no-op if missing.

Map
- `std::map::new<K,V>() -> Map<K,V>` (type args inferred from expected type when available; otherwise T206).
- `len`, `contains`, `get`, `insert`, `remove` are defined by key equality.
- `insert` replaces the existing value if the key is present.
- `remove` is a no-op if the key is missing.

Mut variants and guards
- `_mut` variants (`push_mut`, `insert_mut`, `remove_mut`, etc.) require `mut` effect
  and the guard `require { std::<collection>::can_mut(var) }` on the first argument.
- In Phase 17.3, `can_mut` is a pure predicate and is currently lowered as `true`;
  `_mut` operations must preserve value semantics and may clone as needed.
- Future phases can optimize in-place updates when uniqueness tracking exists;
  the guard keeps that upgrade sound and backward compatible.

## Runtime Representation (v1)

All collection values are `i32` pointers to heap-allocated headers in linear memory.
The runtime uses the shared bump allocator; no collection frees occur in 17.3.

### List and Set Header
```
struct ListHeader {
    u32 len;
    u32 cap;
    u32 flags;    // reserved (bit 0 may encode uniqueness later)
    u32 data_ptr; // i32 pointer to element buffer
}
```
- `Set<T>` reuses the same header and buffer layout as `List<T>`.
- Elements are stored contiguously in the buffer using the element layout rules
  from `docs/runtime/arrays-tuples.md` (size/align/stride).

### Map Header
```
struct MapHeader {
    u32 len;
    u32 cap;
    u32 flags;    // reserved
    u32 data_ptr; // i32 pointer to element buffer
}
```
- Elements are stored as `(K, V)` tuples using the tuple layout rules
  in `docs/runtime/arrays-tuples.md`.
- This means a `Map<K,V>` is a list of key/value tuples with linear search.

## Key Equality (Map/Set)

Map and Set require equatable keys. Phase 17.3 uses compiler-synthesized
structural equality and linear search (hashing is deferred).

Equatable types (v1):
- Primitives: `Int`, `Bool`, `U8`, `U64`, `U128`, `U256`, `String`, `Bytes`.
- `Option<T>` / `Result<T,E>` where payloads are equatable.
- Structs, enums, and tuples composed only of equatable types.

Non-equatable in v1:
- `List`, `Set`, `Map`, `Resource`, and any type containing them.

Typing rules:
- `Map<K,V>` and `Set<T>` are rejected if `K`/`T` is not equatable.
- Introduce a dedicated diagnostic code for unsupported key types.

Equality implementation:
- The compiler emits structural equality logic per key type.
- `String`/`Bytes` use `std::str::eq` / `std::bytes::eq`.
- `Option`/`Result` compare tags then payloads.
- Structs/tuples compare fields in order; enums compare tag then payload.

Note: `Array<T>` (including `[T; N]` sugar) equality is deferred until Phase 17.4+
defines a loop or builtin equality strategy; arrays are not equatable in the unified
dynamic model.

## Errors and Traps

- Bounds errors for `insert`/`remove` (List) trap with a new runtime code
  `R009` (CollectionBounds). `get`/`pop` remain total via `Option`.
- Invalid pointers/headers (null, misaligned, or out-of-bounds header) trap with
  `R010` (InvalidBuffer).
- Allocation failures reuse `R001` (AllocatorOom) from the shared allocator.

Diagnostics docs (`docs/diagnostics.md`) should be updated when these are wired.

## Doc Updates (when implementing)
- `docs/runtime/arrays-tuples.md`: replace “runtime-defined” with the header layout.
- `docs/collections.md`: update semantics and error behavior.
- `docs/typing.md`: document key-equatable restrictions and mut semantics.

## Open Questions
- Whether to add hashing + `Hash` trait in Phase 17.8 or keep linear search longer.
- Whether to add uniqueness tracking to make `can_mut` meaningful.
- Whether map/set should preserve insertion order (current design does via list).
