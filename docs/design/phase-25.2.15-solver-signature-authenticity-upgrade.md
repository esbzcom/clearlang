# Phase 25.2.15 - Solver Signature Authenticity Upgrade

## Status
Design lock + implementation record for `25.2.15` in `docs/TODO.md`.

## Goal
Upgrade solver bundle `.sig` verification from integrity-metadata mode to cryptographic publisher-authenticity verification with pinned vendor keys and explicit rotation policy.

## Locked Contract
- Runtime solver loading must fail closed unless all of the following hold:
  - `<solver>.sha256` exists and matches `sha256:<hex64>` of solver bytes.
  - `<solver>.sig` exists and is valid JSON envelope:
    - `schema_version: 1`
    - `scheme: "ed25519"`
    - `key_id` is present in pinned trusted signers from `phase-25.1.16-solver-supply-chain.lock.json`
    - `signed_payload` equals the checksum sidecar entry
    - `signature` verifies against the trusted signer public key
- Signature policy source of truth is `docs/design/phase-25.1.16-solver-supply-chain.lock.json`:
  - `signature_mode: "publisher-auth-ed25519-v1"`
  - `signature_schema_version: 1`
  - `trusted_signers[]` with key status gating (`active|next`)
  - `rotation_policy` with `allowed_statuses` and `min_trusted_signers`

## Vendor Staging Contract
- `xtask solver-vendor-stage` now emits signed `.sig` JSON envelopes (not plain integrity metadata line).
- Required env var:
  - `CLG_SOLVER_VENDOR_SIGNING_KEY_HEX` (32-byte Ed25519 private key hex)
- Optional signer identity override:
  - `--key-id <ID>` (default `z3-vendor-k7-2026q2`)

## Determinism and Safety
- Verification logic is deterministic and side-effect free.
- Any schema mismatch, parse failure, signer mismatch, payload mismatch, or signature verification failure rejects the solver candidate.
- If no valid solver candidate remains, existing non-strict behavior keeps VC status as `generated`; strict/release flows remain fail-closed by existing policy gates.

## References
- `crates/cli/src/commands/build/solver.rs`
- `xtask/src/main/core.rs`
- `xtask/src/main/artifacts_cli_models.rs`
- `xtask/src/main/tests.rs`
- `crates/cli/tests/solver_outcomes.rs`
- `crates/cli/tests/solver_replay_stability.rs`
- `crates/cli/tests/verified_std_core_subset.rs`
- `crates/cli/tests/profile_regression_gate.rs`
- `crates/cli/tests/phase25_solver_supply_chain_gate.rs`
- `docs/design/phase-25.1.16-solver-supply-chain.lock.json`
- `docs/release-process.md`
