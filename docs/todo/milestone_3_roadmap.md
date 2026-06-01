# Milestone 3 - Proved Release, Distribution, and Standalone Std (25-26)

This document merges prior Milestone 25 and Milestone 26 planning into one Milestone 3 execution backlog.

## Part A - Milestone 25 Roadmap

### 25 Milestone_3 Roadmap

Execution order for Milestone 3: **policy lock -> proof closure -> release UX -> quality gates -> distribution**.

- [x] 25.0 Release assurance policy (`release == proved`) [Gate A]
  - [x] 25.0.1 Publish milestone_3 design lock with explicit proof completion thresholds and fail-closed policy. (`docs/design/phase-25.0.1-milestone-3-design-lock.md`)
  - [x] 25.0.2 Define theorem-grade proof certification policy (`proved_all`) requiring pure functions, all VCs `proved`, zero assumptions, and strict-mode verification. (`docs/design/phase-25.0.2-theorem-grade-certification-policy.md`)
  - [x] 25.0.3 Lock assurance policy decision: `release == proved`; allow non-proved compile only in non-release/dev workflows. (`docs/design/phase-25.0.3-release-equals-proved-policy.md`)
  - [x] 25.0.4 Enforce release fail-closed policy: block release on any `failed|unknown|timeout|assumed` proof outcome. (`docs/design/phase-25.0.4-fail-closed-release-enforcement.md`)
  - [x] 25.0.5 Emit theorem-grade status in assurance artifacts/signature payload (`proof_status: proved_all|not_proved_all`) with deterministic serialization. (`docs/design/phase-25.0.5-proof-status-emission.md`)
  - [x] 25.0.6 Add verifier policy gate (`clg verify --require-assurance proved_all`) and wire it into release workflows. (`docs/design/phase-25.0.6-verify-require-assurance-gate.md`)
  - [x] 25.0.7 Add release compile profile behavior so production compile/publish fails unless theorem-grade policy passes. (`docs/design/phase-25.0.7-production-release-profile-gate.md`)
  - [x] 25.0.8 Add CI/release gate that blocks publish unless required proof matrix and evidence artifacts are complete. (`docs/design/phase-25.0.8-ci-release-proof-matrix-gate.md`)
  - [x] 25.0.9 Add CI gate asserting release bundles contain zero assumption boundaries (`unsigned.int_model`, `bitwise.uninterpreted`, `crypto.uninterpreted`). (`docs/design/phase-25.0.9-zero-assumption-boundary-release-gate.md`)
  - [x] 25.0.10 Add release-target proof parity gate: current release target (Windows) must produce deterministic proof outcomes and assurance status across identical runs; expand platform set after self-contained solver matrix lands. (`docs/design/phase-25.0.10-cross-platform-proof-parity-gate.md`)
  - [x] 25.0.11 Wire release gate to std proof matrix allowlist: release bundles may include only APIs/symbols marked `proved` in the canonical matrix. (`docs/design/phase-25.0.11-release-symbol-allowlist-gate.md`)
  - [x] 25.0.12 Add strict release-surface policy that only permits features/intrinsics with fully proved semantics (unproved surfaces remain non-release/dev-only). (`docs/design/phase-25.0.12-strict-release-surface-policy.md`)
  - [x] 25.0.13 Enforce crypto proof boundary policy in strict release gates: any remaining `crypto.uninterpreted` boundary blocks theorem-grade/release-grade claims. (`docs/design/phase-25.0.13-crypto-proof-boundary-release-gate.md`)
  - [x] 25.0.14 Align README/docs assurance claims with profile reality (avoid implying `standard` compile is theorem-grade proved). (`docs/design/phase-25.0.14-docs-assurance-profile-alignment.md`)
  - [x] 25.0.15 Keep language surface minimal: defer `theorem` keyword and treat theorem-grade as certification status (not syntax) for milestone_3. (`docs/design/phase-25.0.15-defer-theorem-keyword.md`)

- [x] 25.1 Proof engine and solver closure [Gate B]
  - [x] 25.1.1 Publish Gate B design lock (policy, non-goals, deterministic inputs, and exit criteria) before solver implementation lands. (`docs/design/phase-25.1.1-gate-b-design-lock.md`)
  - [x] 25.1.2 Reserve and document solver-era diagnostics for proof execution/artifact failures (`solver unavailable`, `timeout`, `artifact mismatch`, deterministic replay mismatch). (`docs/design/phase-25.1.2-solver-diagnostics-reservation.md`)
  - [x] 25.1.3 Define `--emit-proof` artifact schema/version and canonical serialization rules (including backward/forward compatibility policy). (`docs/design/phase-25.1.3-emit-proof-schema-v1.md`)
  - [x] 25.1.4 Lock deterministic solver profile as explicit strict input (version/options/timeouts) and include it in release evidence/signature claims. (`docs/design/phase-25.1.4-deterministic-solver-profile-lock.md`)
  - [x] 25.1.5 Version VC/proof schemas for solver-era statuses and add backward/forward compatibility gates for `status` + counterexample fields. (`docs/design/phase-25.1.5-vc-proof-schema-compatibility-gates.md`)
  - [x] 25.1.6 Add solver runtime safety/isolation policy (resource limits, timeout/kill semantics, crash handling) with fail-closed diagnostics mapping. (`docs/design/phase-25.1.6-solver-runtime-safety-isolation.md`)
  - [x] 25.1.7 Close VC soundness gaps so emitted obligations are solver-ready (no placeholder/unconstrained local symbols). (`docs/design/phase-25.1.7-vc-solver-ready-symbol-closure.md`)
  - [x] 25.1.8 Integrate theorem-prover execution path (Z3 baseline) and record deterministic per-VC outcomes (`proved|failed|unknown|timeout`). (`docs/design/phase-25.1.8-z3-vc-outcome-integration.md`)
  - [x] 25.1.9 Add proof artifact emission (`--emit-proof`) and verification wiring in `clg verify`. (`docs/design/phase-25.1.9-proof-artifact-emit-verify-wiring.md`)
  - [x] 25.1.10 Bind proof artifact hash and solver-profile hash into signed payload/manifest and enforce verification consistency in `clg verify`. (`docs/design/phase-25.1.10-proof-solver-hash-binding.md`)
  - [x] 25.1.11 Add deterministic `--emit-proof` artifact reproducibility gate (byte/hash stability across identical inputs and runs). (`docs/design/phase-25.1.11-emit-proof-reproducibility-gate.md`)
  - [x] 25.1.12 Harden solver determinism contract: pin solver version/options/timeouts and add replay-stability CI checks. (`docs/design/phase-25.1.12-solver-determinism-contract-and-replay.md`)
  - [x] 25.1.13 Add CI replay gates for deterministic solver outcomes (`proved|failed|unknown|timeout`) across identical runs and release target platforms (Windows-only for current milestone scope). (`docs/design/phase-25.1.13-ci-replay-gates-release-target-platforms.md`)
  - [x] 25.1.14 Make solver integration self-contained by default (no external system install required) by shipping a platform bundle or Rust-managed vendor path. (`docs/design/phase-25.1.14-self-contained-solver-vendor-path.md`)
  - [x] 25.1.15 Define required self-contained solver support matrix (OS/arch coverage, unsupported-target policy, and CI validation strategy). (`docs/design/phase-25.1.15-solver-support-matrix.md`)
  - [x] 25.1.16 Add solver supply-chain/security gates: pinned version, checksum/signature verification, license/notice inclusion, CVE update policy, and rollback procedure. (`docs/design/phase-25.1.16-solver-supply-chain-security-gates.md`)
  - [x] 25.1.17 Add bitvector proof encoding for covered unsigned paths (starting with `U64`) to retire `unsigned.int_model` assumptions on those paths. (`docs/design/phase-25.1.17-u64-bitvector-bridge-and-unsigned-boundary-retirement.md`)
  - [x] 25.1.18 Add bitwise SMT encoding for covered operators/intrinsics to retire `bitwise.uninterpreted` assumptions on those paths. (`docs/design/phase-25.1.18-u64-bitwise-bv64-encoding.md`)
  - [x] 25.1.19 Close crypto proof-model gaps required for `release == proved` by replacing `crypto.uninterpreted` for release-enabled intrinsic surfaces. (`docs/design/phase-25.1.19-crypto-release-closure.md`)
  - [x] 25.1.20 Define strict-production cutover policy from `generated`/`solver_unavailable` placeholders to solver-required fail-closed behavior. (`docs/design/phase-25.1.20-production-cutover-fail-closed-policy.md`)
  - [x] 25.1.21 Define Gate B completion metric as a machine-checkable closure rule for release-enabled surfaces (explicit zero-boundary + deterministic proof-evidence thresholds). (`docs/design/phase-25.1.21-gate-b-closure-metric.md`)

