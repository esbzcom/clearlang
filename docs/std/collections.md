# Package: `std::collections`

## Purpose
Core deterministic container APIs with proof-friendly contracts and stable behavior.

## Key Types
- `List<T>`
- `Set<T>`
- `Map<K, V>`
- `CollectionError`

## Class/Method Draft

### `List<T>`
Methods:
- `new() -> List<T>`
- `len(self) -> Int`
- `is_empty(self) -> Bool`
- `get(self, index: Int) -> Option<T>`
- `push(self, value: T) -> List<T>`
- `insert(self, value: T, index: Int) -> List<T>`
- `remove(self, index: Int) -> List<T>`
- `remove_take(self, index: Int) -> (List<T>, Option<T>)`
- `pop(self) -> Option<T>`

### `Set<T>`
Methods:
- `new() -> Set<T>`
- `len(self) -> Int`
- `is_empty(self) -> Bool`
- `contains(self, value: T) -> Bool`
- `insert(self, value: T) -> Set<T>`
- `remove(self, value: T) -> Set<T>`
- `subset(self, other: Set<T>) -> Bool`
- `union(self, other: Set<T>) -> Set<T>`
- `intersect(self, other: Set<T>) -> Set<T>`
- `diff(self, other: Set<T>) -> Set<T>`

### `Map<K, V>`
Methods:
- `new() -> Map<K, V>`
- `len(self) -> Int`
- `is_empty(self) -> Bool`
- `contains(self, key: K) -> Bool`
- `get(self, key: K) -> Option<V>`
- `insert(self, key: K, value: V) -> Map<K, V>`
- `insert_take(self, key: K, value: V) -> (Map<K, V>, Option<V>)`
- `remove(self, key: K) -> Map<K, V>`
- `remove_take(self, key: K) -> (Map<K, V>, Option<V>)`

### `CollectionError`
Methods:
- `code(self) -> ErrorCode`
- `equals(self, other: CollectionError) -> Bool`

## First-Production Cut (recommended)
- Keep `List<T>`: `new`, `len`, `get`, `push`, `insert`, `remove`, `remove_take`, `pop`.
- Keep `Set<T>`: `new`, `len`, `contains`, `insert`, `remove`, `subset`.
- Keep `Map<K, V>`: `new`, `len`, `contains`, `get`, `insert`, `insert_take`, `remove`, `remove_take`.
- Defer heavy set algebra/cardinality proof features if they delay launch.

## Notes
- Collection APIs are expected to stay immutable-return style in first production cut.
- Proof roadmap order should remain `subset -> set ops -> cardinality-heavy reasoning`.

## Summary
- Provides bounded, predictable APIs for list/set/map operations.
- Primary proof roadmap focus includes bounds safety, subset/membership, and invariants.
- Designed to integrate tightly with VC/SMT obligations and fail-closed diagnostics.
