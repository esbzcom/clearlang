# Phase 21.0 - Precompiled Std-Core Package Surface Lock

## Status
Design lock for Phase 21 execution (`21.0.1`-`21.0.8` in `docs/TODO.md`).

## Goal
Freeze the package split and callable function surface for precompiled std packaging:
- keep deterministic helpers in `std::core`,
- keep host capability calls in `std::host`,
- keep chain-specific constructors in chain packages.

## Design Principles Check
- Simple for users: source-level call names stay `std::<module>::<fn>`.
- AI-friendly: stable package/function mapping and deterministic import behavior.
- Provably correct: pure deterministic helpers are package-auditable and versioned.
- Crypto-focused: host capability boundaries remain explicit and fail-closed.

## Canonical Package Split
1. `std::core` (precompiled package)
   - Deterministic pure helpers and collection helpers.
2. `std::host` (host-backed package/interface)
   - Runtime capability calls only (`wasi`, `env`, `crypto`).
3. `std::<chain>` (chain packages)
   - Chain-specific type constructors and chain I/O wrappers.

## Locked Function Surface

### `std::core`
- `std::str::{len, eq, concat}`
- `std::bytes::{len, concat, eq, eq_ct, from_string, to_string}`
- `std::u64::{add_wrap, sub_wrap, mul_wrap, add_sat, sub_sat, mul_sat, rotl, rotr, to_bytes_le, to_bytes_be, from_bytes_le, from_bytes_be}`
- `std::u128::{from_limbs, lo, hi}`
- `std::u256::{from_limbs, limb0, limb1, limb2, limb3}`
- `std::array::{len}`
- `std::slice::{len, from_array, sub}`
- `std::list::{new, len, can_mut, get, push, insert, remove, remove_take, pop, push_mut, insert_mut, remove_mut, pop_mut}`
- `std::set::{new, len, can_mut, contains, insert, remove, insert_mut, remove_mut}`
- `std::map::{new, len, can_mut, contains, get, insert, insert_take, remove, remove_take, insert_mut, remove_mut}`

### `std::host`
- `std::wasi::{print}`
- `std::env::{time, random}`
- `std::crypto::{hash, hmac, verify}`

Notes for Phase 21 execution:
- `std::env::{time,random}` strict-mode policy must be locked explicitly (allow/deny matrix and diagnostics) before Phase 21 sign-off.
- `chain_id`, storage, and event capabilities require canonical ownership mapping (`std::host` vs chain wrappers) aligned with runtime host-import docs.

#### Host Capability Ownership v1 (`21.0.4.1`)

Canonical ownership target (v1):
- `time`, `random`: owned by `std::host` and surfaced as `std::env::{time,random}`.
- `chain_id`: host capability owned by `std::host` runtime boundary, surfaced to users via chain wrappers (`std::<chain>::...`) rather than a chain-agnostic core value helper.
- Storage (`storage_get/set/delete`): host capability owned by `std::host` runtime boundary, surfaced through chain wrappers for chain-specific policy/encoding.
- Events (`emit_event`): host capability owned by `std::host` runtime boundary, surfaced through chain wrappers.

Explicit non-goals for v1:
- No cross-chain unified storage/event API in `std::core`.
- No promotion of storage/event helpers to deterministic `pure` core helpers.
- No runtime-capability expansion beyond the existing host-import design targets in Phase 21.

#### Strict Determinism Policy for `std::env::{time,random}` (`21.0.4.2`)

Policy lock (Phase 21):
- Strict mode denies host-nondeterministic env capabilities `std::env::time` and `std::env::random`.
- Denial is deterministic and fail-closed with diagnostic `C106`.

Profile matrix (Phase 21 lock):

| Profile | `std::env::time` | `std::env::random` | Failure diagnostic |
|---|---|---|---|
| `contract_static` | deny | deny | `C106` |
| `shared_app` | deny | deny | `C106` |

Implementation lock for Phase 21:
- Host-profile v0 capability allowlist includes `std::env::{time,random}` for surface alignment with canonical `std::host`.
- Strict-gate profile policy still denies both capabilities in Phase 21, producing deterministic `C106`.

#### Precompiled Std-Core Activation Contract v1 (`21.0.7.0`)

This lock defines when precompiled std-core mode is active and what fallback behavior is permitted.

Build switch:
- `clg build --std-core-link-mode intrinsic`
- `clg build --std-core-link-mode precompiled`

Profile switch:
- strict host profile file (`clg.host-profile.json`) with `profile` set to `contract_static` or `shared_app`.

Activation rules (Phase 21 lock):
- `--std-core-link-mode precompiled` requires `--compiler-mode strict`.
- Non-strict build + `--std-core-link-mode precompiled` fails deterministically with `C035`.
- In strict mode, activation is evaluated against the selected host profile and strict preflight package inputs (`clg.lock.json`, `clg.package-metadata.json`, `clg.package-abi.json`).

Fallback policy lock:
- `intrinsic` mode: intrinsic lowering remains allowed for std-core-capable symbols.
- `precompiled` mode: fallback-to-intrinsic for the locked precompiled std-core symbol set is forbidden by policy and will be enforced as a fail-closed diagnostic gate in `21.0.7.2`.
- `21.0.7.1` must prove at least one canonical locked std-core symbol is linked via package ABI/import path under precompiled mode.

