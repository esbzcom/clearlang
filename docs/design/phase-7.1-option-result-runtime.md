# Phase 7.1 - Option/Result Runtime Layout

## Goal
Freeze the in-memory representation for `Option<T>` and `Result<T, E>` so lowering, codegen, and proofs can rely on a single `{ tag, payload }` contract.

## Canonical Encoding
Each variant value occupies **16 bytes** (4 x `i32`) in linear memory or the stack frame. The layout is shared by `Option` and `Result`:

| Offset (bytes) | Slot            | Type | Meaning |
| -------------- | --------------- | ---- | ------- |
| 0              | `tag`           | `i32`| Discriminant; see tag table below. |
| 4              | `payload_lo`    | `i32`| Low word of the payload (inline scalars) or pointer for heap data. |
| 8              | `payload_hi`    | `i32`| High word / length metadata paired with `payload_lo`. |
| 12             | `reserved`      | `i32`| Always zeroed; preserves 16-byte alignment and a future-proof metadata slot.

Reasons for the shape:
- 16-byte stride keeps every value 8-byte aligned and matches existing pointer+length encodings (e.g., strings) without reallocation.
- Reusing the same layout for Option/Result avoids variant-specific tooling and simplifies SMT encodings.
- The reserved slot allows extending the payload without breaking existing codegen or proofs.

## Tag Assignments
| Variant kind | Tag constant | Meaning |
| ------------ | ------------ | ------- |
| `Option`     | `0`          | `None` (payload slots zeroed). |
|              | `1`          | `Some`. Payload slots hold the `Some` value, encoded using the two-word convention. |
| `Result`     | `0`          | `Err`. Payload slots encode the error payload. |
|              | `1`          | `Ok`. Payload slots encode the success payload. |

Any tag other than `0` or `1` is **invalid** and must trigger a trap (see below).

## Payload Rules
- **Inline scalars** (`Int`, `Bool`) use `payload_lo` for the value and leave `payload_hi`/`reserved` as zero.
- **Pointer-backed data** (strings, future aggregates) store a pointer in `payload_lo` and the logical length/metadata in `payload_hi`. The pointed-to region follows the existing runtime invariants (e.g., string header `{len, bytes...}`).
- **Zero-sized payloads** (`None`, `Err` for no-error payload) zero all payload slots to maintain deterministic hashing and proof friendliness.
- All constructors and destructors must zero `reserved` after writes to keep the layout canonical for hashing and SMT.

## Invalid Tag Policy
Introduce a shared runtime helper (emitted once per module):
```
(func $clg::trap_invalid_tag (param $tag i32) (param $kind i32)
  ;; $kind = 0 (Option), 1 (Result)
  ;; Implementation: call into the existing trap machinery with code R003 (to be allocated).
)
```
Lowering that reads a tag must insert:
1. A bounds check `tag <= 1`.
2. On failure, call `trap_invalid_tag` before unwinding (mirrors existing R00x traps).

This keeps diagnostics deterministic and helps AI tooling reason about variant misuse.

## Integration Points
- **Typer / Lowering**: future IR helpers (`ir::Variant { tag, payload_lo, payload_hi }`) must follow this layout exactly. Sugar rewrites (`if let`, `??`, `?`) emit tag comparisons against `0`/`1` only.
- **IR helpers**: `Instr::VariantInit` constructs a variant in the canonical layout, `Instr::VariantLoadTag|PayloadLo|PayloadHi` project individual fields, and `Instr::ReturnIf` performs failure-tag early returns.
- **Codegen**: Wasm emission writes tag first, then payload words, then zeroes `reserved`. Clearing happens even when a slot already held the correct data to avoid stale bits.
- **Proof Packaging**: hashing and canonical orderings treat the 16-byte blob as the authoritative representation of a variant value.
- **Tooling**: `--emit-vcs` and SMT encoders can now model variants as `(tag: i32, lo: i32, hi: i32)` tuples, referencing this document.

## Diagnostics Mapping
Reserve runtime code `R003` for invalid variant tags. All trap sites (Option/Result destructors, `Expr::Try`, future pattern matching) must surface `R003` with structured context `{ kind: "Option"|"Result", tag }`.

## Future Work
- Implement the constructors/destructors and lowering helpers using this contract.
- Extend SMT encoding so VCs reason over the explicit `(tag, lo, hi)` tuple.
- Add Wasm regression tests that cover well-formed and invalid-tag scenarios (expecting an `R003` trap).

## References
- Phase 6.6 ADT ergonomics design (`docs/design/phase-6.6-adt-ergonomics.md`).
- Runtime strings layout (`docs/runtime/strings.md`) for pointer/length conventions.
- Typing overview (`docs/typing.md`) - updated to link to this note.

