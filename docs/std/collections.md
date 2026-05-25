# Catalog: Collections (`std::list`, `std::set`, `std::map`)

## Purpose
Deterministic collection APIs for list/set/map with proof-friendly contracts and stable behavior.

## Import Paths
- There is no concrete `std::collections` module path.
- Use concrete module paths:
  - `std::list`
  - `std::set`
  - `std::map`

## Sub-Namespaces
- `list`
- `set`
- `map`
- `collection_error`

## Types
- `List<T>` (built-in)
- `Set<T>` (built-in)
- `Map<K, V>` (built-in)
- `CollectionError`

## API Style (first production)
- Keep collection operations as module namespace functions:
  - `std::list::len(l: List<T>) -> Int`
  - `std::set::contains(s: Set<T>, value: T) -> Bool`
  - `std::map::get(m: Map<K, V>, key: K) -> Option<V>`
- `List<T>`/`Set<T>`/`Map<K, V>` are std types; current language surface does not require member-call syntax.
- Dot-call (`l.len()`) can be added later as syntax sugar only (`l.len()` -> `std::list::len(l)`), with no runtime model change.

## Type/Function Draft

### `list`
Functions in `std::list`:
- `new() -> List<T>`
- `len(l: List<T>) -> Int`
- `is_empty(l: List<T>) -> Bool`
- `get(l: List<T>, index: Int) -> Option<T>`
- `push(l: List<T>, value: T) -> List<T>`
- `insert(l: List<T>, value: T, index: Int) -> List<T>`
- `insert_checked(l: List<T>, value: T, index: Int) -> Result<List<T>, CollectionError>`
- `remove(l: List<T>, index: Int) -> List<T>`
- `remove_checked(l: List<T>, index: Int) -> Result<List<T>, CollectionError>`
- `remove_take(l: List<T>, index: Int) -> (List<T>, Option<T>)`
- `pop(l: List<T>) -> Option<T>`

### `set`
Functions in `std::set`:
- `new() -> Set<T>`
- `len(s: Set<T>) -> Int`
- `is_empty(s: Set<T>) -> Bool`
- `contains(s: Set<T>, value: T) -> Bool`
- `insert(s: Set<T>, value: T) -> Set<T>`
- `remove(s: Set<T>, value: T) -> Set<T>`
- `subset(s: Set<T>, other: Set<T>) -> Bool`
- `union(s: Set<T>, other: Set<T>) -> Set<T>`
- `intersect(s: Set<T>, other: Set<T>) -> Set<T>`
- `diff(s: Set<T>, other: Set<T>) -> Set<T>`

### `map`
Functions in `std::map`:
- `new() -> Map<K, V>`
- `len(m: Map<K, V>) -> Int`
- `is_empty(m: Map<K, V>) -> Bool`
- `contains(m: Map<K, V>, key: K) -> Bool`
- `get(m: Map<K, V>, key: K) -> Option<V>`
- `insert(m: Map<K, V>, key: K, value: V) -> Map<K, V>`
- `insert_take(m: Map<K, V>, key: K, value: V) -> (Map<K, V>, Option<V>)`
- `remove(m: Map<K, V>, key: K) -> Map<K, V>`
- `remove_take(m: Map<K, V>, key: K) -> (Map<K, V>, Option<V>)`

### `collection_error`
Functions:
- `code(err: CollectionError) -> ErrorCode`
- `equals(a: CollectionError, other: CollectionError) -> Bool`

## First-Production Cut (recommended)
- Keep `List<T>`: `new`, `len`, `get`, `push`, `insert`, `insert_checked`, `remove`, `remove_checked`, `remove_take`, `pop`.
- Keep `Set<T>`: `new`, `len`, `contains`, `insert`, `remove`, `subset`.
- Keep `Map<K, V>`: `new`, `len`, `contains`, `get`, `insert`, `insert_take`, `remove`, `remove_take`.
- Keep guarded mutable compatibility surfaces used by current code/tests:
  - `std::list::{can_mut,push_mut,insert_mut,remove_mut,pop_mut}`
  - `std::set::{can_mut,insert_mut,remove_mut}`
  - `std::map::{can_mut,insert_mut,remove_mut}`
- Defer heavy set algebra/cardinality proof features if they delay launch.

## Notes
- This file is an umbrella catalog for collection modules, not a direct import path.
- Collection APIs are immutable-return style in first production cut.
- Proof roadmap order remains `subset -> set ops -> cardinality-heavy reasoning`.
- `insert` and `remove` are fail-closed convenience APIs; out-of-range indices MUST terminate deterministically via the standard panic/failure path.
- `insert_checked` and `remove_checked` are the preferred non-terminating APIs for user-facing/business workflows.
- Set/map behavior that affects hashing/signing workflows MUST define deterministic canonical ordering before use in canonical serialization.

## Security Considerations
- Collections used in security-critical hashing/signing pipelines SHOULD avoid unspecified iteration/order semantics.
- Checked variants SHOULD be preferred where caller-facing error handling is required.

## Contract Conformance Checklist
- `new` inference failure behavior MUST remain deterministic (`T206`-style contract).
- Index-based list operations MUST validate bounds deterministically.
- `subset`/set algebra outputs MUST be deterministic for identical logical inputs.
- `Map`/`Set` key equality requirements MUST be explicit and stable.

## Summary
- Provides predictable list/set/map operations for production-safe logic.
- Aligns with deterministic and proof-oriented Phase 26 runtime gates.