- [x] 25.2 Release workflow and proved-only UX simplification [Gate C]
  - [x] 25.2.1 Record Gate C discussion conclusions as policy lock: (a) simplified command options with proved-only release assumptions, (b) one-command release path, (c) built-in Z3 migration path, and (d) IDE/VSCode-first CLI contracts. (`docs/design/phase-25.2.1-gate-c-policy-lock.md`, `docs/design/phase-25.2.2-release-ux-design-lock.md`, `docs/design/phase-25.2.3-release-command-orchestration.md`, `docs/design/phase-25.2.4-release-proved-only-default.md`)
  - [x] 25.2.2 Publish Gate C UX design lock: default policy is `release == proved` (`proved_all` required), with shipped primary commands (`clg check`, `clg release`) and `clg test` reserved for `25.3.2`; advanced commands are retained for expert/debug workflows only. (`docs/design/phase-25.2.2-release-ux-design-lock.md`)
  - [x] 25.2.3 Add one-command `clg release` orchestration (lock -> build/prove -> sign -> verify -> release bundle) with fail-closed behavior and minimal required flags. (`docs/design/phase-25.2.3-release-command-orchestration.md`)
  - [x] 25.2.4 Make `clg release` enforce theorem-grade proof by default (no optional downgrade path for release artifacts). (`docs/design/phase-25.2.4-release-proved-only-default.md`)
  - [x] 25.2.5 Add `clg strict init <root>` to generate/validate strict preflight inputs (`clg.project.json`/`clg.lock.json` + trust/profile files) and prefill release defaults to reduce CLI parameters. (`docs/design/phase-25.2.5-strict-init-bootstrap.md`)
  - [x] 25.2.6 Add CLI simplification/deprecation pass: hide and remove flag-heavy legacy release path from primary docs/help and emit migration guidance to `clg release` (no compatibility aliases required pre-production). (`docs/design/phase-25.2.6-release-migration-guidance.md`)
  - [x] 25.2.7 Add `clg check` as a fast deterministic preflight command for local iteration (non-release), aligned with release policy inputs. (`docs/design/phase-25.2.7-check-command.md`)
  - [x] 25.2.8 Add IDE integration contract for VSCode plugin support: stable machine-readable outputs (`--json-errors`, structured stage/progress events, deterministic exit-code mapping) for `check|test|release`. (`docs/design/phase-25.2.8-ide-cli-contract.md`)
  - [x] 25.2.9 Add non-interactive mode guarantees for all primary commands (no prompts, deterministic stdout/stderr separation, plugin-safe logs). (`docs/design/phase-25.2.9-non-interactive-cli-contract.md`)
  - [x] 25.2.10 Add VSCode plugin-facing command profile docs (recommended invocations, expected JSON schema/versioning, cancellation/timeout behavior). (`docs/design/phase-25.2.10-vscode-command-profile.md`)
  - [x] 25.2.11 Add CI contract tests for IDE-facing CLI behavior to prevent breaking plugin integrations across releases. (`docs/design/phase-25.2.11-ide-cli-ci-contract-tests.md`)
  - [x] 25.2.12 Add release-precheck gating so `fmt` + `lint` + tests must pass before strict signed publish flow (local and CI). (`docs/design/phase-25.2.12-release-precheck-gate.md`)
  - [x] 25.2.13 Add `clg fmt` for `.clear` sources with deterministic formatting output. (`docs/design/phase-25.2.13-clg-fmt-command.md`)
  - [x] 25.2.14 Add `clg lint` for `.clear` sources (quality/safety checks) with stable diagnostics and `--deny-warnings` support. (`docs/design/phase-25.2.14-clg-lint-command.md`)
  - [x] 25.2.15 Upgrade solver bundle `.sig` verification from integrity-metadata mode to cryptographic publisher-authenticity verification (pinned vendor key/cert + rotation policy). (`docs/design/phase-25.2.15-solver-signature-authenticity-upgrade.md`)
  - [x] 25.2.16 Add solver backend abstraction (`external-z3-cli` and `rust-z3-lib`) with deterministic backend selection policy. (`docs/design/phase-25.2.16-solver-backend-abstraction-policy.md`)
  - [x] 25.2.17 Implement `rust-z3-lib` backend behind a feature/cutover flag while keeping `external-z3-cli` as temporary fallback. (`docs/design/phase-25.2.17-rust-z3-lib-backend-cutover.md`)
  - [x] 25.2.18 Add determinism parity gate between backends (same VC status outcomes and proof artifact hash for identical strict inputs). (`docs/design/phase-25.2.18-solver-backend-parity-gate.md`)
  - [x] 25.2.19 Add release packaging gate to remove runtime dependency on `tools/proof/z3` for supported release targets once `rust-z3-lib` is cut over. (`docs/design/phase-25.2.19-rust-cutover-packaging-gate.md`)

