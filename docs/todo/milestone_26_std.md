# Milestone 26 - Standalone Standard Library Phase (Critical)

Execution order for std proof coverage: **scope lock -> core coverage -> set subset -> set ops -> cardinality -> list/map completion**.
Linking policy for first production release: **embed std into target artifacts with deterministic dead-code elimination**.
Dynamic/shared std runtime linking is explicitly **deferred** to a follow-up phase after first production release.

- [x] 26.0 Std scope and governance [Std Gate A]
  - [x] 26.0.1 Lock v1 std scope as `must-have` vs `stretch` and publish explicit defer list for unresolved `stretch` symbols. (`docs/design/phase-26.0.1-std-scope-and-governance-lock.md`, `docs/std/README.md`)
  - [x] 26.0.2 Lock first-production std policy: embedded linking only, deterministic dead-code elimination/tree-shaken emission, and size-regression guardrails. (`docs/design/phase-26.0.0-std-embedded-first-policy-lock.md`, `docs/design/phase-26.0.1-std-scope-and-governance-lock.md`)
  - [x] 26.0.3 Lock post-first-production roadmap: dynamic/shared std linking remains deferred with explicit activation gates, plus std stability/versioning policy before public GA. (`docs/design/phase-26.0.0-std-embedded-first-policy-lock.md`, `docs/design/phase-26.0.1-std-scope-and-governance-lock.md`)

- [ ] 26.1 Std implementation and release gates [Std Gate B]
  - [x] 26.1.0 Lock `std::core` first-production API cut from `docs/std/core.md`. (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`, `docs/std/core.md`)
    - [x] 26.1.0.1 Keep `Option<T>` baseline methods for first production: `is_some`, `is_none`, `map`, `and_then`, `filter`, `or_else`, `unwrap_or`, `unwrap_or_else`, `expect`, `to_result`. (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`)
    - [x] 26.1.0.2 Keep `Result<T,E>` baseline methods for first production: `is_ok`, `is_err`, `map`, `map_err`, `and_then`, `or_else`, `unwrap_or`, `unwrap_or_else`, `expect`, `expect_err`, `to_option`. (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`)
    - [x] 26.1.0.3 Keep `ErrorCode` baseline methods for first production: `new`, `value`, `equals`. (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`)
    - [x] 26.1.0.4 Keep `CoreError` + `Panic` baseline methods for first production: `CoreError::{new,with_message,code,message,equals}` and `Panic::fail`. (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`)
    - [x] 26.1.0.5 Defer advanced `std::core` helpers until post-first-production unless required by a concrete blocker (`flatten`, `contains*`, `map_or*`, `to_result_else`, `domain/cause` fields). (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`)
    - [x] 26.1.0.6 Add deterministic diagnostics + contract tests for `expect`/`expect_err`/`Panic::fail` failure paths. (`docs/design/phase-26.1.0-std-core-first-production-api-lock.md`)
  - [ ] 26.1.1 Implement all `must-have` std functions/types with deterministic typing/lowering/runtime behavior.
  - [ ] 26.1.2 Add full std coverage matrix (`typed|runtime|proved` per symbol) with CI drift gates.
  - [ ] 26.1.3 Keep strict package metadata/ABI/import-pruning/trust gates green for expanded std surface.
  - [ ] 26.1.4 Add package-level execution slices so every `docs/std/README.md` package has an explicit Phase 26 owner task.
    - [ ] 26.1.4.1 `std::str`: lock first-production API cut + deterministic UTF-8 diagnostics/contracts, including stable `str_pattern::matches(pattern, input)`. (`docs/design/phase-26.1.4.1-std-str-first-production-lock.md`, `docs/std/coverage-matrix.md`) `DRI: std-str-owner`, `Target: 2026-06-05`.
    - [ ] 26.1.4.2 `std::bytes`: lock first-production API cut + constant-time compare contract/coverage. `DRI: std-bytes-owner`, `Target: 2026-06-10`.
    - [ ] 26.1.4.3 `std::int`: lock first-production API cut + checked/wrapping/saturating semantics coverage, plus checked `div/mod`, bitwise (`and/or/xor/not`), and checked shift/rotate contracts for `U64/U128/U256`. `DRI: std-int-owner`, `Target: 2026-06-13`.
    - [ ] 26.1.4.4 `std::codec`: lock first-production canonical encoding/decoding API + deterministic error surface, including decoder safety helpers (`position`, `remaining`, `read_fixed`). `DRI: std-codec-owner`, `Target: 2026-06-18`.
    - [ ] 26.1.4.5 `std::crypto`: lock first-production API cut + proof/assurance boundary policy for enabled intrinsics, including `verify_result::{is_valid,error_or_none,valid,invalid}` invariant enforcement and explicit negative tests rejecting mixed/invalid states. `DRI: std-crypto-owner`, `Target: 2026-06-23`.
    - [ ] 26.1.4.6 `std::host`: lock first-production capability surface + fail-closed host-profile conformance tests, including deterministic `Result<Bool, HostError>` semantics for `storage::set/delete` and `log::{info,warn,error}` (success must be `Ok(true)`). `DRI: std-host-owner`, `Target: 2026-06-25`.
    - [ ] 26.1.4.7 `std::contract`: lock first-production chain-agnostic contract-domain API cut. `DRI: std-contract-owner`, `Target: 2026-06-27`.
    - [ ] 26.1.4.8 `std::unit`: align first-production API cut with Gate D baseline (`assert_true`, `assert_eq_int`, `assert_eq_bool`, `fail`) plus deterministic failure mapping, contract tests for method-name stability, and fail-closed rejection of non-baseline assertion names in production profile. `DRI: std-unit-owner`, `Target: 2026-06-29`.
    - [ ] 26.1.4.9 `std::chain::<target>`: keep deferred by default; add target-specific activation checklist for launches that require it. `DRI: std-chain-owner`, `Target: 2026-07-01`.
    - [ ] 26.1.4.10 `std::dynamic`: keep deferred post-first-production; require explicit activation gates before any runtime-link implementation. `DRI: std-dynamic-owner`, `Target: 2026-07-02`.
    - [ ] 26.1.4.11 `std::list` (from collections catalog): lock deterministic out-of-range behavior for `insert/remove` and add non-terminating `insert_checked/remove_checked` conformance tests. `DRI: std-list-owner`, `Target: 2026-07-04`.
    - [ ] 26.1.4.12 `std::set` (from collections catalog): lock deterministic semantics for `contains/insert/remove` and conformance for mutation/non-mutation paths. `DRI: std-set-owner`, `Target: 2026-07-05`.
    - [ ] 26.1.4.13 `std::map` (from collections catalog): lock deterministic semantics for `contains/get/insert/remove` and take/mut variants with stable error behavior. `DRI: std-map-owner`, `Target: 2026-07-06`.

