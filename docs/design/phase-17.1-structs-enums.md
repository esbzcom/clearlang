# Phase 17.1 - User-Defined Structs and Enums

## Status
- Design note only. Implementation tracked in `docs/TODO.md` under Phase 17.1.

## Goals
- Add first-class `struct` and `enum` types for user-defined data.
- Support construction, field access, and pattern matching.
- Keep runtime layout deterministic and consistent with existing ABI rules.

## Non-Goals
- Generics and trait bounds (Phase 17.2).
- Module/import system (Phase 17.5).
- Linear-aware collections (Phase 17.6).
- Production-grade persistence of struct values in chain state (runtime packages).

## Syntax (Draft)

Structs:
```
struct Point {
    x: Int;
    y: Int;
}

let p = Point { x: 1, y: 2 };
let x = p.x;
```

Enums (unit + tuple variants in the minimal slice):
```
enum Shape {
    Circle(Int),
    Rect(Int, Int),
    Empty
}

match s {
    Shape::Circle(r) => r * r,
    Shape::Rect(w, h) => w * h,
    Shape::Empty => 0
}
```

If struct-like enum variants are desired, they can be added as an extension:
```
enum Event {
    Transfer { from: Address, to: Address, amount: Int }
}
```

## Typing Rules (Draft)

- `Struct` fields are typed by name; construction must initialize all fields exactly once.
- Field access requires the base expression to be the matching struct type.
- Enums are nominal types; variants are qualified as `Type::Variant`.
- Match arms must be exhaustive or include `_`.
- Match binders introduce names with the corresponding field/variant types.

### Resource Interactions

- Non-resource structs/enums cannot contain resource fields.
- Resource fields are allowed on `resource struct` / `resource enum`.
- Existing `resource Name { ... drop { ... } }` declarations remain supported.

## Runtime Layout

Struct values are represented as **pointers** to heap-allocated, contiguous field layouts,
matching tuple layout rules in `docs/runtime/arrays-tuples.md`.

Enum values reuse the canonical 16-byte variant layout:

```
{ tag, payload_lo, payload_hi, reserved }
```

- `tag` encodes the variant index.
- `payload_lo` stores either an inline scalar or a pointer to a heap allocation.
- `payload_hi` is reserved for metadata (e.g., lengths for pointer-backed data).
- `reserved` is zeroed to keep the layout canonical.

For tuple-like variants with multiple fields, `payload_lo` points to a heap allocation
containing the variant fields laid out like a tuple.

## Pattern Matching

Pattern matching for enums mirrors existing `Option`/`Result` match rules:
- `match` scrutinee must be the enum type.
- Arms bind fields in order for tuple variants.
- Exhaustiveness is required (or a default `_` arm).

## Diagnostics (Draft)

- Unknown field/variant names.
- Missing or duplicate field initializers.
- Non-exhaustive match (new error code).
- Resource field in non-resource struct/enum (new error code).

## Follow-Up Work

- Extend lowering and VC/SMT encoding for enum layouts beyond `Option`/`Result`.
- Add tests: constructor/field access, match exhaustiveness, layout roundtrips.
- Update `docs/typing.md` and `docs/runtime/abi.md` with the final rules.
