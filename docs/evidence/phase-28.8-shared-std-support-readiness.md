# Phase 28.8 Shared Std Support Readiness

This document records the completion evidence for TODO item `28.8.2`.

## Scope

`28.8.2` requires proof that the shared-std support model is acceptable:

- deterministic diagnostics exist,
- operational playbooks are repeatable,
- packaging and deployment behavior is explicit, and
- no unresolved ambiguity remains.

## Evidence Present

### Deterministic Diagnostics

Documented diagnostics already cover the main shared-std failure classes:

- `C109` manifest/lock compatibility and shared-std intent conflicts
  - `docs/diagnostics.md`
  - `crates/cli/src/commands/release_defaults.rs`
  - `crates/cli/src/commands/pkg/lock_command.rs`
  - `crates/cli/src/commands/pkg/lockfile_loader.rs`
- `C140` verify-bundle shared-std evidence and provenance failures
  - `docs/diagnostics.md`
  - `crates/cli/src/commands/release.rs`
- `R012` missing runtime shared-std artifact
  - `docs/diagnostics.md`
  - `crates/cli/src/commands/run/package_loader/resolve.rs`
- `R014` shared-std signature/trust failure
  - `docs/diagnostics.md`
  - `crates/cli/src/commands/run/package_loader/signatures.rs`
  - `crates/cli/src/commands/run/package_loader/resolve.rs`
- `R015` shared-std ABI or binding mismatch
  - `docs/diagnostics.md`
  - `crates/cli/src/commands/run/package_loader/resolve.rs`

### Operational Playbooks

Current operator and user playbooks now exist:

- operator publish / upgrade / rollback flow
  - `docs/release/shared-std-operations.md`
- user opt-in / lock / release / verify / rollback flow
  - `docs/release/shared-std-user-guide.md`
- signer rotation policy
  - `docs/security/shared-std-key-rotation.md`
- rollback / incident response
  - `docs/security/shared-std-rollback-procedure.md`

### Command-Level Behavior Gates

Shared-std behavior is already exercised in command-level gates:

- activation gate wrapper
  - `xtask/src/main/std_arch.rs`
- xtask smoke gate entrypoint
  - `shared-std-distribution-check`
- release and verify-bundle smoke coverage
  - `crates/cli/src/commands/release.rs`
- runtime loader negative coverage
  - `crates/cli/src/commands/run/package_loader_tests.rs`

### Packaging And Deployment Alignment

The earlier operator/consumer package-id mismatch is now closed.

Operator publication now uses the same bundled shared-std package-id space that the
schema-v2 manifest/lock contract already accepts:

- `xtask shared-std-publish` now requires `--package-id`
- supported bundled package ids are `std::text`, `std::int`, `std::sequence`, and `std::codec`
- publish output now emits package-specific manifests plus canonical
  `clg.package-metadata.json` and `clg.package-abi.json`
- file-registry publication preserves the nested `std-packages/` artifact layout
- trust anchor ids are written directly into canonical consumer metadata

Implementation and validation references:

- publish command + registry copy logic
  - `xtask/src/main/core/artifacts_and_vendor.rs`
- CLI package-id acceptance model
  - `crates/cli/src/commands/modules/bundled_std_packages.rs`
  - `crates/cli/src/commands/release_defaults.rs`
- existing shared-manifest lockfile generation coverage
  - `crates/cli/src/commands/pkg/tests.rs`
- publish-bundle coverage for package-specific output and canonical metadata
  - `xtask/src/main/tests.rs`

The publish/test path now proves the same package identity can flow through:

- operator publication,
- canonical package metadata/ABI emission,
- shared-manifest lock generation, and
- existing release / verify-bundle / runtime loader shared-std gates.

## Status Snapshot

- deterministic diagnostics: present
- repeatable playbooks: present
- packaging/deployment ambiguity: closed for bundled shared-std package ids
- `28.8.2`: complete
