# Phase 17.2 - Generics and Traits

## Status
- Design note only. Implementation tracked in `docs/TODO.md` under Phase 17.2.

## Goals
- Add type parameters to `struct`, `enum`, `function`, and `type` aliases.
- Introduce traits/interfaces with static dispatch and trait bounds.
- Enable generic stdlib collections (`List<T>`, `Map<K,V>`, `Set<T>`) and shared helpers.
- Keep compilation deterministic and simple via monomorphization.

## Non-Goals
- Trait objects or dynamic dispatch.
- Specialization, negative impls, or overlapping impls.
- Higher-kinded types, higher-rank polymorphism, or generic associated types.
- Default trait method bodies (possible later extension).
- Operator overloading or implicit coercions.

## Syntax (Draft)

Type parameters on structs/enums:
```
struct Box<T> {
    value: T;
}

enum Result<T, E> {
    Ok(T),
    Err(E)
}
```

Type parameters on functions:
```
pure function id<T>(x: T) -> T {
    x
}
```

Traits and impls (static dispatch):
```
trait Eq {
    pure function eq(a: Self, b: Self) -> Bool;
}

impl Eq for Int {
    pure function eq(a: Int, b: Int) -> Bool { a == b }
}
```

Trait bounds (inline and `where`):
```
pure function eq_pair<T: Eq>(a: T, b: T) -> Bool {
    Eq::eq(a, b)
}

pure function eq_pair2<T>(a: T, b: T) -> Bool
    where T: Eq
{
    Eq::eq(a, b)
}
```

Type arguments at call sites (inference first, explicit when needed):
```
let x = id(1);            // infer T = Int
let y = id<Int>(1);       // explicit
let z = std::map::new<K,V>();
```

Notes:
- Trait methods are declared as signatures only; no default bodies in this phase.
- `Self` is only valid inside trait and impl declarations.
- Trait methods are called with `Trait::method(...)` (no dot-method syntax yet).
- Type arguments use `Name<T, U>` and `call<T>(...)` syntax.

### Bounds Placement and Syntax (Draft)

- Inline bounds are allowed in the type parameter list:
  - `function f<T: Eq>(x: T) -> Bool { ... }`
- A `where` clause appears after the return type and before any `require`/`ensure`
  clauses and the function body.
- Multiple bounds are comma-separated. If multiple traits are required for a
  single parameter, repeat the parameter name:
  - `where T: Eq, T: Hash`

## Typing and Inference

- Generic parameters are scoped to the item they are declared on.
- Type arguments are inferred from:
  - Function arguments.
  - Expected type (contextual return type).
- If inference is ambiguous or unused type parameters remain, emit a diagnostic
  and require explicit type arguments.
- Trait bounds are not inferred; they must appear inline or in a `where` clause.
- Trait bounds are required when calling a trait method; bounds are checked
  when monomorphizing each call site.

### Effects and Contracts
- Trait method signatures include effects; impls must match exactly.
- `require`/`ensure` clauses on generic functions are preserved per instantiation.
- Pure/Mut/Io rules apply after monomorphization (no special-casing for generics).

### Resources and Refinements
- Type arguments may be refined aliases; refinement obligations are preserved
  after substitution.
- Resource types are allowed as type parameters, but existing restrictions still
  apply (e.g., `List<Resource>` remains rejected until Phase 17.6).

## Coherence Rules (Draft)

- For each `(Trait, ConcreteType)` pair, there must be exactly one applicable
  `impl` after type substitution.
- Overlapping impls are rejected, even if one is "more specific."
- There is no orphan rule yet (no module system); this will be revisited in
  Phase 17.5 to avoid cross-module conflicts.

## Monomorphization Model

- All generics are monomorphized at compile time into concrete copies.
- Trait calls are resolved to the concrete `impl` function at compile time.
- No vtables or dynamic dispatch are generated.
- VCs and proofs are generated per monomorphized instance.

## Stdlib Impact

- `Option<T>`, `Result<T,E>`, `List<T>`, `Map<K,V>`, `Set<T>` are treated as
  generic types. Initially they may remain compiler-known builtins but are
  typed using the generic machinery.
- Future cleanup can move these definitions into stdlib source once the module
  system exists.

## Diagnostics (Draft)

- Generic parameter count mismatch on types or calls.
- Ambiguous type inference for a generic function call.
- Unused type parameters on a declaration.
- Missing or unsatisfied trait bounds.
- Overlapping or duplicate impls.

## Open Questions

- Whether to allow default trait method bodies (and how to check effects).
  - Recommended: defer; if added later, require explicit effect on the default
    body and enforce exact effect matching on overrides.
- Syntax for explicitly selecting an impl when multiple could match (if ever
  allowed).
  - Recommended: avoid; keep coherence strict so selection is never required.
- How to serialize generic instantiations in debug names and proof metadata.
  - Recommended: canonical, deterministic mangling that expands aliases and
    orders type arguments consistently; include a stable hash in proofs if size
    is a concern.
