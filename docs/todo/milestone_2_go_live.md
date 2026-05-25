# Milestone 2 - Namespace/Packaging/Runtime Go-Live (20-24)

Extracted from the legacy phase-based backlog for milestone-first planning.

### 20 Runnable Namespace Baseline + Lock Gates (milestone_2)
- [x] 20.0 Add language comment syntax support (AI-friendly, deterministic).
  - [x] 20.0.1 Parser/lexer: add line comments `// ...` and block comments `/* ... */` with deterministic tokenization and spans.
  - [x] 20.0.2 Define and document one canonical style for generated code/docs (`//` preferred; `/* ... */` only for multi-line notes).
  - [x] 20.0.3 Add parser + CLI tests proving comments are accepted in runnable fixtures and do not affect diagnostics stability.
  - [x] 20.0.4 Numeric literal readability: support `_` as digit separator (`1_000`), keep `,` invalid (`1,000`), and add a targeted diagnostic suggesting `_`.
- [x] 20.1 Publish post-19 std/package architecture lock (precompiled `std::core` + host-backed `std::host` + chain packages).
  - [x] 20.1.0 Bootstrap minimal strict lockfile support for gate preflight (direct dependencies only).
    - [x] 20.1.0.1 Define minimal lockfile v0 schema for strict mode (`name`, `version`, `digest`) covering direct dependencies only. (`docs/design/phase-20.1.0-strict-lockfile-v0.md`)
    - [x] 20.1.0.2 Implement deterministic strict-mode lockfile loader/validator for v0 schema (no transitive solver in 20.1).
    - [x] 20.1.0.3 Emit stable diagnostics for missing/malformed strict lockfile input and document remediation.
    - [x] 20.1.0.4 Define minimal trust-policy v0 for strict gates (trusted signer set + revocation/expiry checks) as a bootstrap before 22.0.4 lifecycle expansion. (`docs/design/phase-20.1.0-trust-policy-v0.md`)
    - [x] 20.1.0.5 Define minimal host-profile v0 schema/capability set consumed by strict preflight before 24.0.2 profile expansion. (`docs/design/phase-20.1.0-host-profile-v0.md`)
    - [x] 20.1.0.6 Implement deterministic loaders/validators for trust-policy v0 and host-profile v0 with stable diagnostics.
    - [x] 20.1.0.7 Define minimal package metadata schema v0 + ABI contract v0 (direct dependencies only) required by strict preflight before 22.0.3 policy expansion. (`docs/design/phase-20.1.0-package-metadata-abi-v0.md`)
    - [x] 20.1.0.8 Implement deterministic metadata/ABI v0 validators and stable diagnostics for schema/ABI mismatches.
  - [x] 20.1.1 Design lock document for long-term final solution (`docs/design/phase-20.0-std-packaging-runtime-linking.md`).
  - [x] 20.1.2 Define deterministic acceptance gates for package trust and runtime linker behavior in strict mode.
    - [x] 20.1.2.1 Strict-mode source-of-truth gate: resolve packages only from lockfile + trusted local store (no implicit network fetch).
    - [x] 20.1.2.2 Artifact identity gate: require exact `(name, version, digest)` match against lockfile entries; digest mismatch fails closed.
    - [x] 20.1.2.3 Trust gate: require valid package signature against configured trust anchors from trust-policy v0; untrusted/revoked/expired signer fails closed.
    - [x] 20.1.2.4 Metadata/schema gate: package metadata schema version must be accepted; unknown/unsupported schema fails with stable diagnostics.
    - [x] 20.1.2.5 ABI/link gate: imported symbols must match expected signature/effect/capability profile exactly; ABI mismatch fails deterministically.
    - [x] 20.1.2.6 Runtime capability gate: required host capabilities for linked imports must be present in selected host-profile v0, otherwise fail closed.
    - [x] 20.1.2.7 Determinism gate: identical inputs (source, lockfile, package store, policy) produce identical resolved direct-dependency import map and diagnostics ordering.
    - [x] 20.1.2.8 CI functional acceptance suite: add positive + tamper negative tests for 20.1.2.1-20.1.2.6 with fixed diagnostic-code assertions.
  - [x] 20.1.3 Implement strict-mode gate evaluator (code path, no resolver expansion yet).
    - [x] 20.1.3.1 Define a single preflight input model (lockfile v0 entries, package metadata, trust-policy v0, host-profile v0).
    - [x] 20.1.3.2 Implement a pure evaluator (`evaluate_strict_gates`) that returns deterministic, stably ordered violations.
    - [x] 20.1.3.3 Wire evaluator into `clg build --compiler-mode strict` before final link/build outputs.
    - [x] 20.1.3.4 Add deterministic ordering rule (gate id -> package id -> symbol id) and snapshot tests.
    - [x] 20.1.3.5 Ensure strict mode fails closed whenever any gate violation exists, emitting a complete deterministic violation list (no permissive fallback path).
    - [x] 20.1.3.6 Emit canonical direct-dependency import-map artifact in strict preflight and assert deterministic serialization/hash across identical inputs.
  - [x] 20.1.4 Publish diagnostic + fixture matrix for 20.1 gates.
    - [x] 20.1.4.1 Publish proposed package/linker strict-gate diagnostics (`C101`-`C108`) in the phase design lock (`docs/design/phase-20.0-std-packaging-runtime-linking.md`).
    - [x] 20.1.4.2 Publish canonical fixture matrix in the phase design lock (positive/trust-fail/digest-fail/schema-fail/abi-fail/capability-fail/determinism for direct dependencies).
    - [x] 20.1.4.3 Promote `C101`-`C108` to canonical diagnostics registry in `docs/diagnostics.md` and keep diagnostics code-table tests green (`crates/cli/tests/diagnostics_codes.rs`).
    - [x] 20.1.4.4 Add CI determinism replay job that runs strict preflight twice with identical inputs and asserts identical diagnostics ordering.
    - [x] 20.1.4.5 In the same replay job, assert strict preflight import-map artifact bytes/hash are identical.
