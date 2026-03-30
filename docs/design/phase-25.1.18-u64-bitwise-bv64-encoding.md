# Phase 25.1.18: U64 Bitwise BV64 Encoding

## Goal
Retire `bitwise.uninterpreted` for covered `U64` bitwise operators/intrinsics by emitting deterministic BV64 SMT formulas.

## Covered in this phase
- operators: `&`, `|`, `^`, `<<`, `>>`
- intrinsics: `std::u64::rotl`, `std::u64::rotr`

## Not covered in this phase
- bytes conversion intrinsics (`std::u64::{to_bytes_*,from_bytes_*}`) remain assumption-boundary surfaces.
- non-`U64` bitwise semantics remain outside coverage.

## Encoding summary
- `Int -> BV64`: `((_ int2bv 64) x)`
- `BV64 -> Int`: `clg.u64.to_int`
- shifts/rotates use masked shift counts (`& 63`) for deterministic 64-bit behavior.

## Result
- Covered paths no longer emit `bitwise.uninterpreted`.
- uncovered bitwise paths still emit `bitwise.uninterpreted` with explicit symbols.
