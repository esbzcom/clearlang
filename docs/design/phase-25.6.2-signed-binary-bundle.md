# Phase 25.6.2 - Signed Binary Bundle Contract

## Status
Implemented deterministic signed binary bundle generation for milestone_3 GA baseline targets.

## Contract
`cargo run -p xtask -- milestone3-binary-bundle` now emits a deterministic platform bundle containing:
- `bin/<platform>/clg(.exe)`
- `checksums/SHA256SUMS` (sorted, detached checksums)
- `metadata/milestone3-binary-bundle.signed.json` (signed metadata payload)
- `metadata/milestone3-binary-bundle.pubkey.json` (matching verification key material)
- `sbom/*` supply-chain evidence (`milestone2-*` reports)
- `licenses/clearlang-LICENSE.txt`
- `licenses/third-party-licenses.json`

`cargo run -p xtask -- milestone3-binary-bundle-verify --bundle-dir <DIR>` verifies:
- signed payload hash and Ed25519 signature (against emitted pubkey metadata),
- checksum manifest formatting/order,
- per-artifact SHA-256 integrity for all listed members.

Signing policy:
- Ed25519 key seed is provided via `CLG_BINARY_RELEASE_SIGNING_KEY_HEX`.
- Empty/missing/invalid key material fails closed.
- Signature payload hash is deterministic and bound to binary + checksum/SBOM/license artifact hashes.

## Determinism Requirements
- Output directory is recreated on each run.
- Relative path normalization uses forward slashes.
- Checksum manifest ordering is lexicographic.
- Signed payload fields and artifact arrays are deterministic.

## References
- `xtask/src/main/core/dispatch_and_drift.rs`
- `.github/workflows/ci.yml`
- `docs/release-process.md`