- [x] 20.2 Namespaced runnable baseline first: make `clearlang-tests/16_namespaced_call.clear` runnable (not parse-only).
  - [x] 20.2.1 Replace the unresolved `std::math::add` usage with a valid namespaced callable path in a runnable fixture layout.
  - [x] 20.2.2 Add CLI integration coverage asserting `clg run ...16_namespaced_call.clear` succeeds.
  - [x] 20.2.3 Update sample docs to distinguish parse-only vs runnable namespace examples.

### 21 Precompiled Std Core Packaging
- [x] 21.0 Precompiled `std::core` package pipeline.
  - [x] 21.0.1 Implement the versioned std-core artifact pipeline and metadata model with reproducible hash semantics. (`docs/design/phase-21.0-std-core-package-surface.md`)
  - [x] 21.0.2 Implement import pruning so only used package functions are emitted as imports in app Wasm. (`docs/design/phase-21.0-std-core-package-surface.md`)
  - [x] 21.0.3 Reconcile std-core function surface lock with shipped std metadata.
    - [x] 21.0.3.1 Align `docs/design/phase-21.0-std-core-package-surface.md` and `crates/cli/assets/std-metadata.json` (including `std::list::remove_take` and `std::map::{insert_take,remove_take}`) with one canonical source of truth.
    - [x] 21.0.3.2 Add CI drift gate that fails when locked std surface, emitted std metadata, typer std call-check surface (builtins + specialized std call check modules), and codegen std binding map (intrinsic vs package-import routing) diverge.
    - [x] 21.0.3.3 Emit deterministic std binding-map artifact in CI so drift checks do not depend on internal codegen implementation details.
  - [x] 21.0.4 Lock and align `std::host` capability surface for production profile readiness.
    - [x] 21.0.4.1 Define canonical v1 ownership for host-facing capabilities (`time`, `random`, `chain_id`, storage, events) across `std::host` vs chain wrappers, with explicit non-goals.
    - [x] 21.0.4.2 Lock strict-mode deterministic policy for `std::env::{time,random}` (allow/deny profile matrix and diagnostics) and align with runtime host-import docs.
    - [x] 21.0.4.3 Align strict host-profile capability allowlist and strict-gate checks with the canonical v1 `std::host` surface.
    - [x] 21.0.4.4 Publish machine-readable host capability policy artifact per profile (`contract_static`, `shared_app`) and assert deterministic serialization/hash in CI.
    - [x] 21.0.4.5 Add CI profile-conformance fixtures for `std::env::{time,random,chain_id}` and expected strict diagnostics on disallowed capabilities.
  - [x] 21.0.5 Add reproducible std-core artifact CI evidence gates (validation of 21.0.1 behavior).
    - [x] 21.0.5.1 Emit versioned `std::core` artifact + metadata + digest as CI artifacts with deterministic naming.
    - [x] 21.0.5.2 Add replay determinism assertion: identical inputs produce byte-identical std-core artifact and identical digest.
  - [x] 21.0.6 Harden import-pruning as an explicit CI acceptance gate (validation of 21.0.2 behavior).
    - [x] 21.0.6.1 Add Wasm import-section assertions proving only used external/package symbols are emitted.
    - [x] 21.0.6.2 Add negative coverage proving unused symbols declared in metadata/ABI never appear in emitted app Wasm imports.
  - [x] 21.0.7 Add non-vacuous precompiled-link proof gate for std-core.
    - [x] 21.0.7.0 Define and lock precompiled std-core activation contract (explicit build/profile switch and fallback policy) before enforcing non-vacuous link behavior.
    - [x] 21.0.7.1 Add CI fixture proving at least one locked `std::core` symbol is linked via package ABI/import (not satisfied by local intrinsic lowering only).
    - [x] 21.0.7.2 Add failure coverage that rejects fallback-to-intrinsic behavior when precompiled std-core mode is enabled for that symbol set and emits deterministic diagnostics.
  - [x] 21.0.8 Replace transitional synthetic strict fixtures with locked std-core symbols.
    - [x] 21.0.8.1 Migrate strict acceptance fixtures from synthetic `std::core::math::*` symbols to canonical locked std-core symbols.
    - [x] 21.0.8.2 Keep deterministic diagnostics ordering/hash assertions green after fixture migration, with no semantic regression in the existing `C101`-`C108` strict acceptance suite.