- [x] 25.3 Unit testing and test runner with deterministic mock support [Gate D]
  - Evidence index (implementation files, including Rust): `docs/design/phase-25.3-evidence-index.md`
  - [x] 25.3.0 Publish Gate D design lock before implementation (scope, non-goals, deterministic contracts, fail-closed policy, completion criteria, and implementation order), and lock Gate D exit criterion: `clg test` is promoted from planned to shipped only after all required `25.3.x` gates are green. (`docs/design/phase-25.3.0-gate-d-design-lock.md`, `docs/design/phase-25.3-evidence-index.md`)
  - [x] 25.3.0.1 Execute Gate D in dependency order: (1) contract/layout/schema foundations (`25.3.1`, `25.3.19`, `25.3.23`) -> (2) runner core (`25.3.2`, `25.3.10`, `25.3.11`, `25.3.14.1`) -> (3) machine-readable outputs (`25.3.3`, `25.3.9`, `25.3.21`) -> (4) deterministic mock system (`25.3.4`, `25.3.15`, `25.3.16`, `25.3.17`, `25.3.24`) -> (5) release/proved safety gates (`25.3.12`, `25.3.13`, `25.3.14`) -> (6) CI/release-precheck + platform determinism (`25.3.7`, `25.3.22`) -> (7) migration/quality/docs closeout (`25.3.6`, `25.3.8`, `25.3.18`, `25.3.25`, `25.3.26`). (`docs/design/phase-25.3-evidence-index.md`)
  - [x] 25.3.1 Define canonical unit-test layout under `tests/` and function naming convention `test_*` (no annotation syntax), including deterministic discovery ordering and duplicate-name conflict diagnostics.
  - [x] 25.3.2 Add `clg test` command that discovers/runs ClearLang unit tests and returns non-zero on failures (zero discovered tests returns success with deterministic `no_tests` summary/report status for local runs; CI/release-precheck policy is explicit and fail-closed).
  - [x] 25.3.3 Add deterministic test reports (`human|json|junit`) with stable failure diagnostics.
  - [x] 25.3.4 Add deterministic mock feature for unit tests (canonical `tests/mocks/` layout, explicit override/binding rules, per-test mock-set selection contract, multi-set precedence/merge policy, and fail-closed signature/effect mismatch diagnostics).
  - [x] 25.3.5 Lock `clg test` proof-mode policy for unit workflows with one deterministic default mode first; keep advanced mode selection as documented follow-up. (`docs/design/phase-25.3.5-test-proof-mode-policy.md`, `docs/design/phase-25.3-evidence-index.md`)
  - [x] 25.3.6 Define Gate D "enough test cases" acceptance matrix using risk/scenario completeness (parser/type/contracts/runtime/mock interactions; positive and negative cases) without fixed numeric thresholds. (`docs/design/phase-25.3-evidence-index.md`)
  - [x] 25.3.7 Integrate `clg test` as a required gate in CI and `xtask release-precheck` with deterministic report schema checks (no smoke-only fallback for Gate D completion). (`docs/design/phase-25.3-evidence-index.md`)
  - [x] 25.3.8 Defer deterministic migration/cutover from `clearlang-tests/` to canonical `tests/` until `clg test` + mock behavior is stable, and document temporary source-of-truth policy plus cutover trigger criteria. (`docs/design/phase-25.3-evidence-index.md`)
  - [x] 25.3.9 Extend machine-readable contracts for `clg test`: add `test` stage support in diagnostics/docs, reserve stable test-specific error code range with explicit code-to-failure mapping (plan/schema/mock/timeout/runtime), and lock `--json-errors`/`--json-events` schema compatibility policy.
  - [x] 25.3.10 Lock deterministic execution model for `clg test` (strictly serial by default, ordering guarantees, seed/replay contract, and shared-state isolation rules), including deterministic timeout policy (default per-test timeout 2 minutes; overrides are policy-driven via `tests/test-plan.json`, not required in minimal CLI flags).
  - [x] 25.3.11 Define baseline Gate D assertion/failure semantics: `test_*() -> Bool` contract, deterministic `assertion_false|runtime|timeout` mapping, stable failure codes (`C139|C138|C137`), and replay/report determinism.
  - [x] 25.3.12 Add release isolation gates so `clg release`/production builds fail closed if module graph references `tests/` or `tests/mocks/`, with deterministic diagnostics.
  - [x] 25.3.13 Add release artifact scan gate proving release bundle/import-map contains production modules only (no test/mock paths).
  - [x] 25.3.14 Add tamper-evidence verification tests showing mock/test substitutions fail signed artifact verification/release acceptance.
  - [x] 25.3.14.1 Lock retry policy for deterministic quality gates: no automatic retries in `clg test` default/CI/release-precheck paths; rerun is explicit user action.
  - [x] 25.3.15 Add `tests/test-plan.json` schema v1 for deterministic case selection + per-test mock-set binding (`test_id -> mock_sets[]`) used by `clg test`.
  - [x] 25.3.16 Enforce explicit per-test mock binding policy (no implicit hidden mock fallback for tests that declare mock dependencies); missing bindings fail closed with deterministic diagnostics.
  - [x] 25.3.17 Add mock-state isolation contract so test cases cannot leak mutable mock state across runs (fresh test context or deterministic reset requirement).
  - [x] 25.3.18 Add balanced quality gate requiring non-mocked test coverage alongside mocked scenarios for critical paths (to avoid mock-only confidence). (`docs/design/phase-25.3-evidence-index.md`)
  - [x] 25.3.19 Lock minimal `clg test` CLI contract surface for production use (`<path?>`, `--filter`, `--report human|json|junit`) with deterministic argument validation and stable diagnostics.
  - [x] 25.3.19.1 Defer non-essential test flags (`--plan`, `--mock-set`, `--timeout-ms`, `--fail-fast`, `--list`) unless a concrete production workflow requires them; keep equivalent behavior deterministic via `tests/test-plan.json` policy and runner defaults. (`docs/design/phase-25.3.19.1-test-cli-deferred-flags-policy.md`, `docs/design/phase-25.3-evidence-index.md`)
  - [x] 25.3.20 Lock suite execution policy for failures (default run-all with deterministic final exit/result aggregation); keep fail-fast as deferred/non-essential unless a concrete production workflow requires it.
  - [x] 25.3.21 Add deterministic per-test artifact/replay contract (captured stdout/stderr, selected mock sets, timeout/failure reason, and single-test replay command flow for failing cases).
  - [x] 25.3.22 Add cross-platform determinism gate for `clg test` outputs and report shape on milestone release targets (Windows baseline + Linux parity). (`docs/design/phase-25.3-evidence-index.md`)
  - [x] 25.3.23 Lock `tests/test-plan.json` governance policy (ordering canonicalization, duplicate `test_id` rejection, unknown mock-set handling, missing-entry behavior, and schema migration/versioning).
  - [x] 25.3.24 Add path-resolution safety gates for `tests/` and `tests/mocks/` loading (reject traversal/symlink escape and non-canonical out-of-root bindings) with fail-closed diagnostics.
  - [x] 25.3.25 Extend deterministic runtime safety policy for test workers beyond timeout (memory/fuel limits, crash handling, and deterministic status mapping). (`docs/design/phase-25.3-evidence-index.md`)
  - [x] 25.3.26 Update primary UX/docs/help contracts after `clg test` lands (README, release-process, IDE profile, diagnostics table, CLI help/about string) and add drift gates to prevent stale command-surface docs. (`docs/design/phase-25.3-evidence-index.md`)
  - [x] 25.3.27 Add minimal `std::unit` assertion package for Gate D ergonomics without expanding CLI/test-plan surface (`assert_true`, `assert_eq_int`, `assert_eq_bool`, `fail`).
  - [x] 25.3.28 Close Gate D negative-evidence gaps in `clg test` contract tests: add deterministic integration coverage for `C134` (discovery/layout/signature), `C135` (plan/schema governance), `C137` (timeout), and `C138` (runtime failure mapping).
  - [x] 25.3.29 Lock policy that `clg test` single-process rule is documentation/operational guidance only for Gate D (no runtime project-lock mechanism in this phase). (`docs/design/phase-25.3.0-gate-d-design-lock.md`, `docs/testing.md`)
  - [x] 25.3.30 Clarify proof boundary for tests: theorem-grade assurance (`proved_all`) applies to production source/release artifacts; `tests/` are quality-validation inputs only. (`docs/design/phase-25.3.5-test-proof-mode-policy.md`, `docs/testing.md`)
  - [x] 25.3.31 Add deterministic integration tests for `C136` unsafe mock-path rejection (`symlink`, non-canonical out-of-root resolution, and traversal-like bindings) to match Gate D path-safety claims.
  - [x] 25.3.32 Lock phased rollout/governance for `std::unit` API: Gate D ships minimal subset first (`assert_true`, `assert_eq_int`, `assert_eq_bool`, `fail`), remaining v1 methods are tracked as additive follow-ups only (no breaking signature/behavior changes). (`docs/design/phase-25.3.32-std-unit-rollout-governance-policy.md`, `docs/testing.md`)
  - [x] 25.3.33 Lock `assert_eq_bytes` activation policy: keep method reserved/conditional until `Bytes` is confirmed in std-core scope with deterministic typing/runtime support, and require deterministic unsupported-diagnostic behavior before activation. (`docs/design/phase-25.3.33-assert-eq-bytes-activation-policy.md`, `docs/testing.md`)

