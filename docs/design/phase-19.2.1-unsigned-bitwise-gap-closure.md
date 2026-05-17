# Phase 19.2.1 - Unsigned/Bitwise SMT Gap Closure (Downgrade Path)

## Status
Design lock for `19.2.1` in `docs/TODO.md`.
This slice closes trust gaps by making unsigned/bitwise modeling limits explicit in emitted assumptions when full proof semantics are not yet implemented.

## Goal
Ensure no hidden unsigned/bitwise proof claims remain for the current supported surface (`U128`/`U256` representation helpers and bitwise shift/mask-style `std::u64` intrinsics).

## Design Principles Check
- Simple for users: affected proof limits fail-open only through explicit `assumed` boundaries with concrete symbols.
- AI-friendly: boundaries remain deterministic (`unsigned.int_model`, `bitwise.uninterpreted`) with stable symbol labels for tooling.
- Provably correct: unsupported or partially modeled semantics are downgraded to explicit assumptions rather than implied as proved.
- Crypto-focused: bit-level operations used in crypto-adjacent logic are clearly marked as trust boundaries in VC/proof artifacts.

## Scope
1. Keep existing unsigned boundary coverage for `U8`/`U64`/`U128`/`U256` modeling (`unsigned.int_model`).
2. Explicitly tag bitwise-sensitive `std::u64` intrinsics under `bitwise.uninterpreted`:
   - `std::u64::rotl`
   - `std::u64::rotr`
   - `std::u64::to_bytes_le`
   - `std::u64::to_bytes_be`
   - `std::u64::from_bytes_le`
   - `std::u64::from_bytes_be`
3. Preserve existing bitwise operator tagging (`&`, `|`, `^`, `<<`, `>>`).

## Locked Behavior
1. VC assumption output must include `bitwise.uninterpreted` when any bitwise operator or listed bitwise-sensitive intrinsic is present.
2. Symbols for bitwise assumptions must include exact touched operators/intrinsics.
3. Proof-coverage matrix stays `assumed`/`blocked` for these surfaces until proof-complete modeling lands.

## Non-Goals
1. No new SMT proof semantics for full bit-precise arithmetic in this slice.
2. No change to language typing support for unsupported `U128`/`U256` arithmetic operators.
3. No release-tier policy changes beyond clearer assumption downgrade coverage.

## Exit Criteria for 19.2.1
1. Bitwise-sensitive `std::u64` intrinsics are explicitly downgraded via `bitwise.uninterpreted`.
2. Regression tests verify deterministic assumption emission and symbol labeling.
3. TODO/proof-coverage docs reflect the updated downgrade model.

## References
- `docs/TODO.md`
- `docs/TODO.md`
- `docs/proofs/vc-schema.md`
- `docs/proofs/proof-coverage-matrix.md`
- `docs/proofs/proof-coverage-matrix.json`