### 22 Package Trust + Dependency Resolution
- [x] 22.0 Package metadata trust hardening.
  - [x] 22.0.0 Publish and lock Phase 22 design documents (metadata migration, metadata v1, lockfile v1, resolver/solver determinism, advisory policy, diagnostics). (`docs/design/phase-22.0.0-package-trust-resolution-design-lock.md`, `docs/design/phase-22.0.1-canonical-package-metadata-migration.md`, `docs/design/phase-22.0.2-package-metadata-v1.md`, `docs/design/phase-22.0.3-lockfile-v1.md`, `docs/design/phase-22.1.0-resolver-semver-determinism.md`, `docs/design/phase-22.1.3-vulnerability-response-policy.md`, `docs/design/phase-22.0.6-phase22-diagnostics-reservation.md`)
  - [x] 22.0.1 Unify package metadata to one canonical production model and define migration/deprecation from legacy `clg-packages.json`. (`docs/design/phase-22.0.1-canonical-package-metadata-migration.md`, deterministic coexistence conflict gate in `crates/cli/src/commands/modules/package_metadata.rs`, IT coverage `legacy_and_canonical_package_metadata_conflict_reports_c027`)
  - [x] 22.0.2 Extend package metadata with artifact digest/signature/trust-anchor fields and strict validation. (strict preflight metadata schema v1 support + validation in `crates/cli/src/commands/build/strict_package_contract.rs`, trust-gate linkage checks in `crates/cli/src/commands/build/strict_package_signatures.rs`, regression coverage in strict unit/CLI IT suites)
  - [x] 22.0.3 Expand lockfile flow from 20.1 v0 bootstrap to full deterministic workflow.
    - [x] 22.0.3.1 Add CLI lockfile generation/update commands for exact package versions + digests.
    - [x] 22.0.3.2 Define canonical lockfile serialization/hash rules (stable ordering + deterministic writes).
    - [x] 22.0.3.3 Add replay tests proving byte-identical lockfiles from identical inputs.
  - [x] 22.0.4 Define package metadata/ABI compatibility policy (schema evolution, deprecation windows, migration guarantees) with regression tests. (`docs/design/phase-22.0.4-package-metadata-abi-compatibility-policy.md`, `crates/cli/tests/schema_compatibility_policy.rs`)
  - [x] 22.0.5 Expand signer lifecycle policy from 20.1 trust-policy v0 bootstrap to full production policy (rotation, revocation, expiry, emergency compromise handling). (`docs/design/phase-22.0.5-signer-lifecycle-policy.md`, `crates/cli/src/commands/build/strict_trust_policy.rs` tests, `crates/cli/src/commands/build/strict_package_signatures.rs` tests)
  - [x] 22.0.6 Reserve and register Phase 22 diagnostics for resolver/solver/advisory flows before implementation (stable JSON code contracts). (`docs/diagnostics.md` `C109`-`C119`, `crates/cli/tests/phase22_diagnostics_reservation.rs`)
  - [x] 22.0.7 Refactor shared strict validators (semver/digest/schema/id parsing) into a single module to prevent rule drift. (`docs/design/phase-22.0.7-strict-validator-consolidation.md`, `crates/cli/src/commands/build/strict_validation.rs`)
