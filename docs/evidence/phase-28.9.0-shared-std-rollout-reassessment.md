# Phase 28.9.0 - Shared Std Rollout Reassessment

Date: 2026-06-27
Status: Complete
Owner: std-arch-owner

## Conclusion

The rollout decision may now be re-run.

Gates F-I are green, the supported shared-std workflow remains fail-closed, and the remaining
work moves from readiness gating to product decision and contract lock-in.

This completes `28.9.0` and unblocks `28.9.1`.

## Gate Status

The roadmap now records all prerequisite distribution gates as complete:

- `28.5` Distribution Gate F
- `28.6` Distribution Gate G
- `28.7` Distribution Gate H
- `28.8` Distribution Gate I

Reference:
- `docs/todo/milestone_3_roadmap.md`

## Fail-Closed Review

The current supported workflow still fails closed at the important shared-std boundaries:

- manifest intent validation rejects unsupported delivery modes, empty shared package sets, and
  dependency/shared-package collisions
  - `crates/cli/src/commands/release_defaults.rs`
- lock generation rejects unsupported package ids, missing canonical signatures, missing
  provenance, invalid digests, zero-sized artifacts, and out-of-set shared dependencies
  - `crates/cli/src/commands/pkg/lockfile_loader.rs`
- release and verify-bundle preserve and validate non-empty shared-std evidence
  - `crates/cli/src/commands/release.rs`
  - `crates/cli/tests/cli_it/diagnostics/release_command_pipeline.rs`
- runtime loading rejects missing artifacts, untrusted signatures, and ABI mismatches
  - `crates/cli/src/commands/run/package_loader/resolve.rs`
  - `crates/cli/src/commands/run/package_loader/signatures.rs`
  - `crates/cli/src/commands/run/package_loader_tests.rs`

## Provenance Review

Shared-std publication now emits the full operator-to-consumer evidence set for bundled package
ids:

- package-specific publish manifests
- canonical package signatures
- provenance statements
- canonical `clg.package-metadata.json`
- canonical `clg.package-abi.json`
- deterministic registry layout

Reference:
- `xtask/src/main/core/artifacts_and_vendor.rs`
- `xtask/src/main/tests.rs`
- `docs/evidence/phase-28.8-shared-std-support-readiness.md`

## Validation-Path Review

The major supported-workflow duplication called out earlier in Phase 28 is now closed:

- schema-v2 shared-std lock writing and reading are unified around the same authoritative model
  instead of separate writer/reader interpretations
  - `crates/cli/src/commands/pkg/prelude.rs`
  - `crates/cli/src/commands/pkg/lockfile_loader.rs`
  - `crates/cli/src/commands/pkg/artifacts.rs`
- release, verify-bundle, and runtime checks consume the same shared-std evidence shape
  established by Gates G-H
- the explicit gate command passes:
  - `cargo run -p xtask -- shared-std-distribution-check`

## Next Task

The next roadmap item is `28.9.1`:

- decide whether `shared` becomes a supported production option while preserving `embedded` as a
  valid production mode

That decision should update the earlier experimental-only decision in:

- `docs/design/phase-28.4.1-shared-std-rollout-decision.md`
