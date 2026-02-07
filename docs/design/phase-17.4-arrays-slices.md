# Phase 17.4 - Arrays and Slices

## Status
Design note only. Implementation tracked in `docs/TODO.md` under Phase 17.4.

## Goals
- Add runtime-length arrays and slice views with indexing semantics.
- Keep syntax small and familiar while preserving AI-friendly determinism.
- Provide explicit, stable bounds behavior (compile-time and runtime).
- Simplify arrays to a single runtime type, with optional fixed-size sugar.

## Design Principles Check
- Simple for users: one array type (`Array<T>`) and one literal meaning.
- AI-friendly: single representation reduces ambiguity for generation/repair.
- Provably correct: bounds are enforced deterministically; length constraints move to `require`/`ensure`.
- Crypto-focused: fixed sizes are expressed as explicit length contracts (and can be proven in VC/SMT later).

## Non-Goals
- Mutating element assignment (`arr[i] = x`) or in-place updates.
- Iterators, range syntax, or high-level slicing sugar.
- Equality for dynamic arrays/slices (defer until a loop/iterator story exists).
- Borrow/lifetime tracking (no `&` or alias analysis in this phase).

## Types and Syntax

Decision: unify arrays to a single dynamic representation.

Types:
```
Array<T>  // owned, runtime-length contiguous array
Slice<T>  // view over a contiguous region
```

Notes:
- `Array<T>` and `Slice<T>` are distinct types, but share the same runtime
  representation (see Runtime Representation).
- Array literals `[e1, e2, ...]` always construct `Array<T>`.
- Fixed-size array syntax `[T; N]` is retained as sugar for `Array<T>` with an
  implicit length contract `require { std::array::len(x) == N }`. It does not
  introduce a distinct runtime type or layout.

## Surface Operations (Draft)

Minimal builtins to make the types usable:
```
std::array::len<T>(a: Array<T>) -> Int
std::slice::len<T>(s: Slice<T>) -> Int
std::slice::from_array<T>(a: Array<T>) -> Slice<T>
std::slice::sub<T>(s: Slice<T>, start: Int, len: Int) -> Slice<T>
```

Notes:
- `from_array` creates a slice view of the full array.
- `sub` returns a view into `s` with bounds checks (no copying).
- If `Array<T>` construction needs an explicit API beyond literals, add:
  `std::array::from_list<T>(l: List<T>) -> Array<T>` (copies elements).

## Indexing Semantics

Index expressions apply to `Array<T>` and `Slice<T>`:
```
arr[i]
```

Rules:
- `i` must be `Int`.
- Emit a runtime check: `0 <= i < len`, where `len` comes from the header.
- Out-of-bounds at runtime traps with a dedicated bounds code (see Diagnostics).

Tuple indexing remains constant-only (T115) and unchanged.

## Runtime Representation (v1)

Arrays and slices share a small header layout:
```
struct SliceHeader {
    u32 len;
    u32 data_ptr;  // pointer to the first element
}
```

Representation:
- `Array<T>` and `Slice<T>` values are `i32` pointers to `SliceHeader`.
- The element buffer is contiguous and uses the same element layout rules as
  `docs/runtime/arrays-tuples.md` (size/align/stride).
- `Array<T>` owns the data buffer; `Slice<T>` is a view and may point into a
  subrange of another array's data buffer.

Allocation strategy:
- `Array<T>` allocates the data buffer (and the header).
- `Slice<T>` allocates only the header, reusing an existing data buffer.

## Typing Rules
- `Array<T>` and `Slice<T>` are only well-formed when `T` is non-resource,
  matching the existing resource-in-collection checks.
- Refinement aliases are allowed as element types.
- No implicit equality support for `Array<T>`/`Slice<T>` in Phase 17.4.
- `[T; N]` is accepted in type positions as sugar for `Array<T>` plus a length
  contract. Length mismatches on array literals are compile-time errors; other
  mismatches become runtime contract failures unless a proof discharges them.

## Diagnostics and Traps

Compile-time:
- T114 remains the error for constant out-of-bounds array indices.

Runtime:
- Introduce a dedicated trap code for array/slice bounds (proposed `R011`).
  Alternatively, reuse `R009` if the project prefers a single bounds trap.
- Update `docs/diagnostics.md` when wiring the runtime trap.

## VC/Proof Notes
- Indexing currently relies on runtime guards rather than VC obligations.
- A future optimization may allow proving `0 <= i < len` to eliminate guards,
  but this is out of scope for Phase 17.4.

## Implementation Notes (for TODO breakdown)
- Parser/AST: add `Array<T>` + `Slice<T>` type constructors and keep `[T; N]`
  syntax as sugar that desugars to `Array<T>` plus a length constraint.
- Typer: add well-formedness checks, array-literal typing to `Array<T>`, enforce
  `[T; N]` length constraints, and indexing support for `Array<T>`/`Slice<T>`.
- Lowering/codegen: emit bounds checks for dynamic arrays/slices; load `len`
  and `data_ptr` from the header and apply stride-based addressing.
- Runtime/tests: add `std::array::len`, `std::slice::{len,from_array,sub}`
  helpers, trap tests for OOB, and diagnostic snapshots.
- Docs/ABI: update `docs/runtime/arrays-tuples.md`, `docs/typing.md`, and any
  layout notes to document the dynamic header and the `[T; N]` sugar model.

## Open Questions
- Should we allow `Slice<T>` creation from `Bytes`/`String` in Phase 17.4?
- Should array/slice bounds reuse `R009` or use a new trap code?