- [x] 22.1 Dependency resolution completion gates for milestone_2.
  - [x] 22.1.0 Lock deterministic resolver + semver solver policy (tie-break rules, conflict precedence, diagnostics ordering). (`docs/design/phase-22.1.0-resolver-policy.lock.json`, `crates/cli/tests/resolver_policy_lock.rs`)
  - [x] 22.1.1 Add transitive dependency resolution for compiled packages (deterministic graph + cycle diagnostics). (`crates/cli/src/commands/pkg.rs`, `crates/cli/tests/cli_it/pkg_lock.rs`)
  - [x] 22.1.2 Add deterministic semver solver with lockfile generation/update flow. (`crates/cli/src/commands/pkg.rs`, `crates/cli/tests/cli_it/pkg_lock.rs`)
  - [x] 22.1.3 Add package vulnerability response flow (advisory ingestion, denylist/yank policy, forced-upgrade semantics, deterministic diagnostics). (`crates/cli/src/commands/pkg.rs`, `crates/cli/tests/cli_it/pkg_lock.rs`)
    - [x] 22.1.3.a Strict advisory trust gate: require signed advisory envelope + trust-policy signer verification (trusted, non-revoked, valid signer window).
    - [x] 22.1.3.b Deterministic advisory-time evaluation: support `--advisory-as-of` and lock strict-mode requirement for replay-stable advisory applicability windows.
  - [x] 22.1.4 Add CI determinism replay gates for resolver/solver outputs (resolved graph artifact, lockfile bytes/hash, diagnostics ordering). (`.github/workflows/ci.yml`, `crates/cli/tests/ci_workflow.rs`, `crates/cli/tests/cli_it/pkg_lock.rs`, `crates/cli/src/commands/pkg.rs`)
  - [x] 22.1.5 Add build/run resolution parity policy so package trust/resolution behavior is explicit for both `clg build` and `clg run`.
    - [x] 22.1.5.1 Migrate standard/permissive compiled-package import indexing from legacy `clg-packages.json` to canonical metadata/ABI inputs and retire legacy loader behavior. (`crates/cli/src/commands/modules/package_metadata.rs`, `crates/cli/tests/cli_it/imports.rs`, `crates/cli/tests/cli_it/vc_outputs.rs`, `docs/design/phase-22.0.1-canonical-package-metadata-migration.md`, `docs/diagnostics.md`)
    - [x] 22.1.5.2 Lock and document strict vs non-strict trust semantics for metadata v1 and proof-claim boundaries (strict-only release/audit trust claims). (`docs/design/phase-22.0.2-package-metadata-v1.md`)
    - [x] 22.1.5.3 Emit an explicit non-strict assurance-claim marker in CLI/artifact outputs to prevent interpreting non-strict proofs as release-grade trust evidence.

