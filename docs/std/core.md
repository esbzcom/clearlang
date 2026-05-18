# Namespace: `std::core`

## Purpose
Foundational language-level data and control abstractions used by all higher-level std namespaces.

## Sub-Namespaces
- `option`
- `result`
- `error_code`
- `core_error`
- `panic`

## Types
- `Option<T>` (built-in)
- `Result<T, E>` (built-in)
- `ErrorCode`
- `CoreError`

## Type/Function Draft

### `option`
Functions:
- `is_some(value: Option<T>) -> Bool`
- `is_none(value: Option<T>) -> Bool`
- `contains(opt: Option<T>, value: T) -> Bool` (requires equality capability for `T`)
- `map<U>(opt: Option<T>, f: function(T) -> U) -> Option<U>`
- `and_then<U>(opt: Option<T>, f: function(T) -> Option<U>) -> Option<U>`
- `filter(opt: Option<T>, f: function(T) -> Bool) -> Option<T>`
- `and<U>(opt: Option<T>, other: Option<U>) -> Option<U>`
- `or(opt: Option<T>, other: Option<T>) -> Option<T>`
- `or_else(opt: Option<T>, f: function() -> Option<T>) -> Option<T>`
- `unwrap_or(opt: Option<T>, default: T) -> T`
- `unwrap_or_else(opt: Option<T>, f: function() -> T) -> T`
- `map_or<U>(opt: Option<T>, default: U, f: function(T) -> U) -> U`
- `map_or_else<U>(opt: Option<T>, default_f: function() -> U, f: function(T) -> U) -> U`
- `expect(opt: Option<T>, code: ErrorCode) -> T`
- `flatten(opt: Option<Option<T>>) -> Option<T>`
- `to_result<E>(opt: Option<T>, err: E) -> Result<T, E>`
- `to_result_else<E>(opt: Option<T>, err_f: function() -> E) -> Result<T, E>`

### `result`
Functions:
- `is_ok(value: Result<T, E>) -> Bool`
- `is_err(value: Result<T, E>) -> Bool`
- `contains(res: Result<T, E>, value: T) -> Bool` (requires equality capability for `T`)
- `contains_err(res: Result<T, E>, err: E) -> Bool` (requires equality capability for `E`)
- `map<U>(res: Result<T, E>, f: function(T) -> U) -> Result<U, E>`
- `map_err<F>(res: Result<T, E>, f: function(E) -> F) -> Result<T, F>`
- `and_then<U>(res: Result<T, E>, f: function(T) -> Result<U, E>) -> Result<U, E>`
- `and<U>(res: Result<T, E>, other: Result<U, E>) -> Result<U, E>`
- `or(res: Result<T, E>, other: Result<T, E>) -> Result<T, E>`
- `or_else(res: Result<T, E>, f: function(E) -> Result<T, E>) -> Result<T, E>`
- `unwrap_or(res: Result<T, E>, default: T) -> T`
- `unwrap_or_else(res: Result<T, E>, f: function(E) -> T) -> T`
- `map_or<U>(res: Result<T, E>, default: U, f: function(T) -> U) -> U`
- `map_or_else<U>(res: Result<T, E>, err_f: function(E) -> U, ok_f: function(T) -> U) -> U`
- `expect(res: Result<T, E>, code: ErrorCode) -> T`
- `expect_err(res: Result<T, E>, code: ErrorCode) -> E`
- `flatten(res: Result<Result<T, E>, E>) -> Result<T, E>`
- `to_option(res: Result<T, E>) -> Option<T>`

### `error_code`
Functions:
- `new(code: Int) -> ErrorCode`
- `from_parts(domain: Int, code: Int) -> ErrorCode`
- `value(code: ErrorCode) -> Int`
- `domain(code: ErrorCode) -> Option<Int>`
- `equals(a: ErrorCode, other: ErrorCode) -> Bool`

### `core_error`
Functions:
- `new(code: ErrorCode) -> CoreError`
- `with_message(code: ErrorCode, message: String) -> CoreError`
- `with_cause(code: ErrorCode, cause: ErrorCode) -> CoreError`
- `code(err: CoreError) -> ErrorCode`
- `message(err: CoreError) -> Option<String>`
- `cause(err: CoreError) -> Option<ErrorCode>`
- `equals(a: CoreError, other: CoreError) -> Bool`

### `panic`
Functions:
- `fail(code: ErrorCode) -> Int` (terminal fail path; does not return on success path)

## First-Production Cut (recommended)
- Keep `Option<T>`: `is_*`, `map`, `and_then`, `filter`, `or_else`, `unwrap_or`, `unwrap_or_else`, `expect`, `to_result`.
- Keep `Result<T, E>`: `is_*`, `map`, `map_err`, `and_then`, `or_else`, `unwrap_or`, `unwrap_or_else`, `expect`, `expect_err`, `to_option`.
- Keep `ErrorCode`: `new`, `value`, `equals`.
- Keep `CoreError`: `new`, `with_message`, `code`, `message`, `equals`.
- Keep `panic::fail`.
- Defer advanced helpers (`flatten`, `contains*`, `map_or*`, domain/cause fields) if they slow phase delivery.

## Notes
- `Option<T>` and `Result<T, E>` are language-level built-ins in current ClearLang; `std::core` documents the helper function surface around them.
- `option`/`result` in this file are logical API groups over built-ins, not separate importable modules.
- `panic::fail` is intended as the single deterministic fail path for library helpers.
- `panic::fail` MUST always terminate execution with deterministic diagnostics; `Int` is a surface placeholder, not a recoverable value.
- This draft avoids new keywords and new syntax; all additions are library-level APIs.
- All functions above MUST be deterministic and map to stable diagnostics and proof obligations.
- `contains*` helpers are enabled only for equality-capable types; non-equatable usage MUST fail with deterministic diagnostics.

## Error Taxonomy Policy
- `ErrorCode` domains MUST be globally non-overlapping across std packages.
- `ErrorCode` values MUST remain stable across patch/minor releases once locked.
- `CoreError` wrapping (`with_message`, `with_cause`) MUST NOT lose the primary `ErrorCode`.
- Contract/domain-level code registries SHOULD reserve contiguous ranges per package.

## Contract Conformance Checklist
- `option` and `result` helper behavior MUST be total for all input variants.
- `expect`/`expect_err` MUST fail via deterministic terminal path with stable code mapping.
- `panic::fail` MUST NOT return to caller under any success-path execution.
- Equality-dependent helpers (`contains*`) MUST reject unsupported type capabilities deterministically.

## Real-World Scenario Coverage
- Validation pipeline: `option::filter`, `option::to_result`, `result::map_err`.
- Service fallback/retry: `option::or_else`, `result::or_else`.
- Recovery defaults: `option::unwrap_or_else`, `result::unwrap_or_else`.
- Machine-readable errors: `error_code` and `core_error`.
- Fail-closed terminal path: `panic::fail`.
- Cross-layer error translation: `result::map_err`, `core_error::with_message`, `core_error::with_cause`.
- Normalization to scalar values: `option::map_or_else`, `result::map_or_else`.

## Summary
- Defines deterministic semantics for optional/error-bearing control flow.
- Serves as the base for proof obligations used by contracts and verification tooling.
- Must stay minimal, stable, and dependency-light.


