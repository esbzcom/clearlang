# Phase 25.6.10 - Release Bundle Provenance Contract

## Status
Implemented provenance contract for release bundles and `clg verify-bundle`.

## Contract
`clg release` emits a signed provenance artifact:
- `<stem>.provenance.json`
- Included in `<stem>.release-bundle.json` under `artifacts.provenance` with deterministic SHA-256.

Provenance file format:
- Outer envelope uses the existing signed-assurance manifest shape (`schema_version: 1`, `payload`, `signature`).
- `payload.kind` is fixed to `clearlang.release_provenance`.
- `payload.schema_version` is fixed to `1`.
- `payload.release_signature_key_id` must match release signature `key_id`.
- `payload.artifacts.*_sha256` includes deterministic hash claims for core release artifacts.

## Verify Behavior
`clg verify-bundle` behavior:
1. If provenance exists, verify signature and payload contract.
2. Validate `release_signature_key_id` consistency against release signature file.
3. Fail closed on malformed/invalid provenance with deterministic diagnostics (`C140`).
4. If `--require-provenance` is passed and provenance is missing, fail closed (`C140`).

## Policy
Per Phase `25.6.0` policy lock:
- Provenance is required for GA release-train artifacts.
- Local/dev verification may omit provenance unless `--require-provenance` is explicitly set.

## References
- `crates/cli/src/commands/release.rs`
- `docs/design/phase-25.6.0-binary-ga-and-provenance-policy-lock.md`
- `docs/release-process.md`