### 23 Runtime Package Loader + Linker
- [x] 23.0 Runtime linker for compiled packages.
  - [x] 23.0.0 Publish and lock Phase 23 runtime loader/linker design before implementation. (`docs/design/phase-23.0-runtime-loader-linker-design-lock.md`)
    - [x] 23.0.0.1 Lock runtime source-of-truth inputs (lockfile/import-map/trust-policy/host-profile), canonical artifact locator model, deterministic resolution ordering, and canonical runtime link artifact contract (`clg.runtime-link.json` + hash).
    - [x] 23.0.0.2 Reserve and register Phase 23 runtime-loader diagnostics (`R012`-`R017`) before wiring implementation, and pin with reservation tests. (`docs/diagnostics.md`, `crates/cli/tests/phase23_diagnostics_reservation.rs`)
    - [x] 23.0.0.3 Lock deterministic replay contract and runtime signer-time semantics (signed_at-anchored, no ambient wall-clock dependence) for runtime loader outputs/diagnostics.
  - [x] 23.0.1 Implement host-side package loader core from trusted local store/index with explicit no-implicit-network default. (`crates/cli/src/commands/run/package_loader.rs`, `crates/cli/src/commands/run/mod.rs`, `crates/cli/tests/run_smoke.rs`)
  - [x] 23.0.2 Enforce fail-closed runtime trust gates (digest/signature/policy + lock/import-map consistency) before linking any package artifact. (`crates/cli/src/commands/run/package_loader.rs`, `crates/cli/tests/run_smoke.rs`)
  - [x] 23.0.3 Implement deterministic runtime linker/import binding path and deterministic runtime diagnostics for missing/mismatched/untrusted package artifacts. (`crates/cli/src/commands/run/mod.rs`, `crates/cli/src/commands/run/package_loader.rs`, `crates/cli/tests/run_smoke.rs`)
  - [x] 23.0.4 Add artifact availability/resilience policy (mirrors/cache/offline mode/retry/failure behavior) and operational runbook coverage. (`crates/cli/src/commands/run/package_loader.rs`, `crates/cli/tests/run_smoke.rs`, `docs/runtime/runtime-loader-resilience-runbook.md`)
- [x] 23.1 Runtime loading completion gate for milestone_2.
  - [x] 23.1.1 Enable automatic runtime package loader/linker path in runtime hosts (`clg run` and production host integrations) without manual import wiring. (`crates/cli/src/lib.rs`, `crates/cli/src/main.rs`, `crates/cli/src/commands/run/mod.rs`, `docs/runtime/host-integration-api.md`)
  - [x] 23.1.2 Add CI tamper + determinism replay matrix for runtime loading/linking (missing/mismatch/untrusted artifacts, diagnostics ordering, replay stability). (`crates/cli/tests/run_smoke.rs`, `.github/workflows/ci.yml`, `crates/cli/tests/ci_workflow.rs`)
  - [x] 23.1.3 Keep fail-closed runtime trust checks mandatory in all production profiles (no permissive fallback for runtime package loading). (`crates/cli/src/commands/run/package_loader.rs`, `crates/cli/tests/run_smoke.rs`)
  - [x] 23.1.4 Add staged rollout/canary + rollback criteria and release gate evidence for runtime loader enablement in production hosts. (`docs/runtime/runtime-loader-rollout-gate.md`, `crates/cli/tests/runtime_loader_rollout_gate.rs`)

### 24 Host Profiles + Milestone_2 Exit
- [x] 24.0 Host capability profile alignment.
  - [x] 24.0.1 Keep `std::crypto`/`std::env`/`std::wasi` host-backed with explicit determinism policies. (`docs/runtime/host-backed-determinism-policy.md`, `docs/runtime/host-imports.md`, `crates/cli/tests/cli_it/imports.rs`, `crates/cli/tests/cli_it/runtime_env.rs`, `crates/cli/tests/cli_it/crypto.rs`)
  - [x] 24.0.2 Expand host-profile docs from 20.1 host-profile v0 bootstrap to full static/contract vs shared/app production policy. (`docs/runtime/host-profiles-production-policy.md`, `docs/runtime/host-imports.md`, `docs/design/phase-20.1.0-host-profile-v0.md`, `crates/cli/tests/host_profile_policy_docs.rs`)
  - [x] 24.0.3 Add host conformance certification suite for deterministic std-host capability behavior across supported runtimes. (`crates/cli/tests/host_conformance_certification.rs`, `docs/runtime/host-profiles-production-policy.md`)
