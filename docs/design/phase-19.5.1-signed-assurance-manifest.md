# Phase 19.5.1 - Signed Assurance Manifest Per Build

## Status
Design lock for `19.5.1` in `docs/TODO.md`.

## Goal
Emit a deterministic signed assurance manifest on signed builds so audit/release workflows can consume one artifact with assurance level, assumptions, dependency trust labels, and toolchain fingerprint metadata.

## Design Principles Check
- Simple for users: signed builds emit a default assurance manifest artifact without extra required flags.
- AI-friendly: manifest schema is stable and machine-readable (`clg.assurance_manifest.v1` + `schema_version = 1`).
- Provably correct: manifest is signed over canonical JSON payload and includes exact module/proofs hashes tied to emitted artifacts.
- Crypto-focused: dependency trust labels and assumption boundaries are explicit for security/audit pipelines.

## Scope
1. Add build output behavior:
   - `clg build --sign ...` emits signed assurance manifest.
   - default path derived from `--sig-out` (`*.sig.json` -> `*.assurance.json`).
   - optional override via `--assurance-manifest-out <FILE>`.
2. Add manifest payload fields:
   - assurance tier/levels,
   - assumptions summary (`total`, `items`, VC refs),
   - dependency trust labels for `primitive`/`external` assumed surfaces,
   - toolchain name + deterministic fingerprint,
   - compiler mode / proof strictness / module+proof hashes.
3. Sign manifest payload with existing Ed25519 key flow and emit signature metadata (`key_id`, `payload_hash`, `signature`).
4. Add CLI tests for:
   - manifest emission + signature verifiability,
   - invalid flag combination diagnostic (`C034`).

## Locked Behavior
1. Manifest envelope:
   - `schema_version = 1`,
   - `payload` (assurance manifest data),
   - `signature` (Ed25519 over canonical payload JSON).
2. `--assurance-manifest-out` without `--sign` fails with deterministic build diagnostic `C034`.
3. Manifest generation does not alter existing signature/verify semantics for `out.sig.json`.

## Non-Goals
1. No policy enforcement on required assurance tier in this slice (`19.5.3`).
2. No human-oriented explain mode in verify flow (`19.5.2`).
3. No changes to runtime verify kernel/trust-anchor execution model.

## Exit Criteria for 19.5.1
1. Signed builds emit signed assurance manifest artifact with deterministic schema.
2. Integration tests cover manifest signature validity and `C034`.
3. TODO/rollout focus advances to `19.5.2`.

## References
- `docs/TODO.md`
- `docs/rollout/DEVPLAN.md`
- `docs/proofs/proof-section.md`
- `docs/diagnostics.md`
- `crates/cli/src/commands/build.rs`
- `crates/cli/src/proofs.rs`
- `crates/cli/src/signing.rs`
