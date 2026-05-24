# Std Coverage Matrix (Phase 26)

Status keys:
- `typed`: typer + lowering coverage
- `runtime`: codegen/runtime execution coverage
- `proved`: theorem-grade proof coverage in release policy

Allowed values:
- `yes`
- `no`
- `deferred`

## `std::str`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::str::len` | yes | yes | no | Release-enabled API; proof coverage pending Gate B/C integration. |
| `std::str::is_empty` | yes | yes | no | Lowered via `len == 0`; proof status follows `len`. |
| `std::str::equals` | yes | yes | no | Alias of `std::str::eq`; theorem-grade closure pending. |
| `std::str::concat` | yes | yes | no | Runtime string model coverage exists; proof closure pending. |
| `std::str::starts_with` | yes | yes | no | Implemented intrinsic; proof modeling pending. |
| `std::str::ends_with` | yes | yes | no | Implemented intrinsic; proof modeling pending. |
| `std::str::contains` | yes | yes | no | Implemented intrinsic; proof modeling pending. |
| `std::str::to_bytes` | yes | yes | no | Lowered to `std::bytes::from_string`; proof closure pending. |
| `std::str::slice` | no | no | deferred | Deferred from first-production release cut. |
| `std::str::trim` | no | no | deferred | Deferred from first-production release cut. |
| `std::str_pattern::matches` | no | no | deferred | Locked for API stability in 26.1.4.1; implementation pending. |