- [x] 24.1 Milestone_2 release gate.
  - [x] 24.1.1 Verify Phases 20-24 completion without regressions to Phase 19 strict/profile guarantees. (`.github/workflows/ci.yml`, `crates/cli/tests/ci_workflow.rs`, `crates/cli/tests/host_conformance_certification.rs`)
  - [x] 24.1.2 Publish `release_notes/milestone_2.md` once gates are green. (`release_notes/milestone_2.md`, `crates/cli/tests/milestone2_release_notes.rs`)
- [x] 24.2 Go-live checklist (must be green before milestone_2 tag).
  - [x] 24.2.1 Runnable baseline: `clg run clearlang-tests/16_namespaced_call.clear` passes in CI and docs clearly mark runnable vs parse-only fixtures. (`.github/workflows/ci.yml`, `crates/cli/tests/ci_workflow.rs`, `clearlang-tests/README.md`)
  - [x] 24.2.2 Precompiled std-core: CI emits versioned artifact + metadata with reproducible hash and import-pruning coverage. (`.github/workflows/ci.yml`, `crates/cli/tests/ci_workflow.rs`, `crates/cli/tests/cli_it/imports.rs`)
  - [x] 24.2.3 Package trust: digest/signature/trust-anchor metadata validation is enforced; malformed/untrusted metadata fails deterministically. (`.github/workflows/ci.yml`, `crates/cli/tests/cli_it/diagnostics.rs`, `crates/cli/tests/phase22_diagnostics_reservation.rs`)
  - [x] 24.2.4 Dependency resolution: transitive resolver + deterministic semver solver + lockfile enforcement are active in CI. (`.github/workflows/ci.yml`, `crates/cli/tests/cli_it/pkg_lock.rs`, `crates/cli/tests/resolver_policy_lock.rs`)
  - [x] 24.2.5 Runtime loader/linker: automatic package loading works and fails closed on missing/mismatch/untrusted artifacts with stable diagnostics. (`.github/workflows/ci.yml`, `crates/cli/tests/run_smoke.rs`, `docs/runtime/runtime-loader-rollout-gate.md`)
  - [x] 24.2.6 Host profiles: static/contract and shared/app profile behavior is documented and covered by integration tests. (`docs/runtime/host-profiles-production-policy.md`, `crates/cli/tests/host_profile_policy_docs.rs`, `crates/cli/tests/host_conformance_certification.rs`)
  - [x] 24.2.7 Assurance/regression gates: Phase 19 strict/profile tests remain green with no assurance-tier regression on protected fixtures. (`.github/workflows/ci.yml`, `crates/cli/tests/profile_regression_gate.rs`, `crates/cli/tests/phase19_design_principles.rs`)
  - [x] 24.2.8 Release readiness: security review, runbooks, signed artifacts, and `release_notes/milestone_2.md` are complete. (`docs/evidence/milestone_2-readiness.md`, `crates/cli/tests/milestone2_readiness_evidence.rs`, `release_notes/milestone_2.md`)
  - [x] 24.2.9 Production SLO/performance gates: package resolution/link latency, startup overhead, and memory/CPU budgets are measured and within defined thresholds. (`xtask/src/main/milestone2_gates.rs`, `.github/workflows/ci.yml`, `docs/evidence/milestone_2-performance.md`, `crates/cli/tests/milestone2_performance_evidence.rs`)
  - [x] 24.2.10 Supply-chain compliance gates: SBOM/license checks for shipped package artifacts and runtime dependencies are green. (`xtask/src/main/milestone2_gates.rs`, `.github/workflows/ci.yml`, `docs/evidence/milestone_2-supply-chain.md`, `crates/cli/tests/milestone2_supply_chain_evidence.rs`)
- [x] 24.3 Milestone_2 delivery governance (execution risk controls).
  - [x] 24.3.1 Assign an explicit owner/DRI for each Phase 20-24 parent task and record it in TODO/planning artifacts.
  - [x] 24.3.2 Add target dates (planned start/end) for each Phase 20-24 parent task and mark critical-path dependencies.
  - [x] 24.3.3 Maintain a milestone_2 risk register (top risks, mitigations, rollback owners) and review weekly.
  - [x] 24.3.4 Add a release-train gate: do not tag milestone_2 unless 24.2.x is fully green and evidence links are attached. (`.github/workflows/ci.yml`, `crates/cli/tests/milestone2_release_train_gate.rs`, `crates/cli/tests/ci_workflow.rs`)



