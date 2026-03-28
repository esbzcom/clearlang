# Phase 25.0.5 - Theorem-Grade Status Emission

## Status
Design lock + implementation note for `25.0.5` in `docs/TODO.md`.

## Goal
Emit deterministic theorem-grade status in assurance artifacts and signature payloads so release tooling can consume a single machine-checkable status field.

## Locked Field
- Field name: `proof_status`
- Allowed values:
  - `proved_all`
  - `not_proved_all`

## Emission Surfaces
1. Signature payload (`out.sig.json` payload map).
2. Signed assurance manifest payload (`out.assurance.json` payload map).
3. Embedded `clearlang.proof` section top-level map.
4. VC JSON output objects (`--emit-vcs`) for deterministic tooling alignment.

## Current Deterministic Evaluation Rule
`proof_status = proved_all` iff all conditions hold:
1. compiler mode is `strict`,
2. VC set is non-empty,
3. every VC has `status == proved`,
4. every VC has zero assumption boundaries.

Otherwise:
- `proof_status = not_proved_all`.

## Compatibility
- `proof_status` is additive and backward-compatible.
- Older artifacts without this field remain parseable.
- New producers emit the field consistently for stable policy automation.

## Exit Criteria for 25.0.5
1. `proof_status` emitted on all listed surfaces.
2. Integration tests assert deterministic value emission in signature/manifest/VC outputs.
3. Proof schema docs mention the field and value contract.

## References
- `docs/TODO.md`
- `docs/design/phase-25.0.2-theorem-grade-certification-policy.md`
- `crates/cli/src/proofs.rs`
- `crates/cli/src/commands/build/vcs_json.rs`
- `crates/cli/tests/signing/tests.rs`
- `crates/cli/tests/cli_it/vc_outputs/basic.rs`
