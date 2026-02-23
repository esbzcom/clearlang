# Proof Coverage Matrix

Status: Phase `18.0.6.3` publication baseline.

This matrix records proof coverage as `proved` vs `assumed` and maps each entry to assurance tiers `L0`-`L3` from `docs/design/phase-18.0-open-questions.md`.

Machine-readable source of truth:
- `docs/proofs/proof-coverage-matrix.json`

Strict-mode default:
- `clg build --emit-vcs` runs strict assumption-boundary checks by default (`--proof-strict=true`).
- Strict checks validate structured `assumptions.items` metadata, not SMT text substrings.

Interpretation:
- `proved`: VC generation/encoding coverage is implemented for this surface.
- `assumed`: the surface depends on an explicit assumption boundary.
- `blocked` at `L3`: strict package-level assurance is not valid if the surface is present.

Current assumption boundaries:
- `unsigned.int_model`
- `bitwise.uninterpreted`
- `crypto.uninterpreted`

## Feature Coverage

| Entry | Status | L0 | L1 | L2 | L3 |
| --- | --- | --- | --- | --- | --- |
| `feature.contract_implication_core` | proved | proved | proved | proved | proved |
| `feature.refinement_premises` | proved | proved | proved | proved | proved |
| `feature.loop_obligations` | proved | proved | proved | proved | proved |
| `feature.linear_collection_control_flow` | proved | proved | proved | proved | proved |
| `feature.unsigned_integer_model` | assumed (`unsigned.int_model`) | assumed | assumed | assumed | blocked |
| `feature.bitwise_shift_operators` | assumed (`bitwise.uninterpreted`) | assumed | assumed | assumed | blocked |
| `feature.crypto_intrinsics_model` | assumed (`crypto.uninterpreted`) | assumed | assumed | assumed | blocked |

## Intrinsic Coverage

Per-intrinsic rows live in `docs/proofs/proof-coverage-matrix.json` under ids:
- `intrinsic.std::bytes::eq_ct`
- `intrinsic.std::crypto::{hash,hmac,verify}`
- `intrinsic.std::u64::{add_wrap,sub_wrap,mul_wrap,add_sat,sub_sat,mul_sat,rotl,rotr,to_bytes_le,to_bytes_be,from_bytes_le,from_bytes_be}`
- `intrinsic.std::u128::{from_limbs,lo,hi}`
- `intrinsic.std::u256::{from_limbs,limb0,limb1,limb2,limb3}`

All listed intrinsic entries are currently `assumed` and therefore `blocked` at `L3`.
Bitwise-sensitive `std::u64` intrinsics (`rotl`, `rotr`, `to_bytes_*`, `from_bytes_*`) are explicitly downgraded under `bitwise.uninterpreted`.
