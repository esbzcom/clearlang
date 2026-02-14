# Phase 17.2 - Generics and Interfaces

## Status
- Design note only. Implementation tracked in `docs/TODO.md` under Phase 17.2.

## Goals
- Add type parameters to `struct`, `enum`, `function`, and `type` aliases.
- Introduce interfaces with static dispatch and interface bounds.
- Enable generic stdlib collections (`List<T>`, `Map<K,V>`, `Set<T>`) and shared helpers.
- Keep compilation deterministic and simple via monomorphization.

## Non-Goals
- Interface objects or dynamic dispatch.
- Specialization, negative implementations, or overlapping implementations.
- Higher-kinded types, higher-rank polymorphism, or generic associated types.
- Default interface method bodies (possible later extension).
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

Interfaces and implementations (static dispatch):
```
interface Eq {
    pure function eq(a: Self, b: Self) -> Bool;
}

implementation Eq for Int {
    pure function eq(a: Int, b: Int) -> Bool { a == b }
}
```

Interface bounds (inline and `where`):
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
- Interface methods are declared as signatures only; no default bodies in this phase.
- `Self` is only valid inside interface and implementation declarations.
- Interface methods are called with `Interface::method(...)` (no dot-method syntax yet).
- Type arguments use `Name<T, U>` and `call<T>(...)` syntax.

### Bounds Placement and Syntax (Draft)

- Inline bounds are allowed in the type parameter list:
  - `function f<T: Eq>(x: T) -> Bool { ... }`
- A `where` clause appears after the return type and before any `require`/`ensure`
  clauses and the function body.
- Multiple bounds are comma-separated. If multiple interfaces are required for a
  single parameter, repeat the parameter name:
  - `where T: Eq, T: Hash`

## Typing and Inference

- Generic parameters are scoped to the item they are declared on.
- Type arguments are inferred from:
  - Function arguments.
  - Expected type (contextual return type).
- If inference is ambiguous or unused type parameters remain, emit a diagnostic
  and require explicit type arguments.
- Interface bounds are not inferred; they must appear inline or in a `where` clause.
- Interface bounds are required when calling an interface method; bounds are checked
  when monomorphizing each call site.

### Effects and Contracts
- Interface method signatures include effects; implementations must match exactly.
- `require`/`ensure` clauses on generic functions are preserved per instantiation.
- Pure/Mut/Io rules apply after monomorphization (no special-casing for generics).

### Resources and Refinements
- Type arguments may be refined aliases; refinement obligations are preserved
  after substitution.
- Resource types are allowed as type parameters, but existing restrictions still
  apply (e.g., `List<Resource>` remains rejected until Phase 17.6).

## Coherence Rules (Draft)

- For each `(Interface, ConcreteType)` pair, there must be exactly one applicable
  `implementation` after type substitution.
- Overlapping implementations are rejected, even if one is "more specific."
- There is no orphan rule yet (no module system); this will be revisited in
  Phase 17.5 to avoid cross-module conflicts.

## Monomorphization Model

- All generics are monomorphized at compile time into concrete copies.
- Interface calls are resolved to the concrete `implementation` function at compile time.
- No vtables or dynamic dispatch are generated.
- VCs and proofs are generated per monomorphized instance only (uninstantiated
  generics produce no proof artifacts).

## Name Mangling (Identifier-Safe)

All mangled names use only `[A-Za-z0-9_$]` to avoid downstream identifier
parsing issues. The scheme is canonical and deterministic.
The `$` character is reserved for mangling and is not allowed in user-defined
identifiers.

- Function instantiations: `name$T1$T2$...` (no trailing `$`).
- Implementation method instantiations: `impl$Trait$Self$method`.
- Optional shortening mode (Phase 17.8.3.1): when `CLG_MANGLE_MAX_LEN` is set,
  long mangled function/implementation names are truncated deterministically with
  an `$h<fnv64-hex>` suffix.
  - Collision safety (Phase 17.8.3.2): if two canonical names map to the same
    emitted shortened name, type-checking fails deterministically (`T250`).
  - Proof/debug traceability (Phase 17.8.3.3): emitted artifacts carry optional
    canonical-name mapping fields so tools can recover the unshortened symbol.
- Type mangling:
  - Primitives: `Int`, `U8`, `U64`, `U128`, `U256`, `Bool`, `String`, `Bytes`.
  - Named: `N$Name$arity$Arg1$Arg2$...` (arity included for parseability).
  - Option: `Option$T`
  - Result: `Result$Ok$Err`
  - List/Set: `List$T`, `Set$T`
  - Map: `Map$K$V`
  - Array: `Array$len$T`
  - Tuple: `Tuple$arity$T1$T2$...`

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
- Missing or unsatisfied interface bounds.
- Overlapping or duplicate implementations.

## Open Questions

- Whether to allow default interface method bodies (and how to check effects).
  - Recommended: defer; if added later, require explicit effect on the default
    body and enforce exact effect matching on overrides.
- Syntax for explicitly selecting an implementation when multiple could match (if ever
  allowed).
  - Recommended: avoid; keep coherence strict so selection is never required.
- How to serialize generic instantiations in debug names and proof metadata.
  - Resolved: use the identifier-safe mangling scheme above; preserve refined
    alias names to avoid collapsing proof obligations.

## Decisions (Phase 17.2)

- Default interface method bodies: deferred.
- Explicit implementation selection: not supported.
- Debug/proof names: identifier-safe mangling as documented above.