- [x] 25.4 Clear project manifests (`json`) [Gate E]
  - [x] 25.4.1 Define `clg.project.json` as the user-authored project/dependency manifest (project name/description/version, `clg` compiler version range, website/contact metadata, declared package requirements). (`docs/design/phase-25.4.1-project-manifest-v1.md`)
  - [x] 25.4.2 Keep `clg.lock.json` as the tool-generated deterministic lockfile (exact versions, digests, and resolved graph identity). (`docs/design/phase-25.4.2-lockfile-tool-generated-contract.md`)
  - [x] 25.4.3 Add resolver flow: `clg pkg lock --generate|--update` reads `clg.project.json` (schema v1) and writes canonical lock outputs.
  - [x] 25.4.4 Ensure imports in `.clear` remain version-free (logical module/package paths only); versions live only in project/lock JSON. (`docs/design/phase-25.4.4-version-free-imports.md`)
  - [x] 25.4.5 Add schema docs, migration notes, and CI drift gates that fail on manifest/lock inconsistency. (`docs/design/phase-25.4.5-manifest-lock-schema-migration-and-drift-gate.md`)
  - [x] 25.4.6 Define migration/coexistence policy from canonical package metadata/ABI inputs to `clg.project.json` + `clg.lock.json`, with deterministic conflict diagnostics. (`docs/design/phase-25.4.6-manifest-lock-migration-coexistence-policy.md`)
  - [x] 25.4.7 Define pre-GA fail-closed cutover policy that removes legacy inputs/flags; no backward-compatibility commitment before GA. (`docs/design/phase-25.4.7-pre-ga-cutover-policy.md`)
  - [x] 25.4.8 Add migration tooling command/docs (`clg pkg migrate-manifest`) to generate `clg.project.json` from existing canonical metadata inputs. (`docs/design/phase-25.4.8-migrate-manifest-command.md`)
  - [x] 25.4.9 Remove legacy non-secret release parameters (`--advisory-as-of`, `--key-id`, `--out-dir`, `--trust-policy`) from primary `clg release` UX once manifest defaults are complete; keep key material flags explicit. (`docs/design/phase-25.4.9-release-legacy-parameter-retirement.md`)
  - [x] 25.4.10 Enforce `clg.lock.json` tool-owned contract with CI drift gate (manual lockfile edits or manifest/lock mismatch fail closed). (`docs/design/phase-25.4.10-lockfile-tool-owned-drift-gate.md`)
  - [x] 25.4.11 Clarify trust-policy UX contract (compile-time `trust-policy.json` vs strict package `clg.trust-policy.json`) and ensure manifest/docs/diagnostics use unambiguous naming. (`docs/design/phase-25.4.11-trust-policy-ux-clarification.md`)
  - [x] 25.4.12 Move release entrypoint into `clg.project.json` (`project.entry`) and remove positional `<FILE>` from `clg release`; release must fail closed when schema v1 entry metadata is missing. (`docs/design/phase-25.4.12-release-entrypoint-manifest-cutover.md`)

