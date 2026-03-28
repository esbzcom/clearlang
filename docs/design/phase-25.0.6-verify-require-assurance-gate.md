# Phase 25.0.6 - Verify Policy Gate (`--require-assurance proved_all`)

## Status
Design lock + implementation note for `25.0.6` in `docs/TODO.md`.

## Goal
Add an explicit verifier policy gate so release workflows can require theorem-grade assurance from signed artifacts.

## CLI Contract
- New flag on `clg verify`:
  - `--require-assurance <STATUS>`
- Locked value in Milestone 3:
  - `proved_all`

## Enforcement Rules
1. `clg verify --require-assurance proved_all` reads signed payload `proof_status`.
2. Verification fails with policy error (`V005`) when:
   - `proof_status` is missing,
   - `proof_status` is invalid,
   - `proof_status != proved_all`,
   - `--require-assurance` value is invalid.
3. If `--assurance-manifest` is provided with `--require-assurance`, verifier also checks:
   - manifest signature validity,
   - manifest `proof_status` exists and is valid,
   - manifest `proof_status` equals signature payload `proof_status`.

## Determinism
- Decision input is fully explicit (signed payload, optional signed manifest, explicit required status).
- No ambient environment/time-based behavior is used in this gate.

## Release Workflow Wiring
- Release pipeline should include:
  - `clg verify ... --require-assurance proved_all`
- This gate composes with existing release policy checks.

## Exit Criteria for 25.0.6
1. Verify command supports `--require-assurance proved_all`.
2. Integration tests cover pass/fail parsing and fail-closed rejection behavior.
3. Release docs include the gate in production verification flow.

## References
- `docs/TODO.md`
- `docs/design/phase-25.0.5-proof-status-emission.md`
- `crates/cli/src/main.rs`
- `crates/cli/src/commands/verify.rs`
- `crates/cli/tests/signing/tests.rs`