### Chain Packages (`std::<chain>`)
- `std::eth::{from_bytes, from_array}`
- `std::solana::{from_bytes, from_array}`
- `std::cosmos::{from_bytes, from_array}`

## Phase 21 Acceptance Rules
1. `21.0.1` artifact pipeline
   - CI produces versioned `std::core` artifact(s) and metadata.
   - Hash output is reproducible across identical inputs.
2. `21.0.2` import pruning
   - App Wasm import section includes only actually used package functions.
   - Unused functions from package metadata/ABI must not appear in emitted Wasm imports.
3. Surface drift gates (`21.0.3`)
   - Locked function surface, emitted std metadata, typer std call-check surface (builtins + specialized std call-check modules), and codegen std binding map (intrinsic vs package-import routing) stay in sync.
   - Drift checks consume a deterministic binding-map artifact rather than internal implementation details.
4. Host capability alignment (`21.0.4`)
   - `std::host` capability catalog and strict host-profile allowlist/checks are aligned and deterministic.
   - Host capability policy is published as a machine-readable per-profile artifact and covered by profile-conformance fixtures.
5. Artifact reproducibility evidence (`21.0.5`)
   - CI evidence gates prove reproducible std-core artifact bytes/digest under identical inputs.
6. Import-pruning evidence (`21.0.6`)
   - CI evidence gates prove used-only imports and unused-symbol exclusion behavior.
7. Non-vacuous precompiled-link proof (`21.0.7`)
   - Precompiled std-core activation contract (explicit `--std-core-link-mode` + host profile + fallback policy) is design-locked before enforcement.
   - CI proves at least one locked `std::core` symbol is linked through package ABI/import in precompiled mode (not only local intrinsic lowering).
   - Fallback-to-intrinsic under precompiled mode fails with deterministic diagnostics.
8. Strict fixture realism (`21.0.8`)
   - Strict acceptance fixtures use canonical locked std-core symbols (no synthetic placeholder namespace surface).
   - Migration must keep existing strict acceptance (`C101`-`C108`) semantics stable.

Boundary clarification:
- `21.0.1` and `21.0.2` are implementation tasks.
- `21.0.5` and `21.0.6` are CI evidence gates that validate those implementations.
- `21.0.7.0` defines activation semantics so `21.0.7.1/21.0.7.2` are objectively testable.

Current execution notes:
- `xtask std-core-artifact` emits a versioned std-core artifact bundle (`.wasm`, surface metadata, strict package metadata/ABI, manifest, digest).
- CI replays the generator twice and asserts byte-identical outputs before uploading the canonical artifact bundle.
- `xtask std-surface-drift-check` enforces a four-way drift gate:
  - locked function surface in this design doc,
  - emitted std metadata (`crates/cli/assets/std-metadata.json`),
  - typer std callable surface (`builtins.rs` + `check/expr/calls/collections.rs`),
  - codegen std binding routing map (`intrinsic` vs `package_import`).
- Canonical binding-route lock file: `docs/design/phase-21.0-std-binding-map.lock.json`.
- CI emits deterministic binding-map artifacts (`std-binding-map-v1.json` + `.sha256`) and replays generation twice to assert byte identity.
- `xtask host-capability-policy-artifact` emits deterministic host capability policy artifacts per profile (`contract_static`, `shared_app`) and validates against lock file `docs/design/phase-21.0-host-capability-policy.lock.json`.
- CI replays host capability policy artifact emission twice and asserts byte-identical outputs (`host-capability-policy-v1.json` + `.sha256`).
- CI/profile-conformance fixtures cover strict behavior for `std::env::time`, `std::env::random`, and `std::env::chain_id`:
  - `time`/`random` rejected with deterministic `C106` under strict mode,
  - `chain_id` accepted when required capability is present in host profile.
- `21.0.7.1` non-vacuous proof fixture is covered by `strict_acceptance_precompiled_std_core_symbol_links_via_package_import` in `crates/cli/tests/cli_it/diagnostics.rs`:
  - strict precompiled mode links canonical locked symbol `std::str::len` via package ABI/import,
  - emitted Wasm code is asserted to call the imported function index (not intrinsic-only lowering).
- `crates/cli/assets/std-metadata.json` is aligned with the locked collection take APIs:
  - `std::list::remove_take`
  - `std::map::insert_take`
  - `std::map::remove_take`
- CLI import integration coverage pins this metadata surface via:
  - `import_std_list_remove_take_item_works`
  - `import_std_map_take_items_work`
- Import pruning is covered in `crates/cli/tests/cli_it/imports.rs` with Wasm import-section assertions:
  - `build_with_compiled_package_import_succeeds` verifies used package import emission.
  - `build_with_compiled_package_import_prunes_unused_exports` verifies unused metadata exports are not emitted.
- CI runs `cargo test -p clg-cli --test cli_it imports::` as an explicit import-pruning acceptance gate.

## Non-Goals (Phase 21)
- No transitive resolver or semver solver changes (Phase 22).
- No runtime auto-loader behavior changes (Phase 23).
- No chain-specific policy expansion beyond constructor surface lock.

## References
- `docs/TODO.md`
- `docs/design/phase-21.0-std-binding-map.lock.json`
- `docs/design/phase-21.0-host-capability-policy.lock.json`
- `docs/design/phase-20.0-std-packaging-runtime-linking.md`
- `docs/design/phase-18.4-compiled-package-imports.md`
- `docs/runtime/host-imports.md`
- `docs/runtime/chain-packages.md`
