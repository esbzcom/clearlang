# Collections (Runtime Semantics, Phase 17.3)

Purpose

- Provide deterministic runtime semantics for common collections.
- Keep diagnostics stable and AI-friendly with precise spans and codes.
- Keep rules small and prove-correct: each API has a clear typing judgment and runtime behavior.

Naming

- Modules: lower-case paths, e.g., `std::list`, `std::set`, `std::map`.
- Types: PascalCase with parameters, e.g., `List<T>`, `Set<T>`, `Map<K,V>`.
- Functions: lower-case verbs, e.g., `len`, `push`, `get`.

Types

- `List<T>`: sequence of `T`.
- `Set<T>`: unordered set of `T` (no duplicates, abstractly).
- `Map<K,V>`: mapping from `K` to `V`.

APIs and Typing Rules

List
- `std::list::len(l: List<T>) -> Int`
  - Rule: Ctx l : List<T> |- len(l) : Int
- `std::list::get(l: List<T>, i: Int) -> Option<T>`
  - Rule: Ctx l : List<T>, i : Int |- get(l,i) : Option<T>
- `std::list::push(l: List<T>, x: T) -> List<T>`
  - Rule: Ctx l : List<T>, x : T |- push(l,x) : List<T>
- `std::list::insert(l: List<T>, x: T, i: Int) -> List<T>`
  - Rule: Ctx l : List<T>, x : T, i : Int |- insert(l,x,i) : List<T>
- `std::list::remove(l: List<T>, i: Int) -> List<T>`
  - Rule: Ctx l : List<T>, i : Int |- remove(l,i) : List<T>
- `std::list::pop(l: List<T>) -> Option<T>`
  - Rule: Ctx l : List<T> |- pop(l) : Option<T>
- `std::list::new()`: requires explicit type arguments until inference improves (T206).

Set
- `std::set::len(s: Set<T>) -> Int`
  - Rule: Ctx s : Set<T> |- len(s) : Int
- `std::set::contains(s: Set<T>, x: T) -> Bool`
  - Rule: Ctx s : Set<T>, x : T |- contains(s,x) : Bool
- `std::set::insert(s: Set<T>, x: T) -> Set<T>`
  - Rule: Ctx s : Set<T>, x : T |- insert(s,x) : Set<T>
- `std::set::remove(s: Set<T>, x: T) -> Set<T>`
  - Rule: Ctx s : Set<T>, x : T |- remove(s,x) : Set<T>
- `std::set::new()`: requires explicit type arguments until inference improves (T206).

Map
- `std::map::len(m: Map<K,V>) -> Int`
  - Rule: Ctx m : Map<K,V> |- len(m) : Int
- `std::map::contains(m: Map<K,V>, k: K) -> Bool`
  - Rule: Ctx m : Map<K,V>, k : K |- contains(m,k) : Bool
- `std::map::get(m: Map<K,V>, k: K) -> Option<V>`
  - Rule: Ctx m : Map<K,V>, k : K |- get(m,k) : Option<V>
- `std::map::insert(m: Map<K,V>, k: K, v: V) -> Map<K,V>`
  - Rule: Ctx m : Map<K,V>, k : K, v : V |- insert(m,k,v) : Map<K,V>
- `std::map::remove(m: Map<K,V>, k: K) -> Map<K,V>`
  - Rule: Ctx m : Map<K,V>, k : K |- remove(m,k) : Map<K,V>
- `std::map::new()`: requires explicit type arguments until inference improves (T206).

Mutable Variants and Guards (Phase 6.4)
- Each mutating API now has a _mut variant (e.g., std::list::push_mut, std::set::remove_mut, std::map::insert_mut)
  that shares the same typing rule as its pure counterpart but requires the surrounding function to use the mut effect.
- Callers must declare a guard `require { std::<collection>::can_mut(var) }` for the first argument. These guard
  predicates are pure (Bool) and document aliasing expectations.
- Missing guards raise T402; guards whose first argument is not a variable raise T403. Pure callers still hit T401
  before guard checks.
- VC generation emits mut_pre obligations for each _mut call so proofs can reference the guard expression directly.
- Runtime lowering currently treats guard predicates as uninterpreted (they evaluate to true until the mutable runtime
  exists), keeping focus on the static proof story.

Runtime Semantics (Phase 17.3)

List
- `get` and `pop` are total (return `None` on out-of-bounds or empty).
- `insert` traps if `i < 0` or `i > len`; `remove` traps if `i < 0` or `i >= len` (runtime error `R009`).

Set
- `insert` is idempotent (no duplicates); `remove` is a no-op if missing.

Map
- `insert` replaces the existing value if the key is present; `remove` is a no-op if missing.

Key Equality (Map/Set)
- Keys must be equatable; non-equatable key types are rejected (T220).
- Equatable types: primitives, strings/bytes, Option/Result of equatable types, and structs/enums/tuples/arrays that
  only contain equatable types.
- Non-equatable: List/Set/Map/Resource and any type containing them.

Diagnostics (Stable Codes)

- `T206` cannot infer element type for `std::{list,set,map}::new()`
  - Example: `std::list::new()` -> T206 with span on call.
- `T207` expected collection kind
  - Example: `std::list::len(0)` -> T207 "expected List argument, found `Int`".
- `T208` element/key/value type mismatch
  - Example: `std::list::push(l: List<Int>, true)` -> T208 expecting `Int`, found `Bool`.
- `T220` non-equatable key type for `Map`/`Set`.
- Also applicable:
  - `T002` arity mismatch, `T005` index must be Int (for list insert/get/remove), `T003` arg type mismatch
    (non-collection cases).

AI-Friendly JSON Example

```
{
  "ok": false,
  "errors": [
    {
      "code": "T207",
      "stage": "type",
      "message": "at 42..56: expected List argument, found `Int`",
      "file": "sample.clear",
      "start": 42,
      "end": 56
    }
  ]
}
```

Design Notes (Prove Correct)

- All APIs are pure and total at the type level; runtime semantics are deterministic.
- Option/Result APIs avoid partial functions: out-of-bounds `get`/`pop` return `None` by construction in the type.
- Each rule preserves typing invariants by construction; later phases will add memory/runtime proofs.