- [x] 25.5 Literal ergonomics for low-level/crypto code
  - [x] 25.5.1 Add integer literal support for `0x...` (hex) and `0b...` (binary) with deterministic parsing, underscore rules, and diagnostics.
    - [x] Lock lexical grammar in parser docs/tests:
          `hex_literal := 0[xX] hexdigit (hexdigit|_)*`,
          `bin_literal := 0[bB] bindigit (bindigit|_)*`.
    - [x] Lock underscore policy (deterministic/fail-closed):
          underscores allowed only between digits; reject underscore immediately after prefix, trailing underscore, and repeated adjacent underscores.
    - [x] Lock sign-token policy:
          `-` is not part of integer literal tokens; prefixed literals are parsed as non-negative literal tokens.
    - [x] Add deterministic parse diagnostics for:
          missing digits after base prefix, invalid digit for base, invalid underscore placement.
    - [x] Add parser coverage matrix with stable pass/fail snapshots for valid/invalid examples (mixed case prefixes/digits included).
  - [x] 25.5.2 Extend typing/inference rules to base-prefixed literals with deterministic diagnostics.
    - [x] Preserve existing numeric policy: base-prefixed literals follow current literal typing rules (default `Int`, expected-type coercion for unsigned contexts, explicit cast paths).
    - [x] Add deterministic range-fit checks for contextual unsigned typing (`U8`, `U64`, `U128`, `U256`) on base-prefixed literals, aligned with existing unsigned literal diagnostics.
    - [x] Lock non-negative constraint behavior for unsigned contexts/casts with prefixed literals (prefixed literal tokens are non-negative, and unsigned cast/context checks remain deterministic).
    - [x] Add typer regression tests covering:
          `Int` default, contextual unsigned acceptance, overflow/range rejection, and deterministic error payload shape.
  - [x] 25.5.3 Add VC/proof regression coverage for hex/binary literals in bitwise/unsigned paths to ensure no proof determinism regressions.
    - [x] Add proof snapshots where equivalent decimal vs hex/binary sources produce the same VC status outcomes under identical strict inputs.
    - [x] Add bitwise/shift regression cases using prefixed literals on covered unsigned paths (`U64`) and ensure no new assumption drift.
    - [x] Add CI determinism checks for proof artifact/report stability (status + hash invariants) when using base-prefixed literals.

- [x] 25.6 First usable binary releases
  - [x] 25.6.0 Lock Phase 25.6 artifact/provenance policy before implementation (GA artifact = distributable `clg` binaries; provenance required for GA release-train artifacts; local/dev may remain non-GA). (`docs/design/phase-25.6.0-binary-ga-and-provenance-policy-lock.md`)
  - [x] 25.6.1 Lock GA target matrix and support policy (baseline required gates: Windows + Linux; macOS is preview/non-blocking in this phase). (`docs/design/phase-25.6.0-binary-ga-and-provenance-policy-lock.md`)
  - [x] 25.6.2 Produce signed release binaries with reproducible metadata, checksums, and SBOM/license bundles. (`xtask milestone3-binary-bundle`, `.github/workflows/ci.yml`, `docs/design/phase-25.6.2-signed-binary-bundle.md`)
  - [x] 25.6.3 Publish install/upgrade/uninstall/verify docs and add smoke coverage per GA target. (`docs/release/milestone_3-binary-operations.md`, `.github/workflows/ci.yml`)
  - [x] 25.6.4 Add release-train checklist plus rollback/incident runbook for binary distribution failures. (`docs/release/milestone_3-release-train-checklist.md`, `docs/release/milestone_3-binary-incident-runbook.md`)
  - [x] 25.6.5 Publish `release_notes/milestone_3.md` with compatibility matrix, known limitations, and upgrade notes. (`release_notes/milestone_3.md`)
  - [x] 25.6.6 Add single distributable release bundle artifact contract (package all required release outputs + detached checksum/signature) to simplify operator workflow. (`docs/design/phase-25.6.6-single-distributable-bundle-contract.md`)
  - [x] 25.6.7 Add `clg verify-bundle` command that verifies from release bundle manifest without manual per-file wiring.
  - [x] 25.6.8 Add keyring-by-`key_id` verification workflow for rotated release keys (historical verification without manual key selection ambiguity).
  - [x] 25.6.9 Add deterministic release readiness gate command (`clg release --check-only` or equivalent) for pre-signing/operator preflight.
  - [x] 25.6.10 Add release-bundle provenance attestation contract (signed builder/provenance statement with deterministic schema); provenance is required for GA release-train artifacts and `clg verify-bundle` must fail closed when required provenance is missing/invalid. (`docs/design/phase-25.6.10-release-bundle-provenance-contract.md`)
  - [x] 25.6.11 Add fail-closed negative test matrix for `clg verify-bundle` (tampered manifest, artifact hash mismatch, detached signature mismatch, unknown/revoked `key_id`, and truncated/missing bundle members) with deterministic diagnostics.
    - [x] Tampered bundle manifest fails closed with deterministic diagnostics.
    - [x] Artifact hash mismatch fails closed with deterministic diagnostics.
    - [x] Detached signature mismatch fails closed with deterministic diagnostics.
    - [x] Truncated/missing bundle members fail closed with deterministic diagnostics.
    - [x] Unknown/revoked `key_id` flow (depends on 25.6.8 keyring-by-`key_id` workflow).
  - [x] 25.6.12 Add cross-runner reproducibility witness gate for GA binaries (independent clean builders produce identical bytes/hashes per supported target, or fail release-train gate). (`xtask binary-repro-witness`, `.github/workflows/ci.yml`)
  - [x] 25.6.13 Lock binary publication/distribution policy (supported channels, metadata/signature/checksum parity requirements, and explicit defer list for unsupported channels) before GA. (`docs/design/phase-25.6.13-binary-publication-policy-lock.md`)


## Part B - Milestone 26 Standalone Standard Library (Critical)

# Milestone 26 - Standalone Standard Library Phase (Critical)

Execution order for std proof coverage: **scope lock -> core coverage -> set subset -> set ops -> cardinality -> list/map completion**.
Linking policy for first production release: **embed std into target artifacts with deterministic dead-code elimination**.
Dynamic/shared std runtime linking is explicitly **deferred** to a follow-up phase after first production release.

