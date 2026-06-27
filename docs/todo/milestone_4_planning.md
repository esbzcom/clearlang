# Milestone 4 - Post-Milestone-3 Planning

Milestones 1-3 are complete. The next task is to re-establish an explicit backlog before new
feature work resumes.

Planning rule: do not add new execution slices that weaken the current supported production
contract for `embedded` or `shared` delivery, or that cut across the README design principles
without an explicit design lock.

- [x] 29.0 Milestone 4 planning lock [Planning Gate A] `Completed: 2026-06-27`
  - [x] 29.0.0 Publish Milestone 4 purpose, prioritization rules, and success target so post-Milestone-3 work resumes from an explicit planning contract rather than ad hoc feature selection. (`docs/design/phase-29.0.0-milestone-4-planning-lock.md`) `Completed: 2026-06-27`

- [x] 29.1 First execution slice selection [Planning Gate B] `Completed: 2026-06-27`
  - [x] 29.1.0 Select the first post-Milestone-3 implementation slice, with explicit scope, DRI, success target, and acceptance gate, while preserving the current release/proof/distribution safety contract. (`docs/design/phase-29.1.0-first-post-m3-slice-selection.md`) `Completed: 2026-06-27`
  - [x] 29.1.1 Publish the selected execution order and rationale, including what is intentionally deferred. (`docs/design/phase-29.1.0-first-post-m3-slice-selection.md`) `Completed: 2026-06-27`

- [x] 29.2 `std::sequence` shared-std promotion [Execution Gate A] `Completed: 2026-06-27`
  - [x] 29.2.0 Lock the supported `std::sequence` shared-package contract: package id, modules, symbols, publication shape, and fail-closed migration/activation expectations. (`docs/design/phase-29.2.0-std-sequence-shared-std-lock.md`) `Completed: 2026-06-27`
  - [x] 29.2.1 Extend bundled shared-std package allowlists, canonical publication flow, and manifest/lock validation so `std::sequence` is a supported shared package alongside `std::text`, `std::int`, and `std::codec`. (`crates/cli/src/commands/modules/bundled_std_packages.rs`, `xtask/src/main/core/artifacts_and_vendor.rs`, `xtask/src/main/artifacts_cli_models.rs`) `Completed: 2026-06-27`
  - [x] 29.2.2 Extend release, verify-bundle, runtime-loader, import-migration, and xtask publish coverage to exercise `std::sequence` happy-path and fail-closed behavior. (`crates/cli/src/commands/modules/bundled_std_packages.rs`, `xtask/src/main/tests.rs`) `Completed: 2026-06-27`
  - [x] 29.2.3 Update user/operator documentation only after implementation lands so the supported shared-package set stays accurate. (`docs/release/shared-std-user-guide.md`, `docs/release/shared-std-operations.md`, `docs/release-process.md`, `docs/design/phase-28.9.1-shared-std-production-decision.md`, `docs/design/phase-28.9.2-shared-std-product-contract-lock.md`) `Completed: 2026-06-27`
