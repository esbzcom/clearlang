# Phase 28.9.2 - Shared Std Product Contract Lock

Date: 2026-06-27
Status: Locked
Owner: std-arch-owner

## Purpose

This document locks the final supported product contract for shared std after the `28.9.1`
promotion decision.

The contract is intentionally narrow:

- `embedded` remains the default supported production mode
- `shared` is a supported explicit opt-in production mode
- the supported shared workflow is the schema-v2 fail-closed workflow only

## Supported Activation Semantics

Shared std is supported only when all of the following are true:

1. `clg.project.json` uses `schema_version = 2`
2. `std.delivery = "shared"`
3. `std.packages[]` is non-empty
4. every shared package id is in the current bundled supported set
5. lock, release, verify-bundle, and runtime flows consume the resulting shared evidence without
   fallback to embedded mode

Embedded remains supported when:

1. the project stays on the embedded delivery path
2. no shared-std evidence is requested

The supported bundled shared package ids are:

- `std::text`
- `std::int`
- `std::sequence`
- `std::codec`
- `std::contract`
- `std::eth`
- `std::solana`
- `std::cosmos`

Anything outside that package-id set is not part of the current supported shared contract.

## CI And Profile Coverage Expectations

The supported contract is enforced by behavior gates, not source-shape audits.

The minimum required coverage set is:

1. `xtask release-precheck`
2. `xtask shared-std-distribution-check`
3. release/verify-bundle command fixtures proving:
   - non-empty shared evidence survives the normal release path
   - upgrade/rollback flows remain deterministic
   - strict import-map, bundle-manifest, and provenance tampering fail closed
4. runtime loader smoke coverage proving:
   - unsupported ABI ranges fail with `R015`
   - missing artifacts fail with `R012`
   - untrusted signers fail with `R014`

Authoritative references:

- `xtask/src/main/core/dispatch_and_drift.rs`
- `xtask/src/main/std_arch.rs`
- `crates/cli/tests/cli_it/diagnostics/release_command_pipeline.rs`
- `crates/cli/tests/run_smoke/runtime_loader_core.rs`

## Compatibility And Deprecation Rules

The supported compatibility contract is:

1. `embedded` is not deprecated by this rollout and remains a valid production mode
2. `shared` does not replace the default
3. shared production activation requires schema v2; schema-v1 shared workflows are unsupported
4. no silent downgrade from `shared` to `embedded` is allowed
5. no implicit migration of embedded projects is allowed
6. expanding the supported shared package-id set requires an explicit design/doc/coverage update

This keeps the compatibility surface predictable for users and AI tooling:

- existing embedded projects keep working without migration pressure
- shared users get one documented activation shape
- unsupported historical or partial shared workflows are rejected instead of heuristically repaired

## Documentation And Operational Ownership

The supported documentation set for shared std is:

- user workflow: `docs/release/shared-std-user-guide.md`
- operator publication/distribution: `docs/release/shared-std-operations.md`
- release overview: `docs/release-process.md`
- signer rotation: `docs/security/shared-std-key-rotation.md`
- rollback/incident handling: `docs/security/shared-std-rollback-procedure.md`

Operational ownership is locked as a documentation-backed process requirement:

1. changes to shared activation, publishing, trust, provenance, upgrade, or rollback behavior must
   update the relevant docs in the same patch
2. changes that alter the supported package-id set or validation contract must update:
   - this contract lock
   - the user/operator docs
   - the behavior-gate coverage

## Debt Cleanup Closure

The rollout no longer carries the product debts called out earlier in Phase 28:

1. normal shared workflow exists through `pkg lock`, `release`, and `verify-bundle`
2. schema-v2 shared evidence is validated through one authoritative command/runtime model
3. release-side, verify-side, and runtime-side fail-closed checks operate on the same evidence
   shape
4. packaging, provenance, registry layout, rotation, rollback, and support playbooks are documented

Closure references:

- `docs/evidence/phase-28.8-shared-std-support-readiness.md`
- `docs/evidence/phase-28.9.0-shared-std-rollout-reassessment.md`
- `docs/design/phase-28.9.1-shared-std-production-decision.md`

## Final Product Position

Milestone 3 closes with two supported production modes:

1. `embedded` — default, simpler deployment, minimal operational surface
2. `shared` — explicit opt-in, deterministic evidence chain, supported for the current bundled
   shared package set

That is the locked product contract going forward.