- [x] 26.0 Std scope and governance [Std Gate A]
  - [x] 26.0.1 Lock v1 std scope as `must-have` vs `stretch` and publish explicit defer list for unresolved `stretch` symbols. (`docs/design/phase-26.0.1-std-scope-and-governance-lock.md`, `docs/std/README.md`)
  - [x] 26.0.2 Lock first-production std policy: embedded linking only, deterministic dead-code elimination/tree-shaken emission, and size-regression guardrails. (`docs/design/phase-26.0.0-std-embedded-first-policy-lock.md`, `docs/design/phase-26.0.1-std-scope-and-governance-lock.md`)
  - [x] 26.0.3 Lock post-first-production roadmap: dynamic/shared std linking remains deferred with explicit activation gates, plus std stability/versioning policy before public GA. (`docs/design/phase-26.0.0-std-embedded-first-policy-lock.md`, `docs/design/phase-26.0.1-std-scope-and-governance-lock.md`)

- [x] 26.1 Std implementation and release gates [Std Gate B]
  - [x] 26.1.0 Lock `std::core` first-production API cut from `docs/std/core.md`. (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`, `docs/std/core.md`)
    - [x] 26.1.0.1 Keep `Option<T>` baseline methods for first production: `is_some`, `is_none`, `map`, `and_then`, `filter`, `or_else`, `unwrap_or`, `unwrap_or_else`, `expect`, `to_result`. (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`)
    - [x] 26.1.0.2 Keep `Result<T,E>` baseline methods for first production: `is_ok`, `is_err`, `map`, `map_err`, `and_then`, `or_else`, `unwrap_or`, `unwrap_or_else`, `expect`, `expect_err`, `to_option`. (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`)
    - [x] 26.1.0.3 Keep `ErrorCode` baseline methods for first production: `new`, `value`, `equals`. (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`)
    - [x] 26.1.0.4 Keep `CoreError` + `Panic` baseline methods for first production: `CoreError::{new,with_message,code,message,equals}` and `Panic::fail`. (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`)
    - [x] 26.1.0.5 Defer advanced `std::core` helpers until post-first-production unless required by a concrete blocker (`flatten`, `contains*`, `map_or*`, `to_result_else`, `domain/cause` fields). (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`)
    - [x] 26.1.0.6 Add deterministic diagnostics + contract tests for `expect`/`expect_err`/`Panic::fail` failure paths. (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`)
  - [x] 26.1.1 Implement all `must-have` std functions/types with deterministic typing/lowering/runtime behavior (excluding explicitly deferred Gate C set-proof surfaces).
  - [x] 26.1.2 Add full std coverage matrix (`typed|runtime|proved` per symbol) with CI drift gates.
  - [x] 26.1.3 Keep strict package metadata/ABI/import-pruning/trust gates green for expanded std surface.
  - [x] 26.1.4 Add package-level execution slices so every `docs/std/README.md` package has an explicit Phase 26 owner task.
    - [x] 26.1.4.1 `std::str`: lock first-production API cut + deterministic UTF-8 diagnostics/contracts, including stable `str_pattern::matches(pattern, input)`. (`docs/design/phase-26.1.4.1-std-str-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-str-owner`, `Target: 2026-06-05`.
    - [x] 26.1.4.2 `std::bytes`: lock first-production API cut + constant-time compare contract/coverage. (`docs/design/phase-26.1.4.2-std-bytes-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-bytes-owner`, `Target: 2026-06-10`.
    - [x] 26.1.4.3 `std::int`: lock first-production API cut + checked/wrapping/saturating semantics coverage, plus checked `div/mod`, bitwise (`and/or/xor/not`), and checked shift/rotate contracts for `U64/U128/U256`. (`docs/design/phase-26.1.4.3-std-int-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-int-owner`, `Target: 2026-06-13`.
    - [x] 26.1.4.4 `std::codec`: lock first-production canonical encoding/decoding API + deterministic error surface, including decoder safety helpers (`position`, `remaining`, `read_fixed`). (`docs/design/phase-26.1.4.4-std-codec-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-codec-owner`, `Target: 2026-06-18`.
    - [x] 26.1.4.5 `std::crypto`: lock first-production API cut + proof/assurance boundary policy for enabled intrinsics, including `verify_result::{is_valid,error_or_none,valid,invalid}` invariant enforcement and explicit negative tests rejecting mixed/invalid states. (`docs/design/phase-26.1.4.5-std-crypto-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-crypto-owner`, `Target: 2026-06-23`.
    - [x] 26.1.4.6 `std::host`: lock first-production capability surface + fail-closed host-profile conformance tests, including deterministic `Result<Bool, HostError>` semantics for `storage::set/delete` and `log::{info,warn,error}` (success must be `Ok(true)`). (`docs/design/phase-26.1.4.6-std-host-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-host-owner`, `Target: 2026-06-25`.
    - [x] 26.1.4.7 `std::contract`: lock first-production chain-agnostic contract-domain API cut. (`docs/design/phase-26.1.4.7-std-contract-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-contract-owner`, `Target: 2026-06-27`.
    - [x] 26.1.4.8 `std::unit`: align first-production API cut with Gate D baseline (`assert_true`, `assert_eq_int`, `assert_eq_bool`, `fail`) plus deterministic failure mapping, contract tests for method-name stability, and fail-closed rejection of non-baseline assertion names in production profile. (`docs/design/phase-26.1.4.8-std-unit-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-unit-owner`, `Target: 2026-06-29`.
    - [x] 26.1.4.9 `std::chain::<target>`: keep deferred by default; add target-specific activation checklist for launches that require it. (`docs/design/phase-26.1.4.9-std-chain-target-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-chain-owner`, `Target: 2026-07-01`.
    - [x] 26.1.4.10 `std::dynamic`: keep deferred post-first-production; require explicit activation gates before any runtime-link implementation. (`docs/design/phase-26.1.4.10-std-dynamic-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-dynamic-owner`, `Target: 2026-07-02`.
    - [x] 26.1.4.11 `std::list` (from collections catalog): lock deterministic out-of-range behavior for `insert/remove` and add non-terminating `insert_checked/remove_checked` conformance tests. (`docs/design/phase-26.1.4.11-std-list-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-list-owner`, `Target: 2026-07-04`.
    - [x] 26.1.4.12 `std::set` (from collections catalog): lock deterministic semantics for `contains/insert/remove` and conformance for mutation/non-mutation paths. (`docs/design/phase-26.1.4.12-std-set-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-set-owner`, `Target: 2026-07-05`.
    - [x] 26.1.4.13 `std::map` (from collections catalog): lock deterministic semantics for `contains/get/insert/remove` and take/mut variants with stable error behavior. (`docs/design/phase-26.1.4.13-std-map-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-map-owner`, `Target: 2026-07-06`.

- [x] 26.2 Finite-set proof roadmap (ordered execution) [Std Gate C]
  - [x] 26.2.0 Publish Gate C design lock with ordered execution, assumption-boundary policy, required evidence artifacts, and deterministic performance guardrails. (`docs/design/phase-26.2.0-finite-set-proof-design-lock.md`)
  - [x] 26.2.1 Add `std::set::subset(a, b) -> Bool` API with typing/lowering/runtime coverage and deterministic regression tests; update coverage row from `deferred` to implemented state. (`docs/std/coverage-matrix.md`, `docs/design/phase-26.2.0-finite-set-proof-design-lock.md`)
  - [x] 26.2.2 Add finite-set VC/SMT reasoning for membership + subset and enforce strict no-assumption gate for theorem-grade claims over set properties (`assumptions.items == []` on set VCs in strict release workflow). (`docs/proofs/proof-coverage-matrix.{md,json}`, `docs/std/coverage-matrix.md`, `docs/design/phase-26.2.0-finite-set-proof-design-lock.md`)
  - [x] 26.2.3 Sequence rule: complete subset proof support first (API + VC/SMT + tests) before expanding other set operators; fail closed if `union/intersect/diff` are enabled early. (`docs/std/coverage-matrix.md`)
  - [x] 26.2.4 Sequence rule: add proof support for `union`/`intersect`/`diff` after subset is complete and stable; update std/proof coverage matrices per operator. (`docs/std/coverage-matrix.md`, `docs/proofs/proof-coverage-matrix.{md,json}`)
  - [x] 26.2.5 Sequence rule: add cardinality-heavy proofs last (`len`, bounds, set-size relations) with deterministic solver performance guardrails (median `<= +10%`, p95 `<= +20%`, zero timeouts on release-enabled finite-set fixtures). (`docs/std/coverage-matrix.md`, `docs/evidence/phase-26.2-finite-set-performance.{md,json}`)

- [x] 26.3 List proof roadmap [Std Gate D]
  - [x] 26.3.0 Publish Gate D design lock with ordered execution, assumption-boundary policy, required evidence artifacts, and deterministic solver-performance guardrails. (`docs/design/phase-26.3.0-list-proof-design-lock.md`) `DRI: std-list-owner`, `Target: 2026-07-08`.
  - [x] 26.3.1 Sequence rule: close read-only list proofs first (`new`, `len`, `is_empty`, `get`) with deterministic bounds diagnostics and regression coverage; fail closed if `push/pop/insert/remove/remove_take` proofs are enabled before read-only closure is complete. (`docs/std/coverage-matrix.md`, `docs/design/phase-26.3.0-list-proof-design-lock.md`) `DRI: std-list-owner`, `Target: 2026-07-10`.
  - [x] 26.3.2 Add VC/SMT reasoning for index safety invariants (`0 <= i < len`) and `get` value-preservation under unchanged list state. (`docs/proofs/proof-coverage-matrix.{md,json}`, `docs/std/coverage-matrix.md`, `docs/design/phase-26.3.0-list-proof-design-lock.md`) `DRI: std-list-owner`, `Target: 2026-07-12`.
  - [x] 26.3.3 Sequence rule: close append/pop semantics next (`push`, `pop`) with deterministic list-length/emptiness invariants (`push => len + 1`, `pop some <=> len > 0`, `pop none <=> len == 0`); fail closed if `insert/remove/remove_take` proofs are enabled before append/pop closure is complete. (`docs/std/coverage-matrix.md`, `docs/design/phase-26.3.0-list-proof-design-lock.md`) `DRI: std-list-owner`, `Target: 2026-07-14`.
  - [x] 26.3.4 Sequence rule: close indexed mutation semantics after 26.3.2/26.3.3 (`insert`, `remove`, `remove_take`) with shape/order preservation and deterministic out-of-range behavior. (`docs/std/coverage-matrix.md`, `docs/proofs/proof-coverage-matrix.{md,json}`, `docs/design/phase-26.3.0-list-proof-design-lock.md`) `DRI: std-list-owner`, `Target: 2026-07-16`.
  - [x] 26.3.5 Add theorem-grade no-assumption gate for release-enabled list proofs (`assumptions.items == []` on list VCs in strict release workflow); fail closed on any list proof regression. (`docs/proofs/proof-coverage-matrix.{md,json}`, `docs/design/phase-26.3.0-list-proof-design-lock.md`) `DRI: std-list-owner`, `Target: 2026-07-18`.
  - [x] 26.3.6 Update proof/coverage evidence for each list symbol as it closes (`docs/std/coverage-matrix.md`, `docs/proofs/proof-coverage-matrix.{md,json}`), including compatibility rows (`insert_checked`, `remove_checked`) when enabled. (`docs/std/coverage-matrix.md`, `docs/proofs/proof-coverage-matrix.{md,json}`) `DRI: std-list-owner`, `Target: 2026-07-19`.
  - [x] 26.3.7 Add deterministic performance guardrails for list-proof fixtures (median `<= +10%`, p95 `<= +20%`, zero timeouts on release-enabled list fixtures) and publish evidence artifact (`docs/evidence/phase-26.3-list-performance.{md,json}`). (`docs/evidence/phase-26.3-list-performance.{md,json}`, `docs/design/phase-26.3.0-list-proof-design-lock.md`) `DRI: std-list-owner`, `Target: 2026-07-21`.

- [x] 26.4 Map proof roadmap [Std Gate E]
  - [x] 26.4.0 Publish Gate E design lock with ordered execution, assumption-boundary policy, required evidence artifacts, and deterministic solver-performance guardrails. (`docs/design/phase-26.4.0-map-proof-design-lock.md`) `DRI: std-map-owner`, `Target: 2026-07-22`.
  - [x] 26.4.1 Sequence rule: close read-only map proofs first (`new`, `len`, `is_empty`, `contains`, `get`) with deterministic key-presence diagnostics and regression coverage; fail closed if mutation map proofs are enabled before read-only closure is complete. (`docs/std/coverage-matrix.md`, `docs/design/phase-26.4.0-map-proof-design-lock.md`) `DRI: std-map-owner`, `Target: 2026-07-24`.
  - [x] 26.4.2 Add VC/SMT reasoning for key-membership/value-consistency invariants across unchanged map state and deterministic overwrite semantics for `insert`/`insert_take`. (`docs/proofs/proof-coverage-matrix.{md,json}`, `docs/std/coverage-matrix.md`, `docs/design/phase-26.4.0-map-proof-design-lock.md`) `DRI: std-map-owner`, `Target: 2026-07-26`.
  - [x] 26.4.3 Sequence rule: close mutation map proof semantics next (`insert`, `insert_take`, `remove`, `remove_take`) with deterministic present/absent-key behavior, plus alias-conformance checks for `insert_mut`/`remove_mut`; fail closed if release enables mutation proofs before 26.4.2 closure. (`docs/std/coverage-matrix.md`, `docs/proofs/proof-coverage-matrix.{md,json}`, `docs/design/phase-26.4.0-map-proof-design-lock.md`) `DRI: std-map-owner`, `Target: 2026-07-28`.
  - [x] 26.4.4 Add theorem-grade no-assumption gate for release-enabled map proofs (`assumptions.items == []` on map VCs in strict release workflow); fail closed on any map proof regression. (`docs/proofs/proof-coverage-matrix.{md,json}`, `docs/design/phase-26.4.0-map-proof-design-lock.md`) `DRI: std-map-owner`, `Target: 2026-07-30`.
  - [x] 26.4.5 Update proof/coverage evidence for each map symbol as it closes (`docs/std/coverage-matrix.md`, `docs/proofs/proof-coverage-matrix.{md,json}`), including compatibility rows (`can_mut`, `insert_mut`, `remove_mut`) when enabled. (`docs/std/coverage-matrix.md`, `docs/proofs/proof-coverage-matrix.{md,json}`) `DRI: std-map-owner`, `Target: 2026-07-31`.
  - [x] 26.4.6 Add deterministic performance guardrails for map-proof fixtures (median `<= +10%`, p95 `<= +20%`, zero timeouts on release-enabled map fixtures) and publish evidence artifact (`docs/evidence/phase-26.4-map-performance.{md,json}`). (`docs/evidence/phase-26.4-map-performance.{md,json}`, `docs/design/phase-26.4.0-map-proof-design-lock.md`) `DRI: std-map-owner`, `Target: 2026-08-02`.
  - [x] 26.4.7 Publish post-Gate-E map iteration/ordering expansion lock (`keys`, `values`, `entries` and/or iterators) with canonical ordering and deterministic serialization constraints; keep these APIs release-disabled until that lock + conformance tests + coverage rows land. (`docs/design/phase-26.4.7-map-iteration-ordering-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-map-owner`, `Target: 2026-08-05`.

- [x] 26.5 Deferred test assertion extensions [Std Gate F]
  - [x] 26.5.1 Add expected-failure and trap/error assertion semantics for `clg test` only after baseline `std::unit` assertions are stable. (`docs/design/phase-26.5-test-assertion-extensions-lock.md`, `docs/testing.md`, `crates/cli/src/commands/test/{plan.rs,execution.rs,report.rs}`)
  - [x] 26.5.2 Add deterministic assertion-mismatch diff shape and deterministic failure-id taxonomy for advanced assertion paths. (`docs/design/phase-26.5-test-assertion-extensions-lock.md`, `docs/testing.md`, `crates/cli/src/commands/test/{prelude.rs,execution.rs,report.rs}`)
  - [x] 26.5.3 Evaluate generic `assert_eq<T>` only with explicit equality-capability constraints and deterministic diagnostics policy. (`docs/design/phase-26.5-test-assertion-extensions-lock.md`, `docs/std/unit.md`)

- [x] 26.6 Std architecture debt cleanup (long-term) [Std Gate G]
  - [x] 26.6.0 Publish architecture lock for long-term std implementation model (single-source std definitions, generation pipeline, and compatibility strategy). (`docs/design/phase-26.6-std-architecture-lock.md`) `Completed: 2026-05-31`.
  - [x] 26.6.1 Introduce canonical std source-of-truth package layout (ClearLang-first where feasible) and classify symbols as `pure_std` vs `host_std`. (`docs/design/phase-26.6-std-catalog.lock.json`) `Completed: 2026-05-31`.
  - [x] 26.6.2 Generate `std-metadata`/builtin signatures from canonical source (no hand-maintained duplicate symbol tables across typer/codegen/cli). (`xtask std-arch-sync`, `crates/cli/assets/std-metadata.json`, `docs/design/phase-26.6-std-builtin-signatures.v1.json`) `Completed: 2026-05-31`.
  - [x] 26.6.3 Add per-symbol conformance harness enforcing doc/signature/lowering/runtime parity and fail CI on drift. (`xtask std-arch-conformance-check`, `xtask release-precheck`) `Completed: 2026-05-31`.
  - [x] 26.6.4 Migrate collection/string/bytes/int surfaces from stage-specific duplicated logic to shared generated contracts + thin adapters. (`crates/typer/src/guards.rs` canonical alias helper + catalog-governed signature/effect/route parity in `std-arch-conformance-check`) `Completed: 2026-05-31`.
  - [x] 26.6.5 Lock host-capability boundary model (`std::host` ownership, import mapping, deterministic fail-closed behavior) and remove ambiguous overlaps (`std::env`/`std::wasi` compatibility paths only where explicitly approved). (`docs/design/phase-26.6-std-architecture-lock.md`, `docs/design/phase-26.6-std-catalog.lock.json`) `Completed: 2026-05-31`.
  - [x] 26.6.6 Add deprecation/cutover plan for legacy intrinsic aliases and compatibility shims, with deterministic diagnostics and migration notes. (`deprecated_alias_of` catalog contract in `docs/design/phase-26.6-std-catalog.lock.json`, migration policy in `docs/design/phase-26.6-std-architecture-lock.md`) `Completed: 2026-05-31`.
  - [x] 26.6.7 Add Gate G completion criteria: zero unmanaged std symbol definitions, green conformance matrix, and stable release-grade std architecture docs. (`docs/design/phase-26.6-std-architecture-lock.md`) `Completed: 2026-05-31`.
  - [x] 26.6.8 Replace placeholder `can_mut` semantics (`true` predicate) with explicit ownership/uniqueness model or lock a permanent policy rationale; include deterministic diagnostics, migration notes, and conformance evidence for list/set/map mut guards. (`docs/design/phase-26.6.8-can-mut-semantics-lock.md`, `docs/typing.md`, `docs/std/coverage-matrix.md`) `DRI: std-arch-owner`, `Completed: 2026-05-30`.
