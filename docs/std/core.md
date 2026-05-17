# Package: `std::core`

## Purpose
Foundational language-level data and control abstractions used by all higher-level std packages.

## Key Types
- `Option<T>`
- `Result<T, E>`
- `ErrorCode`
- `CoreError`
- `Panic`

## Class/Method Draft

### `Option<T>`
Methods:
- `is_some(self) -> Bool`
- `is_none(self) -> Bool`
- `contains(self, value: T) -> Bool`
- `map<U>(self, f: function(T) -> U) -> Option<U>`
- `and_then<U>(self, f: function(T) -> Option<U>) -> Option<U>`
- `filter(self, f: function(T) -> Bool) -> Option<T>`
- `and<U>(self, other: Option<U>) -> Option<U>`
- `or(self, other: Option<T>) -> Option<T>`
- `or_else(self, f: function() -> Option<T>) -> Option<T>`
- `unwrap_or(self, default: T) -> T`
- `unwrap_or_else(self, f: function() -> T) -> T`
- `map_or<U>(self, default: U, f: function(T) -> U) -> U`
- `map_or_else<U>(self, default_f: function() -> U, f: function(T) -> U) -> U`
- `expect(self, code: ErrorCode) -> T`
- `flatten(self) -> Option<T>` (when `self` is `Option<Option<T>>`)
- `to_result<E>(self, err: E) -> Result<T, E>`
- `to_result_else<E>(self, err_f: function() -> E) -> Result<T, E>`

### `Result<T, E>`
Methods:
- `is_ok(self) -> Bool`
- `is_err(self) -> Bool`
- `contains(self, value: T) -> Bool`
- `contains_err(self, err: E) -> Bool`
- `map<U>(self, f: function(T) -> U) -> Result<U, E>`
- `map_err<F>(self, f: function(E) -> F) -> Result<T, F>`
- `and_then<U>(self, f: function(T) -> Result<U, E>) -> Result<U, E>`
- `and<U>(self, other: Result<U, E>) -> Result<U, E>`
- `or(self, other: Result<T, E>) -> Result<T, E>`
- `or_else(self, f: function(E) -> Result<T, E>) -> Result<T, E>`
- `unwrap_or(self, default: T) -> T`
- `unwrap_or_else(self, f: function(E) -> T) -> T`
- `map_or<U>(self, default: U, f: function(T) -> U) -> U`
- `map_or_else<U>(self, err_f: function(E) -> U, ok_f: function(T) -> U) -> U`
- `expect(self, code: ErrorCode) -> T`
- `expect_err(self, code: ErrorCode) -> E`
- `flatten(self) -> Result<T, E>` (when `self` is `Result<Result<T, E>, E>`)
- `to_option(self) -> Option<T>`

### `ErrorCode`
Methods:
- `new(code: Int) -> ErrorCode`
- `from_parts(domain: Int, code: Int) -> ErrorCode`
- `value(self) -> Int`
- `domain(self) -> Option<Int>`
- `equals(self, other: ErrorCode) -> Bool`

### `CoreError`
Methods:
- `new(code: ErrorCode) -> CoreError`
- `with_message(code: ErrorCode, message: String) -> CoreError`
- `with_cause(code: ErrorCode, cause: ErrorCode) -> CoreError`
- `code(self) -> ErrorCode`
- `message(self) -> Option<String>`
- `cause(self) -> Option<ErrorCode>`
- `equals(self, other: CoreError) -> Bool`

### `Panic`
Methods:
- `fail(code: ErrorCode) -> Int`

## First-Production Cut (recommended)
- Keep `Option<T>`: `is_*`, `map`, `and_then`, `filter`, `or_else`, `unwrap_or`, `unwrap_or_else`, `expect`, `to_result`.
- Keep `Result<T, E>`: `is_*`, `map`, `map_err`, `and_then`, `or_else`, `unwrap_or`, `unwrap_or_else`, `expect`, `expect_err`, `to_option`.
- Keep `ErrorCode`: `new`, `value`, `equals`.
- Keep `CoreError`: `new`, `with_message`, `code`, `message`, `equals`.
- Keep `Panic::fail`.
- Defer advanced helpers (`flatten`, `contains*`, `map_or*`, domain/cause fields) if they slow phase delivery.

## Notes
- `Option<T>` and `Result<T, E>` are language-level built-ins in current ClearLang; `std::core` documents the helper method surface around them.
- `Panic::fail` is intended as the single deterministic fail path for library helpers.
- This draft avoids new keywords and new syntax; all additions are library-level APIs.
- All methods above are deterministic by contract and intended to map to stable diagnostics and proof obligations.

## Real-World Scenario Coverage
- Validation pipeline: `Option::filter`, `Option::to_result`, `Result::map_err`.
- Service fallback/retry: `Option::or_else`, `Result::or_else`.
- Recovery defaults: `Option::unwrap_or_else`, `Result::unwrap_or_else`.
- Machine-readable errors: `ErrorCode` and `CoreError`.
- Fail-closed terminal path: `Panic::fail`.
- Cross-layer error translation: `Result::map_err`, `CoreError::with_message`, `CoreError::with_cause`.
- Normalization to scalar values: `Option::map_or_else`, `Result::map_or_else`.

## Summary
- Defines deterministic semantics for optional/error-bearing control flow.
- Serves as the base for proof obligations used by contracts and verification tooling.
- Must stay minimal, stable, and dependency-light.