- [ ] 26.2 Finite-set proof roadmap (ordered execution) [Std Gate C]
  - [ ] 26.2.1 Add `std::set::subset(a, b) -> Bool` API with typing/lowering/runtime coverage and regression tests.
  - [ ] 26.2.2 Add finite-set VC/SMT reasoning for membership + subset and enforce strict no-assumption gate for theorem-grade claims over set properties.
  - [ ] 26.2.3 Sequence rule: complete subset proof support first (API + VC/SMT + tests) before expanding other set operators.
  - [ ] 26.2.4 Sequence rule: add proof support for `union`/`intersect`/`diff` after subset is complete and stable.
  - [ ] 26.2.5 Sequence rule: add cardinality-heavy proofs last (`len`, bounds, set-size relations) with solver performance guardrails.

- [ ] 26.3 List proof roadmap [Std Gate D]
  - [ ] 26.3.1 Define/lock list proof contracts for core APIs (`len`, `get`, `push`, `insert`, `remove`, `remove_take`, `pop`).
  - [ ] 26.3.2 Add VC/SMT reasoning for list index bounds and shape-preservation invariants.
  - [ ] 26.3.3 Add theorem-grade gates for list proofs (no assumption boundaries on release-enabled list surfaces).

- [ ] 26.4 Map proof roadmap [Std Gate E]
  - [ ] 26.4.1 Define/lock map proof contracts for core APIs (`len`, `contains`, `get`, `insert`, `insert_take`, `remove`, `remove_take`).
  - [ ] 26.4.2 Add VC/SMT reasoning for key-membership/value-consistency invariants.
  - [ ] 26.4.3 Add theorem-grade gates for map proofs (no assumption boundaries on release-enabled map surfaces).

- [ ] 26.5 Deferred test assertion extensions [Std Gate F]
  - [ ] 26.5.1 Add expected-failure and trap/error assertion semantics for `clg test` only after baseline `std::unit` assertions are stable.
  - [ ] 26.5.2 Add deterministic assertion-mismatch diff shape and deterministic failure-id taxonomy for advanced assertion paths.
  - [ ] 26.5.3 Evaluate generic `assert_eq<T>` only with explicit equality-capability constraints and deterministic diagnostics policy.
