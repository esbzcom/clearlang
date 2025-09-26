# Collections (Type-Only, Phase 4.6-"4.7)

Purpose
- Provide type-only APIs for common collections to enable composition and testing before runtime support.
- Keep diagnostics stable and AI-friendly with precise spans and codes.
- Keep rules small and prove-correct: each API has a clear typing judgment.

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
  - Rule: " S l : List<T> #' " S len(l) : Int
- `std::list::get(l: List<T>, i: Int) -> Option<T>`
  - Rule: " S l : List<T>, " S i : Int #' " S get(l,i) : Option<T>
- `std::list::push(l: List<T>, x: T) -> List<T>`
  - Rule: " S l : List<T>, " S x : T #' " S push(l,x) : List<T>
- `std::list::insert(l: List<T>, x: T, i: Int) -> List<T>`
  - Rule: " S l : List<T>, " S x : T, " S i : Int #' " S insert(l,x,i) : List<T>
- `std::list::remove(l: List<T>, i: Int) -> List<T>`
  - Rule: " S l : List<T>, " S i : Int #' " S remove(l,i) : List<T>
- `std::list::pop(l: List<T>) -> Option<T>`
  - Rule: " S l : List<T> #' " S pop(l) : Option<T>
- `std::list::new()`: not yet supported -" requires type inference; see T206.

Set
- `std::set::len(s: Set<T>) -> Int`
  - Rule: " S s : Set<T> #' " S len(s) : Int
- `std::set::contains(s: Set<T>, x: T) -> Bool`
  - Rule: " S s : Set<T>, " S x : T #' " S contains(s,x) : Bool
- `std::set::insert(s: Set<T>, x: T) -> Set<T>`
  - Rule: " S s : Set<T>, " S x : T #' " S insert(s,x) : Set<T>
- `std::set::remove(s: Set<T>, x: T) -> Set<T>`
  - Rule: " S s : Set<T>, " S x : T #' " S remove(s,x) : Set<T>
- `std::set::new()`: not yet supported -" requires type inference; see T206.

Map
- `std::map::len(m: Map<K,V>) -> Int`
  - Rule: " S m : Map<K,V> #' " S len(m) : Int
- `std::map::contains(m: Map<K,V>, k: K) -> Bool`
  - Rule: " S m : Map<K,V>, " S k : K #' " S contains(m,k) : Bool
- `std::map::get(m: Map<K,V>, k: K) -> Option<V>`
  - Rule: " S m : Map<K,V>, " S k : K #' " S get(m,k) : Option<V>
- `std::map::insert(m: Map<K,V>, k: K, v: V) -> Map<K,V>`
  - Rule: " S m : Map<K,V>, " S k : K, " S v : V #' " S insert(m,k,v) : Map<K,V>
- `std::map::remove(m: Map<K,V>, k: K) -> Map<K,V>`
  - Rule: " S m : Map<K,V>, " S k : K #' " S remove(m,k) : Map<K,V>
- `std::map::new()`: not yet supported -" requires type inference; see T206.


Mutable Variants & Guards (Phase 6.4)
- Each mutating API now has a _mut variant (e.g., std::list::push_mut, std::set::remove_mut, std::map::insert_mut) that shares the same typing rule as its pure counterpart but requires the surrounding function to use the mut effect.
- Callers must declare a guard equire { std::<collection>::can_mut(var) } for the first argument. These guard predicates are pure (Bool) and document aliasing expectations.
- Missing guards raise T402; guards whose first argument is not a variable raise T403. Pure callers still hit T401 before guard checks.
- VC generation emits mut_pre obligations for each _mut call so proofs can reference the guard expression directly.
- Runtime lowering currently treats guard predicates as uninterpreted (they evaluate to 	rue until the mutable runtime exists), keeping focus on the static proof story.
Diagnostics (Stable Codes)
- `T206` cannot infer element type for `std::{list,set,map}::new()`
  - Example: `std::list::new()` +' T206 with span on call.
- `T207` expected collection kind
  - Example: `std::list::len(0)` +' T207 "expected List argument, found `Int`".
- `T208` element/key/value type mismatch
  - Example: `std::list::push(l: List<Int>, true)` +' T208 expecting `Int`, found `Bool`.
- Also applicable:
  - `T002` arity mismatch, `T005` index must be Int (for list insert/get/remove), `T003` arg type mismatch (non-collection cases).

AI-'Friendly JSON Example
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
- All APIs are pure and total at the type level; effects and runtime semantics arrive later.
- Option/Result APIs avoid partial functions: out-of-bounds `get`/`pop` return `None` by construction in the type.
- Each rule preserves typing invariants by construction; later phases will add memory/runtime proofs.




